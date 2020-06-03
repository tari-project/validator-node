use super::MakeItRain;
use crate::console::Terminal;
use deadpool_postgres::Client;
use serde_json::json;
use structopt::StructOpt;
use tari_validator_node::{
    config::NodeConfig,
    db::{
        models::{asset_states::*, digital_assets::*},
        utils::db::db_client,
    },
    types::{AssetID, Pubkey, RaidID, TemplateID},
};

#[derive(StructOpt, Debug)]
pub enum AssetCommands {
    /// Create new asset
    Create(CreateAsset),
    /// List assets of template
    List {
        /// TemplateID in the form {type}.{version}
        template: TemplateID,
    },
    /// View asset details
    View {
        /// Work with tokens of asset
        asset_id: AssetID,
    },
    /// Runs load scenario on a Single Use Token asset:
    ///
    /// 1. Issue `tokens` amount of tokens
    /// 2. Start `concurrency` parallel users
    /// 3. For every user create unique pubkey and get chunk of tokens
    /// 4. issue sell_token
    /// 5. Once instruction goes to Commit - send redeem_token
    /// 6. Repeat step 4 with next token from chunk
    MakeItRain(MakeItRain),
}

#[derive(StructOpt, Debug)]
pub struct CreateAsset {
    /// TemplateID in the form {type}.{version}
    pub template: TemplateID,
    /// Name of the asset
    #[structopt(empty_values = false)]
    pub name: String,
    /// Description
    #[structopt(short = "d", long, default_value)]
    pub description: String,
    /// Fully qualified domain name
    #[structopt(short = "f", long)]
    pub fqdn: Option<String>,
    /// RaidID
    #[structopt(short = "r", long)]
    pub raid_id: Option<String>,
    /// Pubkey of issuer
    #[structopt(short = "p", long)]
    pub issuer: Pubkey,
    /// Additional data as a JSON in a string
    #[structopt(long)]
    pub data: Option<String>,
}

impl AssetCommands {
    pub async fn run(self, node_config: NodeConfig) -> anyhow::Result<()> {
        let client = db_client(&node_config).await?;
        match self {
            Self::Create(create) => {
                let asset = create.run(&client).await?;
                Terminal::basic().render_object("Asset created! Details:", asset);
            },
            Self::List { template } => {
                let assets = AssetState::find_by_template_id(&template, &client).await?;
                let mut output = vec![];
                for asset in assets.into_iter() {
                    let da = DigitalAsset::load(asset.digital_asset_id, &client).await?;
                    output.push(json!({
                        "Id": asset.asset_id,
                        "Name": asset.name,
                        "Status": asset.status,
                        "FQDN": da.fqdn,
                        "Description": asset.description
                    }))
                }
                Terminal::basic().render_list(
                    format!("Assets of template {}", template).as_str(),
                    output,
                    &["Id", "Name", "Status", "FQDN", "Description"],
                    &[64, 20, 8, 15, 40],
                );
            },
            Self::View { asset_id } => {
                let asset = AssetState::find_by_asset_id(&asset_id, &client).await?;
                if asset.is_some() {
                    Terminal::basic().render_object("Asset details:", asset);
                } else {
                    println!("Asset not found!");
                }
            },
            Self::MakeItRain(mir) => {
                mir.run(node_config).await?;
            },
        };
        Ok(())
    }
}

impl CreateAsset {
    async fn run(self, client: &Client) -> anyhow::Result<AssetState> {
        let da_id = DigitalAsset::insert(
            NewDigitalAsset {
                template_type: self.template.template_type(),
                fqdn: self.fqdn.clone(),
                raid_id: self.raid_id.clone(),
                ..Default::default()
            },
            &client,
        )
        .await?;
        let raid_id: RaidID = self
            .raid_id
            .map(|rid| rid.parse().unwrap())
            .unwrap_or(RaidID::default());
        // TODO: this is a stub:
        let hash = AssetID::generate_hash(format!(
            "{}{}{:?}{:?}{:?}",
            self.name, self.description, self.fqdn, raid_id, self.data
        ));
        let id = AssetState::insert(
            NewAssetState {
                name: self.name,
                description: self.description,
                asset_id: AssetID::new(self.template, 0, raid_id, hash),
                asset_issuer_pub_key: self.issuer,
                digital_asset_id: da_id,
                initial_data_json: self
                    .data
                    .map(|data| serde_json::from_str(&data).unwrap())
                    .unwrap_or(json!({})),
                ..Default::default()
            },
            &client,
        )
        .await?;
        Ok(AssetState::load(id, &client).await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::build_test_config;
    use tari_test_utils::random::string;

    #[actix_rt::test]
    async fn test_asset_create() {
        let config = build_test_config().unwrap();
        let client = db_client(&config).await.unwrap();
        let asset = CreateAsset {
            template: 1.into(),
            name: "may rocket launch".into(),
            description: "".into(),
            fqdn: Some("disney.com".into()),
            raid_id: None,
            issuer: "user_pub_key".into(),
            data: Some(format!(r#"{{ "custom": "{}" }}"#, string(8))),
        }
        .run(&client)
        .await
        .unwrap();
        assert_eq!(asset.name, "may rocket launch".to_string());
    }
}
