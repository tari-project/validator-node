use super::CommitteeMode;
use crate::db::utils::{errors::DBError, validation::ValidationErrors};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::Client;

#[derive(Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "digital_assets")]
pub struct DigitalAsset {
    pub id: uuid::Uuid,
    pub template_type: String,
    pub committee_mode: CommitteeMode,
    pub node_threshold: Option<u32>,
    pub minimum_collateral: Option<i64>,
    pub consensus_strategy: Option<u32>,
    pub fqdn: Option<String>,
    pub digital_asset_template_id: i64,
    pub raid_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Query paramteres for adding new digital asset record
#[derive(Default, Clone, Debug)]
pub struct NewDigitalAsset {
    pub template_type: String,
    pub committee_mode: CommitteeMode,
    pub node_threshold: Option<u32>,
    pub minimum_collateral: Option<i64>,
    pub consensus_strategy: Option<u32>,
    pub fqdn: Option<String>,
    pub digital_asset_template_id: i64,
    pub raid_id: Option<String>,
}

impl NewDigitalAsset {
    pub fn validate_record(&self) -> Result<(), DBError> {
        let mut validation_errors = ValidationErrors::default();

        if self.committee_mode == CommitteeMode::Public {
            if self.node_threshold.is_none() {
                validation_errors.append_validation_error(
                    "required",
                    "node_threshold",
                    "Node threshold is required for digital assets in public committee mode.",
                );
            }

            if self.minimum_collateral.is_none() {
                validation_errors.append_validation_error(
                    "required",
                    "minimum_collateral",
                    "Minimum collateral is required for digital assets in public committee mode.",
                );
            }

            if self.consensus_strategy.is_none() {
                validation_errors.append_validation_error(
                    "required",
                    "consensus_strategy",
                    "Consensus strategy is required for digital assets in public committee mode.",
                );
            }
        }
        validation_errors.validate()?;

        Ok(())
    }
}

impl DigitalAsset {
    /// Add digital asset record
    pub async fn insert(params: NewDigitalAsset, client: &Client) -> Result<uuid::Uuid, DBError> {
        params.validate_record()?;

        const QUERY: &'static str = "
            INSERT INTO digital_assets (
                template_type,
                committee_mode,
                node_threshold,
                minimum_collateral,
                consensus_strategy,
                fqdn,
                digital_asset_template_id,
                raid_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.template_type,
                &params.committee_mode,
                &params.node_threshold,
                &params.minimum_collateral,
                &params.consensus_strategy,
                &params.fqdn,
                &params.digital_asset_template_id,
                &params.raid_id,
            ])
            .await?;

        Ok(result.get(0))
    }

    /// Load digital asset record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<DigitalAsset, DBError> {
        let stmt = "SELECT * FROM digital_assets WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(DigitalAsset::from_row(result)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{db::utils::validation::*, test_utils::test_db_client};

    #[actix_rt::test]
    async fn crud() -> anyhow::Result<()> {
        dotenv::dotenv().unwrap();
        let (client, _lock) = test_db_client().await;
        let params = NewDigitalAsset {
            template_type: "SingleUseDigitalAsset".to_string(),
            committee_mode: CommitteeMode::Creator,
            ..NewDigitalAsset::default()
        };
        let digital_asset_id = DigitalAsset::insert(params, &client).await?;
        let digital_asset = DigitalAsset::load(digital_asset_id, &client).await?;
        assert_eq!(digital_asset.template_type, "SingleUseDigitalAsset".to_string());
        assert_eq!(digital_asset.committee_mode, CommitteeMode::Creator);

        Ok(())
    }

    #[actix_rt::test]
    async fn public_committee_required_fields() -> anyhow::Result<()> {
        dotenv::dotenv().unwrap();
        let (client, _lock) = test_db_client().await;
        let params = NewDigitalAsset {
            template_type: "SingleUseDigitalAsset".to_string(),
            committee_mode: CommitteeMode::Public,
            ..NewDigitalAsset::default()
        };
        let mut expected_validation_errors = ValidationErrors::default();
        let expected_error = ValidationError {
            code: "required".into(),
            message: "Node threshold is required for digital assets in public committee mode.".into(),
        };
        expected_validation_errors
            .0
            .insert("node_threshold", vec![expected_error]);

        let expected_error = ValidationError {
            code: "required".into(),
            message: "Minimum collateral is required for digital assets in public committee mode.".into(),
        };
        expected_validation_errors
            .0
            .insert("minimum_collateral", vec![expected_error]);

        let expected_error = ValidationError {
            code: "required".into(),
            message: "Consensus strategy is required for digital assets in public committee mode.".into(),
        };
        expected_validation_errors
            .0
            .insert("consensus_strategy", vec![expected_error]);

        let result = DigitalAsset::insert(params, &client).await;
        assert!(result.is_err());
        if let Err(DBError::Validation(validation_errors)) = result {
            assert_eq!(validation_errors, expected_validation_errors);
        } else {
            panic!("Expected an error result response from validation test");
        }

        Ok(())
    }
}
