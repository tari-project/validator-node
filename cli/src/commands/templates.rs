use crate::console::Terminal;
use serde_json::json;
use structopt::StructOpt;
use tari_validator_node::{
    config::NodeConfig,
    db::{models::DigitalAsset, utils::db::db_client},
    template::{single_use_tokens::SingleUseTokenTemplate, Template},
};

#[derive(StructOpt, Debug)]
pub enum TemplateCommands {
    /// List templates
    List,
}

impl TemplateCommands {
    pub async fn run(self, node_config: NodeConfig) -> anyhow::Result<()> {
        let client = db_client(&node_config).await?;
        match self {
            TemplateCommands::List => {
                // TODO: templates are hardcoded for now, at later stage should come from config
                let mut templates = vec![];
                for (id, name) in &[(SingleUseTokenTemplate::id(), "Single Use Tokens")] {
                    let assets_len = DigitalAsset::find_by_template_id(&id, &client).await?.len();
                    templates.push(json!({
                        "Id": id.to_string(),
                        "Name": name,
                        "Assets": assets_len
                    }));
                }
                Terminal::basic().render_list("Available Templates", templates, &["Id", "Name", "Assets"], &[
                    10, 50, 10,
                ]);
            },
        };
        Ok(())
    }
}
