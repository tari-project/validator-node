use crate::console::Terminal;
use awc::Client as WebClient;
use deadpool_postgres::Client;
use serde_json::Value;
use std::time::Duration;
use structopt::StructOpt;
use tari_validator_node::{
    config::NodeConfig,
    db::{models::consensus::instructions::*, utils::db::db_client},
    template::{asset_call_path, token_call_path},
    types::{AssetID, InstructionID, TokenID},
};
use tokio::time::delay_for;

const WAIT: Duration = Duration::from_secs(1);

#[derive(StructOpt, Debug)]
pub enum InstructionCommands {
    Asset {
        asset_id: AssetID,
        contract_name: String,
        data: Value,
    },
    Token {
        token_id: TokenID,
        contract_name: String,
        data: Value,
    },
    // Status of instruction and all subinstructions
    Status {
        instruction_id: InstructionID,
    },
    View {
        instruction_id: InstructionID,
    },
}

impl InstructionCommands {
    pub async fn run(self, node_config: NodeConfig) -> anyhow::Result<()> {
        let client = db_client(&node_config).await?;
        match self {
            Self::Asset {
                asset_id,
                contract_name,
                data,
            } => {
                let web = WebClient::default();
                let url = asset_call_path(&asset_id, contract_name.as_str());
                let url = format!("http://localhost:{}{}", node_config.actix.port, url);
                let mut resp = web.post(url).send_json(&data).await.unwrap();
                if resp.status().is_success() {
                    let instruction: Instruction = resp.json().await.unwrap();
                    delay_for(WAIT).await;
                    display_instruction_status(instruction.id, &client).await?;
                } else {
                    println!("Request Failed: {:?}", resp.body().await);
                }
            },
            Self::Token {
                token_id,
                contract_name,
                data,
            } => {
                let web = WebClient::default();
                let url = token_call_path(&token_id, contract_name.as_str());
                let url = format!("http://localhost:{}{}", node_config.actix.port, url);
                let mut resp = web.post(url).send_json(&data).await.unwrap();
                if resp.status().is_success() {
                    let instruction: Instruction = resp.json().await.unwrap();
                    delay_for(WAIT).await;
                    display_instruction_status(instruction.id, &client).await?;
                } else {
                    println!("Request Failed: {:?}", resp.body().await);
                }
            },
            Self::Status { instruction_id } => {
                display_instruction_status(instruction_id, &client).await?;
            },
            Self::View { instruction_id } => {
                let instruction = Instruction::load(instruction_id, &client).await?;
                Terminal::basic().render_object("Instruction details", instruction);
            },
        };
        Ok(())
    }
}

async fn display_instruction_status(instruction_id: InstructionID, client: &Client) -> anyhow::Result<()> {
    let instruction = Instruction::load(instruction_id, &client).await?;
    let subinstructions = instruction.load_subinstructions(&client).await?;
    let mut instructions = vec![instruction_view(&instruction, true)];
    instructions.extend(subinstructions.iter().map(|i| instruction_view(i, false)));
    Terminal::basic().render_list("Instruction details", instructions, COLUMNS, SIZES);
    Ok(())
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
