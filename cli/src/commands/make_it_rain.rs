use super::InstructionCommands;
use crate::console::Terminal;
use deadpool_postgres::{Client, Pool};
use serde_json::{json, Value};
use std::time::Duration;
use structopt::StructOpt;
use tari_validator_node::{
    config::NodeConfig,
    db::{
        models::{asset_states::*, consensus::instructions::*, tokens::*, wallet::*, TokenStatus},
        utils::db::build_pool,
    },
    template::single_use_tokens::{SellTokenLockParams, TokenContracts},
    types::{AssetID, Pubkey},
};
use tokio::time::delay_for;

const WAIT: Duration = Duration::from_millis(100);
const MAX_RETRIES: usize = 600;

#[derive(StructOpt, Debug, Clone)]
/// Runs load scenario on a Single Use Token asset:
///
/// 1. Issue `tokens` quantity of tokens
/// 2. Start `concurrency` parallel users
/// 3. For every user create unique pubkey and get chunk of tokens
/// 4. issue sell_token
/// 5. Once instruction goes to Commit - send redeem_token
/// 6. Repeat step 4 with next token from chunk
pub struct MakeItRain {
    /// Target asset in the Single Use Token template
    asset_id: AssetID,
    ///// Timeout in seconds for sell_token contract
    //#[structopt(short="s", long, default_value="10")]
    // timeout: u16,
    /// How many parallel threads to run
    #[structopt(short = "c", long, default_value = "4")]
    concurrecy: u16,
    /// How many tokens to issue for test (total requests)
    #[structopt(short = "t", long, default_value = "100")]
    tokens: u16,
}

impl MakeItRain {
    const FIELDS: &'static [&'static str] = &["User", "Success", "Failed", "Error", "Avg sell ms", "Avg redeem ms"];
    const SIZES: &'static [u16] = &[10, 10, 10, 10, 10, 10];

    pub async fn run(self, node_config: NodeConfig) -> anyhow::Result<()> {
        let pool = build_pool(&node_config.postgres)?;
        // Create and retrieve available tokens:
        let instruction = InstructionCommands::Asset {
            asset_id: self.asset_id.clone(),
            contract_name: "issue_tokens".into(),
            data: json!({"quantity": self.tokens}),
        }
        .run(node_config.clone())
        .await?
        .expect("Failed to retrieve instruction for issue_tokens");
        let client = pool.get().await?;
        Self::wait_status(&instruction, InstructionStatus::Commit, &client).await?;

        // retrieve available tokens
        let asset = AssetState::find_by_asset_id(&self.asset_id, &client).await?.unwrap();
        let tokens = Token::find_by_asset_state_id(asset.id.clone(), &client).await?;
        drop(client);
        let mut available_tokens: Vec<_> = tokens
            .into_iter()
            .filter(|token| token.status == TokenStatus::Available)
            .collect();
        available_tokens.truncate(self.tokens as usize);
        // split by concurrent streams
        let chunks = available_tokens.chunks((self.tokens / self.concurrecy) as usize);
        let scenarios_futures = chunks
            .map(|tokens| tokens.iter().cloned().collect())
            .enumerate()
            .into_iter()
            .map(|(i, tokens)| {
                let key = format!("user {}", i);
                self.clone()
                    .user_scenario(key, tokens, node_config.clone(), pool.clone())
            });
        // run user emulations in parallel
        let results = futures::future::join_all(scenarios_futures).await;
        Terminal::basic().render_list("Make it rain stats by threads", results, Self::FIELDS, Self::SIZES);
        Ok(())
    }

    /// Running scenario:
    /// 3. For every user create unique pubkey and get chunk of tokens
    /// 4. issue sell_token
    /// 5. If user random goes over the fake threshold - send money to sell_token wallet
    /// 6. Once instruction goes to Commit - send redeem_token
    /// 7. repeat for other tokens
    async fn user_scenario(self, key: Pubkey, tokens: Vec<Token>, node_config: NodeConfig, pool: Pool) -> Value {
        let mut counters = Counters::default();
        for token in tokens.into_iter() {
            match Self::process_token(&key, &token, &node_config, &pool).await {
                Ok(Some((sell_duration, redeem_duration))) => {
                    counters.sell_timings.push(sell_duration);
                    counters.redeem_timings.push(redeem_duration);
                    counters.success += 1;
                },
                Ok(None) => {
                    counters.instruction_failed += 1;
                },
                Err(err) => {
                    counters.error += 1;
                    println!("User {} failed to process token {}: {}", key, token.token_id, err);
                },
            }
        }
        json!({
            "User": key,
            "Success": counters.success,
            "Failed": counters.instruction_failed,
            "Error": counters.error,
            "Avg sell ms": counters.avg_sell(),
            "Avg redeem ms": counters.avg_redeem(),
        })
    }

    async fn process_token(
        key: &String,
        token: &Token,
        node_config: &NodeConfig,
        pool: &Pool,
    ) -> anyhow::Result<Option<(Duration, Duration)>>
    {
        let time = std::time::Instant::now();
        let sell_token = Self::sell_token(&key, &token, &node_config).await?;
        if let Some(instruction) = sell_token {
            let client = pool.get().await?;
            Self::fill_wallet(&instruction, &client).await?;
            Self::wait_status(&instruction, InstructionStatus::Pending, &client).await?;
            drop(client);
            let sell_timings = time.elapsed();
            let time = std::time::Instant::now();
            let redeem_token = Self::redeem_token(&token, &node_config).await?;
            if let Some(instruction) = redeem_token {
                let client = pool.get().await?;
                Self::wait_status(&instruction, InstructionStatus::Pending, &client).await?;
                let redeem_timings = time.elapsed();
                Ok(Some((sell_timings, redeem_timings)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn sell_token(key: &String, token: &Token, node_config: &NodeConfig) -> anyhow::Result<Option<Instruction>> {
        InstructionCommands::Token {
            token_id: token.token_id.clone(),
            contract_name: "sell_token".into(),
            data: json!({"price": 1, "timeout_secs": 10, "user_pubkey": key}),
        }
        .run(node_config.clone())
        .await
    }

    async fn fill_wallet(instruction: &Instruction, client: &Client) -> anyhow::Result<()> {
        let mut retries = 0;
        let wallet_key = loop {
            let subinstructions = instruction
                .load_subinstructions(&client)
                .await
                .expect("Failed to load subinstructions");
            if subinstructions.len() > 0 {
                let contract: TokenContracts = serde_json::from_value(subinstructions[0].params.clone()).unwrap();
                if let TokenContracts::SellTokenLock(SellTokenLockParams { wallet_key }) = contract {
                    break wallet_key;
                } else {
                    panic!("Expected SellTokenLock contract");
                }
            }
            delay_for(WAIT).await;
            retries += 1;
            if retries > MAX_RETRIES {
                return Err(anyhow::anyhow!("Timeout waiting for subinstruction"));
            }
        };
        let wallet = Wallet::select_by_key(&wallet_key, &client).await?;
        wallet.set_balance(1, &client).await?;
        Ok(())
    }

    async fn redeem_token(token: &Token, node_config: &NodeConfig) -> anyhow::Result<Option<Instruction>> {
        InstructionCommands::Token {
            token_id: token.token_id.clone(),
            contract_name: "redeem_token".into(),
            data: Value::Null,
        }
        .run(node_config.clone())
        .await
    }

    async fn wait_status(instruction: &Instruction, status: InstructionStatus, client: &Client) -> anyhow::Result<()> {
        let mut retries = 0;
        loop {
            let instruction = Instruction::load(instruction.id, &client).await?;
            if instruction.status == status ||
                instruction.status == InstructionStatus::Commit ||
                instruction.status == InstructionStatus::Invalid
            {
                break;
            }
            delay_for(WAIT).await;
            retries += 1;
            if retries > MAX_RETRIES {
                return Err(anyhow::anyhow!("Timeout waiting for instruction commit"));
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct Counters {
    success: usize,
    instruction_failed: usize,
    error: usize,
    sell_timings: Vec<Duration>,
    redeem_timings: Vec<Duration>,
}

impl Counters {
    fn avg_sell(&self) -> Option<u64> {
        Self::avg_duration(&self.sell_timings)
    }

    fn avg_redeem(&self) -> Option<u64> {
        Self::avg_duration(&self.redeem_timings)
    }

    fn avg_duration(measurements: &Vec<Duration>) -> Option<u64> {
        if measurements.len() > 0 {
            Some(measurements.iter().map(|d| d.as_millis() as u64).sum::<u64>() / measurements.len() as u64)
        } else {
            None
        }
    }
}
