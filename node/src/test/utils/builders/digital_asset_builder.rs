use crate::{db::models::*, types::CommitteeMode};
use tokio_postgres::Client;

#[allow(dead_code)]
pub struct DigitalAssetBuilder {
    pub template_type: u32,
    pub committee_mode: CommitteeMode,
    pub fqdn: Option<String>,
    pub raid_id: Option<String>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for DigitalAssetBuilder {
    fn default() -> Self {
        Self {
            template_type: 1,
            committee_mode: CommitteeMode::Creator {
                trusted_node_set: Vec::new(),
            },
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
            fqdn: self.fqdn.to_owned(),
            raid_id: self.raid_id.to_owned(),
        };
        let digital_asset_id = DigitalAsset::insert(params, client).await?;
        Ok(DigitalAsset::load(digital_asset_id, client).await?)
    }
}
