use crate::{
    db::utils::errors::DBError,
    types::{CommitteeMode, TemplateID},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::Client;

#[derive(Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "digital_assets")]
pub struct DigitalAsset {
    pub id: uuid::Uuid,
    pub template_type: u32,
    pub committee_mode: CommitteeMode,
    pub fqdn: Option<String>,
    pub raid_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Query paramteres for adding new digital asset record
#[derive(Default, Clone, Debug)]
pub struct NewDigitalAsset {
    pub template_type: u32,
    pub committee_mode: CommitteeMode,
    pub fqdn: Option<String>,
    pub raid_id: Option<String>,
}

impl DigitalAsset {
    /// Add digital asset record
    pub async fn insert(params: NewDigitalAsset, client: &Client) -> Result<uuid::Uuid, DBError> {
        const QUERY: &'static str = "
            INSERT INTO digital_assets (
                template_type,
                committee_mode,
                fqdn,
                raid_id
            ) VALUES ($1, $2, $3, $4) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.template_type,
                &params.committee_mode,
                &params.fqdn,
                &params.raid_id,
            ])
            .await?;

        Ok(result.get(0))
    }

    /// Find digital asset records by template id
    pub async fn find_by_template_id(template_id: &TemplateID, client: &Client) -> Result<Vec<Self>, DBError> {
        let stmt = "SELECT * FROM digital_assets WHERE template_type = $1";
        let ttype = template_id.template_type();
        let results = client.query(stmt, &[&ttype]).await?;
        Ok(results.into_iter().map(Self::from_row).collect::<Result<Vec<_>, _>>()?)
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
    use crate::{
        test::utils::{load_env, test_db_client},
        types::NodeSelectionStrategy,
    };

    #[actix_rt::test]
    async fn crud() -> anyhow::Result<()> {
        load_env();
        let (client, _lock) = test_db_client().await;
        let params = NewDigitalAsset {
            template_type: 1,
            committee_mode: CommitteeMode::Public {
                node_threshold: 5,
                minimum_collateral: 1000,
                node_selection_strategy: NodeSelectionStrategy::RegisterAll,
            },
            ..NewDigitalAsset::default()
        };
        let digital_asset_id = DigitalAsset::insert(params, &client).await?;
        let digital_asset = DigitalAsset::load(digital_asset_id, &client).await?;
        assert_eq!(digital_asset.template_type, 1);
        assert_eq!(digital_asset.committee_mode, CommitteeMode::Public {
            node_threshold: 5,
            minimum_collateral: 1000,
            node_selection_strategy: NodeSelectionStrategy::RegisterAll
        });

        Ok(())
    }
}
