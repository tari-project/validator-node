use super::AssetStatus;
use crate::{
    db::utils::{errors::DBError, validation::ValidationErrors},
    types::{AssetID, InstructionID},
};
use bytes::BytesMut;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::{
    types::{accepts, to_sql_checked, FromSql, IsNull, Json, ToSql, Type},
    Client,
};

#[derive(Serialize, PostgresMapper, PartialEq, Debug, Clone)]
#[pg_mapper(table = "asset_states_view")]
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
    pub initial_data_json: Value,
    pub asset_id: AssetID,
    pub digital_asset_id: uuid::Uuid,
    pub blocked_until: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub additional_data_json: Value,
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
    pub initial_data_json: Value,
    pub asset_id: AssetID,
    pub digital_asset_id: uuid::Uuid,
}

/// Query parameters for adding new token state append only
#[derive(PartialEq, Default, Clone, Debug, Serialize, Deserialize)]
pub struct NewAssetStateAppendOnly {
    pub asset_id: AssetID,
    pub instruction_id: InstructionID,
    pub state_data_json: Value,
    pub status: AssetStatus,
}

impl NewAssetState {
    pub async fn validate_record(&self, client: &Client) -> Result<(), DBError> {
        let mut validation_errors = ValidationErrors::default();
        if AssetState::find_by_asset_id(&self.asset_id, client).await?.is_some() {
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
    /// Releases lock on asset state
    pub async fn acquire_lock(&mut self, lock_period: u64, client: &Client) -> Result<(), DBError> {
        let block_until = Utc::now() + Duration::seconds(lock_period as i64);

        const QUERY: &'static str =
            "UPDATE asset_states SET blocked_until = $2, updated_at = now() WHERE id = $1 AND blocked_until <= now()";
        let stmt = client.prepare(QUERY).await?;
        client.execute(&stmt, &[&self.id, &block_until]).await?;

        Ok(())
    }

    /// Releases lock on asset state
    pub async fn release_lock(&self, client: &Client) -> Result<(), DBError> {
        let block_until = Utc::now();
        const QUERY: &'static str =
            "UPDATE asset_states SET blocked_until = $3, updated_at = now() WHERE id = $1 AND blocked_until = $2";
        let stmt = client.prepare(QUERY).await?;
        client
            .execute(&stmt, &[&self.id, &self.blocked_until, &block_until])
            .await?;

        Ok(())
    }

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
                initial_data_json,
                asset_id,
                digital_asset_id,
                blocked_until
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id";
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
                &params.initial_data_json,
                &params.asset_id,
                &params.digital_asset_id,
                &Utc::now(),
            ])
            .await?;

        Ok(result.get(0))
    }

    /// Load asset record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<AssetState, DBError> {
        let stmt = "SELECT * FROM asset_states_view WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(AssetState::from_row(result)?)
    }

    /// Find asset state record by asset id
    pub async fn find_by_asset_id(asset_id: &AssetID, client: &Client) -> Result<Option<AssetState>, DBError> {
        let stmt = "SELECT * FROM asset_states_view WHERE asset_id = $1";
        let result = client.query_opt(stmt, &[&asset_id]).await?;
        Ok(result.map(AssetState::from_row).transpose()?)
    }

    // Store append only state
    pub async fn store_append_only_state(
        params: &NewAssetStateAppendOnly,
        client: &Client,
    ) -> Result<uuid::Uuid, DBError>
    {
        const QUERY: &'static str = "
            INSERT INTO asset_state_append_only (
                asset_id,
                state_data_json,
                instruction_id,
                status
            ) VALUES ($1, $2, $3, $4) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.asset_id,
                &params.state_data_json,
                &params.instruction_id,
                &params.status,
            ])
            .await?;

        Ok(result.get(0))
    }
}

impl<'a> ToSql for NewAssetStateAppendOnly {
    accepts!(JSON, JSONB);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        json!(self).to_sql(ty, w)
    }
}

impl<'a> FromSql<'a> for NewAssetStateAppendOnly {
    accepts!(JSON, JSONB);

    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(serde_json::from_value(
            Json::<Value>::from_sql(ty, raw).map(|json| json.0)?,
        )?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::{models::InstructionStatus, utils::validation::*},
        test::utils::{
            builders::{consensus::InstructionBuilder, AssetStateBuilder, DigitalAssetBuilder},
            test_db_client,
        },
    };
    use serde_json::json;

    const PUBKEY: &'static str = "7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0f";

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let digital_asset = DigitalAssetBuilder::default().build(&client).await.unwrap();
        let tari_asset_id: AssetID = "7e6f4b801170db0bf86c9257fe56249.469439556cba069a12afd1c72c585b0f"
            .parse()
            .unwrap();

        let params = NewAssetState {
            name: "AssetName".to_string(),
            description: "Description".to_string(),
            asset_issuer_pub_key: PUBKEY.to_string(),
            initial_data_json: json!({"value": true}),
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

        let found_asset = AssetState::find_by_asset_id(&tari_asset_id, &client).await.unwrap();
        assert_eq!(found_asset, Some(asset));
    }

    #[actix_rt::test]
    async fn store_append_only_state() -> anyhow::Result<()> {
        let (client, _lock) = test_db_client().await;
        let initial_data = json!({"value": true, "value2": 4});
        let asset = AssetStateBuilder {
            initial_data_json: initial_data.clone(),
            ..AssetStateBuilder::default()
        }
        .build(&client)
        .await?;
        assert_eq!(initial_data, asset.initial_data_json);
        assert_eq!(initial_data, asset.additional_data_json);

        let instruction = InstructionBuilder {
            asset_id: Some(asset.asset_id.clone()),
            status: InstructionStatus::Commit,
            ..Default::default()
        }
        .build(&client)
        .await
        .unwrap();
        let empty_value: Option<String> = None;
        let state_data_json = json!({"value": empty_value.clone(), "value2": 8, "value3": 2});
        AssetState::store_append_only_state(
            &NewAssetStateAppendOnly {
                asset_id: asset.asset_id,
                state_data_json: state_data_json.clone(),
                instruction_id: instruction.id.clone(),
                ..Default::default()
            },
            &client,
        )
        .await?;
        let asset = AssetState::load(asset.id, &client).await?;
        assert_eq!(state_data_json, asset.additional_data_json);
        assert_eq!(asset.status, AssetStatus::Active);

        let state_data_json = json!({"value": false, "value3": empty_value.clone()});
        AssetState::store_append_only_state(
            &NewAssetStateAppendOnly {
                asset_id: asset.asset_id,
                state_data_json: state_data_json.clone(),
                instruction_id: instruction.id.clone(),
                status: AssetStatus::Retired,
            },
            &client,
        )
        .await?;
        let asset = AssetState::load(asset.id, &client).await?;
        assert_eq!(state_data_json.clone(), asset.additional_data_json);
        assert_eq!(asset.status, AssetStatus::Retired);

        Ok(())
    }

    #[actix_rt::test]
    async fn asset_id_uniqueness() -> anyhow::Result<()> {
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
