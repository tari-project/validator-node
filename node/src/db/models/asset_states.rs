use super::AssetStatus;
use crate::{
    db::utils::{errors::DBError, validation::ValidationErrors},
    types::AssetID,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::Client;

#[derive(Serialize, PostgresMapper, PartialEq, Debug, Clone)]
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
    pub asset_id: AssetID,
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
    pub asset_id: AssetID,
    pub digital_asset_id: uuid::Uuid,
}

impl NewAssetState {
    pub async fn validate_record(&self, client: &Client) -> Result<(), DBError> {
        let mut validation_errors = ValidationErrors::default();
        if AssetState::find_by_asset_id(self.asset_id.clone(), client)
            .await?
            .is_some()
        {
            validation_errors.append_validation_error(
                "uniqueness",
                "asset_id",
                "New asset state must have unique asset ID.",
            );
        }
        validation_errors.validate()?;

        Ok(())
    }
}

impl AssetState {
    /// Add asset record
    pub async fn insert(params: NewAssetState, client: &Client) -> Result<uuid::Uuid, DBError> {
        params.validate_record(client).await?;

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
    pub async fn find_by_asset_id(asset_id: AssetID, client: &Client) -> Result<Option<AssetState>, DBError> {
        let stmt = "SELECT * FROM asset_states WHERE asset_id = $1";
        let result = client.query_opt(stmt, &[&asset_id]).await?;
        Ok(result.map(AssetState::from_row).transpose()?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::utils::validation::*,
        test_utils::{builders::*, load_env, test_db_client},
    };
    use std::collections::HashMap;

    const PUBKEY: &'static str = "7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0f";

    #[actix_rt::test]
    async fn crud() -> anyhow::Result<()> {
        load_env();
        let (client, _lock) = test_db_client().await;
        let digital_asset = DigitalAssetBuilder::default().build(&client).await.unwrap();
        let tari_asset_id: AssetID = "7e6f4b801170db0bf86c9257fe56249.469439556cba069a12afd1c72c585b0f"
            .parse()
            .unwrap();

        let params = NewAssetState {
            name: "AssetName".to_string(),
            description: "Description".to_string(),
            asset_issuer_pub_key: PUBKEY.to_string(),
            additional_data_json: serde_json::json!({"value": true}),
            asset_id: tari_asset_id.clone(),
            digital_asset_id: digital_asset.id,
            ..NewAssetState::default()
        };
        let asset_id = AssetState::insert(params, &client).await.unwrap();
        let asset = AssetState::load(asset_id, &client).await.unwrap();
        assert_eq!(asset.name, "AssetName".to_string());
        assert_eq!(asset.status, AssetStatus::Active);
        assert_eq!(asset.asset_issuer_pub_key, PUBKEY.to_string());
        assert_eq!(asset.digital_asset_id, digital_asset.id);
        assert_eq!(asset.asset_id, tari_asset_id.clone());

        let found_asset = AssetState::find_by_asset_id(tari_asset_id, &client).await.unwrap();
        assert_eq!(found_asset, Some(asset));
    }

    #[actix_rt::test]
    async fn asset_id_uniqueness() -> anyhow::Result<()> {
        load_env();
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await?;

        let params = NewAssetState {
            name: "AssetName".to_string(),
            description: "Description".to_string(),
            asset_issuer_pub_key: PUBKEY.to_string(),
            asset_id: asset.asset_id.clone(),
            digital_asset_id: asset.digital_asset_id,
            ..NewAssetState::default()
        };
        let mut expected_validation_errors = ValidationErrors::default();
        let expected_error = ValidationError {
            code: "uniqueness".into(),
            message: "New asset state must have unique asset ID.".into(),
        };
        expected_validation_errors.0.insert("asset_id", vec![expected_error]);

        let result = AssetState::insert(params, &client).await;
        assert!(result.is_err());
        if let Err(DBError::Validation(validation_errors)) = result {
            assert_eq!(validation_errors, expected_validation_errors);
        } else {
            panic!("Expected an error result response from validation test");
        }

        Ok(())
    }
}
