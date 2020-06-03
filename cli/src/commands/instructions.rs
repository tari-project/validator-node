use crate::console::Terminal;
use awc::Client as WebClient;
use serde_json::Value;
use std::time::Duration;
use structopt::StructOpt;
use tari_validator_node::{
    config::NodeConfig,
    db::models::consensus::instructions::*,
    template::{asset_call_path, token_call_path},
    types::{AssetID, InstructionID, TokenID},
};
use tokio::time::delay_for;
use tokio_postgres::Client;

const WAIT: Duration = Duration::from_millis(1000);
const MAX_RETRIES: usize = 60;

#[derive(StructOpt, Debug)]
pub enum InstructionCommands {
    Asset {
        asset_id: AssetID,
        contract_name: String,
        data: Value,
        /// Do not show progress, do not output result
        #[structopt(long)]
        silent: bool,
        /// Wait for Commit (by default is waiting for Pending)
        #[structopt(long)]
        wait_commit: bool,
    },
    Token {
        token_id: TokenID,
        contract_name: String,
        data: Value,
        /// Do not show progress, do not output result
        #[structopt(long)]
        silent: bool,
        /// Wait for Commit (by default is waiting for Pending)
        #[structopt(long)]
        wait_commit: bool,
    },
    // Status of instruction and all subinstructions
    Status {
        instruction_id: InstructionID,
    },
    // View details of instruction
    View {
        instruction_id: InstructionID,
    },
}

impl InstructionCommands {
    pub async fn run(self, node_config: NodeConfig, client: &Client) -> anyhow::Result<Instruction> {
        match self {
            Self::Asset {
                asset_id,
                contract_name,
                data,
                silent,
                wait_commit,
            } => {
                let url = asset_call_path(&asset_id, contract_name.as_str());
                let url = format!("http://localhost:{}{}", node_config.actix.port, url);
                Self::call(url, data, silent, wait_commit, client).await
            },
            Self::Token {
                token_id,
                contract_name,
                data,
                silent,
                wait_commit,
            } => {
                let url = token_call_path(&token_id, contract_name.as_str());
                let url = format!("http://localhost:{}{}", node_config.actix.port, url);
                Self::call(url, data, silent, wait_commit, client).await
            },
            Self::Status { instruction_id } => {
                let instruction = Instruction::load(instruction_id, &client).await?;
                Self::display_instruction_status(&instruction, client).await?;
                Ok(instruction)
            },
            Self::View { instruction_id } => {
                let instruction = Instruction::load(instruction_id, client).await?;
                Terminal::basic().render_object("Instruction details", instruction.clone());
                Ok(instruction)
            },
        }
    }

    pub async fn call(
        url: String,
        data: Value,
        silent: bool,
        wait_commit: bool,
        client: &Client,
    ) -> anyhow::Result<Instruction>
    {
        let web = WebClient::default();
        let mut resp = web.post(&url).send_json(&data).await.unwrap();
        if resp.status().is_success() {
            let instruction: Instruction = match resp.json::<Value>().await {
                Ok(val) => {
                    if let Some(err) = val.as_object().expect("Expected object in response").get("error") {
                        return Err(anyhow::anyhow!("POST {} failed: {}", url, err));
                    } else {
                        serde_json::from_value(val)?
                    }
                },
                Err(err) => {
                    return Err(anyhow::anyhow!("POST {} failed: {}", url, err));
                },
            };

            if wait_commit {
                Ok(Self::wait_status(&instruction, InstructionStatus::Commit, client, silent, WAIT).await?)
            } else {
                Ok(Instruction::load(instruction.id, client).await?)
            }
        } else {
            Err(anyhow::anyhow!("Request Failed: {:?}", resp.body().await))
        }
    }

    pub async fn wait_status(
        instruction: &Instruction,
        status: InstructionStatus,
        client: &Client,
        silent: bool,
        refresh_interval: Duration,
    ) -> anyhow::Result<Instruction>
    {
        let mut retries = 0;
        loop {
            let instruction = Instruction::load(instruction.id, &client).await?;
            if !silent {
                Self::display_instruction_status(&instruction, &client).await?;
            }
            if instruction.status == status || instruction.status == InstructionStatus::Commit {
                return Ok(instruction);
            } else if instruction.status == InstructionStatus::Invalid {
                return Err(anyhow::anyhow!(
                    "Instruction {} Invalid {}",
                    instruction.id,
                    instruction.result
                ));
            }
            delay_for(refresh_interval).await;
            retries += 1;
            if retries > MAX_RETRIES {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for instruction {} Commit",
                    instruction.id
                ));
            }
        }
    }

    pub async fn display_instruction_status(instruction: &Instruction, client: &Client) -> anyhow::Result<()> {
        let subinstructions = instruction.load_subinstructions(&client).await?;
        let mut instructions = vec![instruction_view(instruction, true)];
        instructions.extend(subinstructions.iter().map(|i| instruction_view(i, false)));
        Terminal::basic().render_list("Instruction details", instructions, COLUMNS, SIZES);
        Ok(())
    }
}

const COLUMNS: &[&str] = &["Root", "Id", "Status", "Params", "Result"];
const SIZES: &[u16] = &[4, 36, 10, 100];

fn instruction_view(instruction: &Instruction, root: bool) -> serde_json::Value {
    serde_json::json!({
        "Root": if root { " ** " } else { "" },
        "Id": instruction.id,
        "Status": instruction.status,
        "Params": instruction.params,
    })
}
