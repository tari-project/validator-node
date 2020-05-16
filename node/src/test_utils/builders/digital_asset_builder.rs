use crate::db::models::*;
use tokio_postgres::Client;

#[allow(dead_code)]
pub struct DigitalAssetBuilder {
    pub template_type: u32,
    pub committee_mode: CommitteeMode,
    pub node_threshold: Option<u32>,
    pub minimum_collateral: Option<i64>,
    pub consensus_strategy: Option<u32>,
    pub fqdn: Option<String>,
    pub raid_id: Option<String>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for DigitalAssetBuilder {
    fn default() -> Self {
        Self {
            template_type: 1,
            committee_mode: CommitteeMode::Creator,
            node_threshold: None,
            minimum_collateral: None,
            consensus_strategy: None,
            fqdn: None,
            raid_id: None,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl DigitalAssetBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<DigitalAsset> {
        let params = NewDigitalAsset {
            template_type: 1,
            committee_mode: self.committee_mode,
            node_threshold: self.node_threshold,
            minimum_collateral: self.minimum_collateral,
            consensus_strategy: self.consensus_strategy,
            fqdn: self.fqdn.to_owned(),
            raid_id: self.raid_id.to_owned(),
        };
        let digital_asset_id = DigitalAsset::insert(params, client).await?;
        Ok(DigitalAsset::load(digital_asset_id, client).await?)
    }
}
