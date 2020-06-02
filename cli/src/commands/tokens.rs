use crate::console::Terminal;
use serde_json::json;
use structopt::StructOpt;
use tari_validator_node::{
    config::NodeConfig,
    db::{
        models::{asset_states::*, tokens::*},
        utils::db::db_client,
    },
    types::{AssetID, TokenID},
};

#[derive(StructOpt, Debug)]
pub enum TokenCommands {
    List {
        asset_id: AssetID,
    },
    /// View token details
    View {
        token_id: TokenID,
    },
}

impl TokenCommands {
    pub async fn run(self, node_config: NodeConfig) -> anyhow::Result<()> {
        let client = db_client(&node_config).await?;
        match self {
            Self::List { asset_id } => {
                let asset = AssetState::find_by_asset_id(&asset_id, &client).await?;
                match asset {
                    Some(asset) => {
                        let tokens = Token::find_by_asset_state_id(asset.id, &client).await?;
                        if tokens.len() == 0 {
                            println!("No tokens exist for Asset ID");
                        } else {
                            let mut output = vec![];
                            for token in tokens.into_iter() {
                                output.push(json!({
                                    "Id": token.token_id,
                                    "IssueNumber": token.issue_number,
                                    "Status": token.status
                                }))
                            }

                            Terminal::basic().render_list(
                                format!("Tokens of asset ID {}", asset_id.to_string()).as_str(),
                                output,
                                &["Id", "IssueNumber", "Status"],
                                &[96, 20, 20],
                            );
                        }
                    },
                    None => {
                        println!("Asset ID not found!");
                    },
                }
            },
            Self::View { token_id } => {
                let token: Option<DisplayToken> = Token::find_by_token_id(&token_id, &client).await?.map(|t| t.into());
                if token.is_some() {
                    Terminal::basic().render_object("Token details:", token);
                } else {
                    println!("Token not found!");
                }
            },
        };
        Ok(())
    }
}
