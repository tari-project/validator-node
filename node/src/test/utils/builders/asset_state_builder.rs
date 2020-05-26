use super::DigitalAssetBuilder;
use crate::{db::models::*, test::utils::Test, types::AssetID};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use rand::prelude::*;
use serde_json::Value;
use uuid::Uuid;

#[allow(dead_code)]
pub struct AssetStateBuilder {
    pub name: String,
    pub description: String,
    pub limit_per_wallet: Option<u32>,
    pub allow_transfers: bool,
    pub asset_issuer_pub_key: String,
    pub authorized_signers: Vec<String>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub initial_permission_bitflag: i64,
    pub initial_data_json: Value,
    pub digital_asset_id: Option<Uuid>,
    pub asset_id: AssetID,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for AssetStateBuilder {
    fn default() -> Self {
        let x: u32 = random();
        Self {
            name: format!("Asset-{}", x).into(),
            description: "Description of asset".to_string(),
            limit_per_wallet: None,
            allow_transfers: true,
            asset_issuer_pub_key: format!("7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0{}", x)
                .into(),
            authorized_signers: Vec::new(),
            expiry_date: None,
            initial_permission_bitflag: 0,
            initial_data_json: serde_json::from_str("{}").unwrap(),
            digital_asset_id: None,
            asset_id: Test::<AssetID>::from_template(65536.into()), // TODO: Use a real asset ID here for consistency
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl AssetStateBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<AssetState> {
        let digital_asset_id = match self.digital_asset_id {
            Some(digital_asset_id) => digital_asset_id,
            None => DigitalAssetBuilder::default().build(client).await?.id,
        };
        let params = NewAssetState {
            name: self.name.to_owned(),
            description: self.description.to_owned(),
            limit_per_wallet: self.limit_per_wallet,
            allow_transfers: self.allow_transfers,
            asset_issuer_pub_key: self.asset_issuer_pub_key.to_owned(),
            authorized_signers: self.authorized_signers.to_owned(),
            expiry_date: self.expiry_date,
            initial_permission_bitflag: self.initial_permission_bitflag,
            initial_data_json: self.initial_data_json.to_owned(),
            asset_id: self.asset_id.to_owned(),
            digital_asset_id,
        };
        let asset_id = AssetState::insert(params, client).await?;
        Ok(AssetState::load(asset_id, client).await?)
    }
}
