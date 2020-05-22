use crate::{
    db::models::{consensus::*, NewAssetStateAppendOnly, NewTokenStateAppendOnly, ViewStatus},
    test::utils::builders::AssetStateBuilder,
    types::{AssetID, NodeID, ProposalID},
};
use deadpool_postgres::Client;
use uuid::Uuid;

#[allow(dead_code)]
pub struct ViewBuilder {
    pub asset_id: Option<AssetID>,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub instruction_set: Vec<Uuid>,
    pub invalid_instruction_set: Vec<Uuid>,
    pub asset_state_append_only: Vec<NewAssetStateAppendOnly>,
    pub token_state_append_only: Vec<NewTokenStateAppendOnly>,
    pub proposal_id: Option<ProposalID>,
    pub status: Option<ViewStatus>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for ViewBuilder {
    fn default() -> Self {
        Self {
            asset_id: None,
            initiating_node_id: NodeID::stub(),
            signature: "stub-signature".to_string(),
            instruction_set: Vec::new(),
            invalid_instruction_set: Vec::new(),
            asset_state_append_only: Vec::new(),
            token_state_append_only: Vec::new(),
            proposal_id: None,
            status: None,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl ViewBuilder {
    pub async fn prepare(&self, client: &Client) -> anyhow::Result<NewView> {
        let asset_id = match &self.asset_id {
            Some(asset_id) => asset_id.clone(),
            None => AssetStateBuilder::default().build(client).await?.asset_id,
        };

        Ok(NewView {
            asset_id,
            initiating_node_id: self.initiating_node_id,
            signature: self.signature.clone(),
            instruction_set: self.instruction_set.clone(),
            invalid_instruction_set: self.invalid_instruction_set.clone(),
            asset_state_append_only: self.asset_state_append_only.clone(),
            token_state_append_only: self.token_state_append_only.clone(),
        })
    }

    pub async fn build(self, client: &Client) -> anyhow::Result<View> {
        Ok(View::insert(
            self.prepare(client).await?,
            NewViewAdditionalParameters {
                status: self.status,
                proposal_id: self.proposal_id,
            },
            client,
        )
        .await?)
    }
}
