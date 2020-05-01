use super::DigitalAssetBuilder;
use crate::db::models::*;
use chrono::{DateTime, Utc};
use rand::prelude::*;
use serde_json::Value;
use tokio_postgres::Client;
use uuid::Uuid;

#[allow(dead_code)]
pub struct AssetStateBuilder<'a> {
    name: String,
    description: String,
    limit_per_wallet: Option<u32>,
    allow_transfers: bool,
    asset_issuer_pub_key: String,
    authorized_signers: Vec<String>,
    expiry_date: Option<DateTime<Utc>>,
    initial_permission_bitflag: i64,
    additional_data_json: Value,
    digital_asset_id: Option<Uuid>,
    asset_id: String,
    client: &'a Client,
}

#[allow(dead_code)]
impl<'a> AssetStateBuilder<'a> {
    pub fn new(client: &'a Client) -> Self {
        let x: u32 = random();
        AssetStateBuilder {
            name: format!("Asset-{}", x).into(),
            description: "Description of asset".to_string(),
            limit_per_wallet: None,
            allow_transfers: true,
            asset_issuer_pub_key: format!("7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0{}", x)
                .into(),
            authorized_signers: Vec::new(),
            expiry_date: None,
            initial_permission_bitflag: 0,
            additional_data_json: serde_json::from_str("{}").unwrap(),
            digital_asset_id: None,
            asset_id: format!("asset-id-placeholder-{}", x).into(), // TODO: Use a real asset ID here for consistency
            client,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_limit_per_wallet(mut self, limit_per_wallet: u32) -> Self {
        self.limit_per_wallet = Some(limit_per_wallet);
        self
    }

    pub fn allow_transfers(mut self, allow_transfers: bool) -> Self {
        self.allow_transfers = allow_transfers;
        self
    }

    pub fn with_asset_issuer_pub_key(mut self, asset_issuer_pub_key: String) -> Self {
        self.asset_issuer_pub_key = asset_issuer_pub_key;
        self
    }

    pub fn with_authorized_signer(mut self, authorized_signer: String) -> Self {
        self.authorized_signers.push(authorized_signer);
        self
    }

    pub fn with_expiry_date(mut self, expiry_date: DateTime<Utc>) -> Self {
        self.expiry_date = Some(expiry_date);
        self
    }

    pub fn with_initial_permission_bitflag(mut self, initial_permission_bitflag: i64) -> Self {
        self.initial_permission_bitflag = initial_permission_bitflag;
        self
    }

    pub fn with_additional_data_json(mut self, additional_data_json: Value) -> Self {
        self.additional_data_json = additional_data_json;
        self
    }

    pub fn with_digital_asset_id(mut self, digital_asset_id: Uuid) -> Self {
        self.digital_asset_id = Some(digital_asset_id);
        self
    }

    pub fn with_asset_id(mut self, asset_id: String) -> Self {
        self.asset_id = asset_id;
        self
    }

    pub async fn finish(&self) -> anyhow::Result<AssetState> {
        let digital_asset_id = match self.digital_asset_id {
            Some(digital_asset_id) => digital_asset_id,
            None => DigitalAssetBuilder::new(self.client).finish().await?.id,
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
            additional_data_json: self.additional_data_json.to_owned(),
            asset_id: self.asset_id.to_owned(),
            digital_asset_id,
        };
        let asset_id = AssetState::insert(params, self.client).await?;
        Ok(AssetState::load(asset_id, self.client).await?)
    }
}
