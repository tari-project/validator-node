use crate::db::models::*;
use tokio_postgres::Client;

#[allow(dead_code)]
pub struct DigitalAssetBuilder {
    pub template_type: String,
    pub committee_mode: CommitteeMode,
    pub node_threshold: Option<u32>,
    pub minimum_collateral: Option<i64>,
    pub consensus_strategy: Option<u32>,
    pub fqdn: Option<String>,
    pub digital_asset_template_id: i64,
    pub raid_id: Option<String>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for DigitalAssetBuilder {
    fn default() -> Self {
        Self {
            template_type: "SingleUseDigitalAsset".to_string(),
            committee_mode: CommitteeMode::Creator,
            node_threshold: None,
            minimum_collateral: None,
            consensus_strategy: None,
            fqdn: None,
            digital_asset_template_id: 0,
            raid_id: None,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl DigitalAssetBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<DigitalAsset> {
        let params = NewDigitalAsset {
            template_type: self.template_type.to_owned(),
            committee_mode: self.committee_mode,
            node_threshold: self.node_threshold,
            minimum_collateral: self.minimum_collateral,
            consensus_strategy: self.consensus_strategy,
            fqdn: self.fqdn.to_owned(),
            digital_asset_template_id: self.digital_asset_template_id,
            raid_id: self.raid_id.to_owned(),
        };
        let digital_asset_id = DigitalAsset::insert(params, client).await?;
        Ok(DigitalAsset::load(digital_asset_id, client).await?)
    }
}
