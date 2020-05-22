use crate::{
    db::models::consensus::*,
    test::utils::builders::consensus::ViewBuilder,
    types::{NodeID, ProposalID},
};
use deadpool_postgres::Client;

#[allow(dead_code)]
pub struct ProposalBuilder {
    id: Option<ProposalID>,
    new_view: Option<NewView>,
    node_id: Option<NodeID>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for ProposalBuilder {
    fn default() -> Self {
        Self {
            id: None,
            new_view: None,
            node_id: None,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl ProposalBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<Proposal> {
        let id = match self.id {
            Some(id) => id,
            None => ProposalID::new(NodeID::stub()).await?,
        };
        let new_view = match self.new_view {
            Some(new_view) => new_view,
            None => ViewBuilder::default().prepare(client).await?,
        };

        let params = NewProposal {
            node_id: self.node_id.unwrap_or_else(|| NodeID::stub()),
            asset_id: new_view.asset_id.clone(),
            id,
            new_view,
        };
        Ok(Proposal::insert(params, client).await?)
    }
}
