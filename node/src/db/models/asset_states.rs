use super::AssetStatus;
use crate::db::errors::DBError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::Client;

#[derive(Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "asset_states")]
pub struct AssetState {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub status: AssetStatus,
    pub limit_per_wallet: Option<u32>,
    pub allow_transfers: bool,
    pub asset_issuer_pub_key: String,
    pub authorized_signers: Vec<String>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub superseded_by: Option<uuid::Uuid>,
    pub initial_permission_bitflag: i64,
    pub additional_data_json: Value,
    pub asset_id: String,
    pub digital_asset_id: uuid::Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Query paramteres for adding new asset record
#[derive(Default, Clone, Debug)]
pub struct NewAssetState {
    pub name: String,
    pub description: String,
    pub limit_per_wallet: Option<u32>,
    pub allow_transfers: bool,
    pub asset_issuer_pub_key: String,
    pub authorized_signers: Vec<String>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub initial_permission_bitflag: i64,
    pub additional_data_json: Value,
    pub asset_id: String,
    pub digital_asset_id: uuid::Uuid,
}

impl AssetState {
    /// Add asset record
    pub async fn insert(params: NewAssetState, client: &Client) -> Result<uuid::Uuid, DBError> {
        const QUERY: &'static str = "
            INSERT INTO asset_states (
                name,
                description,
                limit_per_wallet,
                allow_transfers,
                asset_issuer_pub_key,
                authorized_signers,
                expiry_date,
                initial_permission_bitflag,
                additional_data_json,
                asset_id,
                digital_asset_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.name,
                &params.description,
                &params.limit_per_wallet,
                &params.allow_transfers,
                &params.asset_issuer_pub_key,
                &params.authorized_signers,
                &params.expiry_date,
                &params.initial_permission_bitflag,
                &params.additional_data_json,
                &params.asset_id,
                &params.digital_asset_id,
            ])
            .await?;

        Ok(result.get(0))
    }

    /// Load asset record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<AssetState, DBError> {
        let stmt = "SELECT * FROM asset_states WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(AssetState::from_row(result)?)
    }

    /// Find asset state record by asset id )
    pub async fn find_by_asset_id(asset_id: String, client: &Client) -> Result<AssetState, DBError> {
        let stmt = "SELECT * FROM asset_states WHERE asset_id = $1";
        let result = client.query_one(stmt, &[&asset_id]).await?;
        Ok(AssetState::from_row(result)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{build_test_config, build_test_pool, builders::*, reset_db};
    use std::collections::HashMap;
    const PUBKEY: &'static str = "7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0f";

    #[actix_rt::test]
    async fn crud() -> anyhow::Result<()> {
        dotenv::dotenv().unwrap();
        let db = build_test_pool().unwrap();
        let config = build_test_config().unwrap();
        reset_db(&config, &db).await.unwrap();
        let client = db.get().await.unwrap();
        let digital_asset = DigitalAssetBuilder::default().build(&client).await?;
        let tari_asset_id = "asset-id-placeholder-0976544466643335678667765432355555555445544".to_string();

        let mut additional_data_json = HashMap::new();
        additional_data_json.insert("value", true);
        let params = NewAssetState {
            name: "AssetName".to_string(),
            description: "Description".to_string(),
            limit_per_wallet: None,
            allow_transfers: false,
            asset_issuer_pub_key: PUBKEY.to_string(),
            authorized_signers: Vec::new(),
            expiry_date: None,
            initial_permission_bitflag: 0,
            additional_data_json: serde_json::to_value(additional_data_json)?,
            asset_id: tari_asset_id.clone(),
            digital_asset_id: digital_asset.id,
        };
        let asset_id = AssetState::insert(params, &client).await?;
        let asset = AssetState::load(asset_id, &client).await?;
        assert_eq!(asset.name, "AssetName".to_string());
        assert_eq!(asset.status, AssetStatus::Active);
        assert_eq!(asset.asset_issuer_pub_key, PUBKEY.to_string());
        assert_eq!(asset.digital_asset_id, digital_asset.id);
        assert_eq!(asset.asset_id, tari_asset_id.clone());

        let found_asset = AssetState::find_by_asset_id(tari_asset_id, &client).await?;
        assert_eq!(found_asset, asset);

        Ok(())
    }
}
