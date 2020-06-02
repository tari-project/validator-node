use crate::ui::{render_object_as_table, render_value_as_table};
use serde_json::Value;
use structopt::StructOpt;
use tari_validator_node::{
    config::NodeConfig,
    db::{
        models::{asset_states::*, consensus::instructions::*},
        utils::db::db_client,
    },
    types::{AssetID, TokenID},
};

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
}

impl InstructionCommands {
    pub async fn run(self, node_config: NodeConfig) -> anyhow::Result<()> {
        let client = db_client(&node_config).await?;
        match self {
            Self::Asset {
                asset_id,
                contract_name,
                data,
            } => {},
            Self::Token {
                token_id,
                contract_name,
                data,
            } => {},
        };
        Ok(())
    }
}
