use crate::db::models::*;
use tokio_postgres::Client;

#[allow(dead_code)]
pub struct DigitalAssetBuilder<'a> {
    template_type: String,
    committee_mode: CommitteeMode,
    node_threshold: Option<u32>,
    minimum_collateral: Option<i64>,
    consensus_strategy: Option<u32>,
    fqdn: Option<String>,
    digital_asset_template_id: i64,
    raid_id: Option<String>,
    client: &'a Client,
}

#[allow(dead_code)]
impl<'a> DigitalAssetBuilder<'a> {
    pub fn new(client: &'a Client) -> Self {
        DigitalAssetBuilder {
            template_type: "SingleUseDigitalAsset".to_string(),
            committee_mode: CommitteeMode::Creator,
            node_threshold: None,
            minimum_collateral: None,
            consensus_strategy: None,
            fqdn: None,
            digital_asset_template_id: 0,
            raid_id: None,
            client,
        }
    }

    pub fn with_template_type(mut self, template_type: String) -> Self {
        self.template_type = template_type;
        self
    }

    pub fn with_committee_mode(mut self, committee_mode: CommitteeMode) -> Self {
        self.committee_mode = committee_mode;
        self
    }

    pub fn with_node_threshold(mut self, node_threshold: u32) -> Self {
        self.node_threshold = Some(node_threshold);
        self
    }

    pub fn with_consensus_strategy(mut self, consensus_strategy: u32) -> Self {
        self.consensus_strategy = Some(consensus_strategy);
        self
    }

    pub fn with_raid_id(mut self, raid_id: String) -> Self {
        self.raid_id = Some(raid_id);
        self
    }

    pub async fn finish(&self) -> anyhow::Result<DigitalAsset> {
        let params = NewDigitalAsset {
            template_type: self.template_type.to_owned(),
            committee_mode: Some(self.committee_mode),
            node_threshold: self.node_threshold,
            minimum_collateral: self.minimum_collateral,
            consensus_strategy: self.consensus_strategy,
            fqdn: self.fqdn.to_owned(),
            digital_asset_template_id: self.digital_asset_template_id,
            raid_id: self.raid_id.to_owned(),
        };
        let digital_asset_id = DigitalAsset::insert(params, self.client).await?;
        Ok(DigitalAsset::load(digital_asset_id, self.client).await?)
    }
}
