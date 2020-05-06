use super::CommitteeMode;
use crate::db::errors::DBError;
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
    pub committee_mode: Option<CommitteeMode>,
    pub node_threshold: Option<u32>,
    pub minimum_collateral: Option<i64>,
    pub consensus_strategy: Option<u32>,
    pub fqdn: Option<String>,
    pub digital_asset_template_id: i64,
    pub raid_id: Option<String>,
}

impl DigitalAsset {
    /// Add digital asset record
    pub async fn insert(params: NewDigitalAsset, client: &Client) -> Result<uuid::Uuid, DBError> {
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
                &params.committee_mode.unwrap_or(CommitteeMode::Public),
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
    use crate::test_utils::{build_test_config, reset_db, test_pool};

    #[actix_rt::test]
    async fn crud() -> anyhow::Result<()> {
        dotenv::dotenv().unwrap();
        let db = test_pool().await;
        let config = build_test_config().unwrap();
        reset_db(&config, &db).await.unwrap();
        let client = db.get().await.unwrap();
        let params = NewDigitalAsset {
            template_type: "SingleUseDigitalAsset".to_string(),
            committee_mode: Some(CommitteeMode::Creator),
            node_threshold: None,
            minimum_collateral: None,
            consensus_strategy: None,
            fqdn: None,
            digital_asset_template_id: 0,
            raid_id: None,
        };
        let digital_asset_id = DigitalAsset::insert(params, &client).await?;
        let digital_asset = DigitalAsset::load(digital_asset_id, &client).await?;
        assert_eq!(digital_asset.template_type, "SingleUseDigitalAsset".to_string());
        assert_eq!(digital_asset.committee_mode, CommitteeMode::Creator);

        Ok(())
    }
}
