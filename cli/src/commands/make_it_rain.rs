use super::InstructionCommands;
use crate::console::Terminal;
use deadpool::managed::PoolConfig;
use deadpool_postgres::{Client, Pool};
use serde_json::{json, Value};
use std::{collections::HashMap, ops::AddAssign, time::Duration};
use structopt::StructOpt;
use tari_validator_node::{
    config::NodeConfig,
    db::{
        models::{consensus::instructions::*, wallet::*},
        utils::db::build_pool,
    },
    template::single_use_tokens::{SellTokenLockParams, TokenContracts},
    types::{AssetID, Pubkey, TokenID},
};
use tokio::{sync::Mutex, time::delay_for};

const MAX_RETRIES: usize = 60;

lazy_static::lazy_static! {
    static ref TERMINAL: Mutex<Terminal> = Mutex::new(Terminal::basic());
    static ref COUNTERS: Mutex<HashMap<String, Counters>> = Mutex::new(HashMap::new());
}

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
    concurrency: u16,
    /// How many tokens to issue for test (total requests)
    #[structopt(short = "t", long, default_value = "100")]
    tokens: u16,
    /// Timeout for sell_token instruction
    #[structopt(long, default_value = "30")]
    timeout: u64,
}

impl MakeItRain {
    pub async fn run(self, mut node_config: NodeConfig) -> anyhow::Result<()> {
        node_config.postgres.pool = Some(PoolConfig {
            max_size: self.concurrency as usize,
            ..Default::default()
        });
        let pool = build_pool(&node_config.postgres)?;
        // split by concurrent streams
        let user_futures = (0..self.concurrency).into_iter().map(|i| {
            let key = format!("user {}", i);
            self.clone().user_scenario(key, &node_config, &pool)
        });
        // run user emulations in parallel
        let results = futures::future::join_all(user_futures).await;
        delay_for(Duration::from_millis(1000)).await;
        println!("Errors (if any):");
        for (i, result) in results.iter().enumerate() {
            if let Err(err) = result {
                println!("{}. {}", i, err)
            }
        }
        Ok(())
    }

    /// Running scenario:
    /// 2. For every user create unique pubkey
    /// 2. issue tokens
    /// 4. pick next created token, issue sell_token
    /// 5. If user random goes over the fake threshold - send money to sell_token wallet
    /// 6. Once instruction goes to Commit - send redeem_token
    /// 7. repeat for other tokens
    async fn user_scenario(self, key: Pubkey, node_config: &NodeConfig, pool: &Pool) -> anyhow::Result<()> {
        let client = pool.get().await?;
        let mut counters = Counters::new(&key);
        let quantity = self.tokens / self.concurrency;
        // issue tokens
        let token_ids = match self.issue_tokens(quantity, node_config, &client).await {
            Ok(token_ids) => token_ids,
            Err(err) => {
                counters.failed += 1;
                println!("User {} failed to issue tokens: {}", key, err);
                return Err(err);
            },
        };

        // run scenario for every token one by one
        for token_id in token_ids.into_iter() {
            match self.process_token(&key, &token_id, &node_config, &client).await {
                Ok((sell_duration, redeem_duration)) => {
                    counters.success(sell_duration, redeem_duration);
                },
                Err(err) => {
                    counters.failed();
                    println!("User {} failed to process token {}: {}", key, token_id, err);
                    return Err(err);
                },
            };
        }
        Ok(())
    }

    async fn issue_tokens(
        &self,
        quantity: u16,
        node_config: &NodeConfig,
        client: &Client,
    ) -> anyhow::Result<Vec<TokenID>>
    {
        let instruction = InstructionCommands::Asset {
            asset_id: self.asset_id.clone(),
            contract_name: "issue_tokens".into(),
            data: json!({ "quantity": quantity }),
            silent: true,
            wait_commit: true,
        }
        .run(node_config.clone(), client)
        .await?;
        Ok(serde_json::from_value(instruction.result)?)
    }

    async fn process_token(
        &self,
        key: &String,
        token_id: &TokenID,
        node_config: &NodeConfig,
        client: &Client,
    ) -> anyhow::Result<(Duration, Duration)>
    {
        let refresh = Duration::from_millis(200 * self.concurrency as u64);
        let time = std::time::Instant::now();
        let instruction = self.sell_token(&key, &token_id, &node_config, &client).await?;
        Self::fill_wallet(&instruction, &client, refresh.clone()).await?;
        InstructionCommands::wait_status(&instruction, InstructionStatus::Pending, &client, true, refresh.clone())
            .await?;
        let sell_timings = time.elapsed();
        let time = std::time::Instant::now();
        let instruction = Self::redeem_token(&token_id, &node_config, &client).await?;
        InstructionCommands::wait_status(&instruction, InstructionStatus::Pending, &client, true, refresh.clone())
            .await?;
        let redeem_timings = time.elapsed();
        Ok((sell_timings, redeem_timings))
    }

    async fn sell_token(
        &self,
        key: &String,
        token_id: &TokenID,
        node_config: &NodeConfig,
        client: &Client,
    ) -> anyhow::Result<Instruction>
    {
        InstructionCommands::Token {
            token_id: token_id.clone(),
            contract_name: "sell_token".into(),
            data: json!({"price": 1, "timeout_secs": self.timeout, "user_pubkey": key}),
            silent: true,
            wait_commit: false,
        }
        .run(node_config.clone(), client)
        .await
    }

    async fn fill_wallet(instruction: &Instruction, client: &Client, refresh_interval: Duration) -> anyhow::Result<()> {
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
                    panic!("Expected SellTokenLock subinstruction");
                }
            }
            delay_for(refresh_interval).await;
            retries += 1;
            if retries > MAX_RETRIES {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for subinstruction of {}",
                    instruction.id
                ));
            }
        };
        let wallet = Wallet::select_by_key(&wallet_key, &client).await?;
        wallet.set_balance(1, &client).await?;
        Ok(())
    }

    async fn redeem_token(
        token_id: &TokenID,
        node_config: &NodeConfig,
        client: &Client,
    ) -> anyhow::Result<Instruction>
    {
        InstructionCommands::Token {
            token_id: token_id.clone(),
            contract_name: "redeem_token".into(),
            data: Value::Null,
            silent: true,
            wait_commit: false,
        }
        .run(node_config.clone(), client)
        .await
    }
}

#[derive(Clone, Default, Debug)]
struct Counters {
    name: String,
    success: u64,
    failed: u64,
    sell_avg: Option<u64>,
    redeem_avg: Option<u64>,
}

impl Counters {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn success(&mut self, sell: Duration, redeem: Duration) {
        *self += Counters {
            name: "".into(),
            success: 1,
            failed: 0,
            sell_avg: Some(sell.as_millis() as u64),
            redeem_avg: Some(redeem.as_millis() as u64),
        };
        actix_rt::spawn(Self::update_display(self.clone()));
    }

    fn failed(&mut self) {
        self.failed += 1;
        actix_rt::spawn(Self::update_display(self.clone()));
    }
}

impl AddAssign for Counters {
    fn add_assign(&mut self, rhs: Self) {
        let total_success = self.success + rhs.success;
        if self.sell_avg.is_some() {
            let total_ms = self.sell_avg.unwrap() * self.success + rhs.sell_avg.unwrap() * rhs.success;
            self.sell_avg = Some(total_ms / total_success);
        } else {
            self.sell_avg = rhs.sell_avg;
        }
        if self.redeem_avg.is_some() {
            let total_ms = self.redeem_avg.unwrap() * self.success + rhs.redeem_avg.unwrap() * rhs.success;
            self.redeem_avg = Some(total_ms / total_success);
        } else {
            self.redeem_avg = rhs.redeem_avg;
        }
        self.success = total_success;
    }
}

impl Counters {
    const FIELDS: &'static [&'static str] = &["User", "Success", "Failed", "Avg sell ms", "Avg redeem ms"];
    const SIZES: &'static [u16] = &[10, 10, 10, 14, 14];

    async fn update_display(record: Counters) {
        let mut counters = COUNTERS.lock().await;
        counters.insert(record.name.clone(), record);
        let mut total = Counters::new("Total");
        let mut counters: Vec<Value> = counters
            .values()
            .map(|next| {
                total += next.clone();
                next.to_display()
            })
            .collect();
        counters.insert(0, total.to_display());
        TERMINAL
            .lock()
            .await
            .render_list("Make it rain: stats by threads", counters, Self::FIELDS, Self::SIZES);
    }

    fn to_display(&self) -> Value {
        json!({
            "User": self.name,
            "Success": self.success,
            "Failed": self.failed,
            "Avg sell ms": self.sell_avg,
            "Avg redeem ms": self.redeem_avg,
        })
    }
}
