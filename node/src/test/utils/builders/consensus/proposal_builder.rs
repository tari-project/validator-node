use crate::{
    db::models::{consensus::*, ProposalStatus},
    test::utils::{builders::consensus::ViewBuilder, Test},
    types::{NodeID, ProposalID},
};
use deadpool_postgres::Client;

#[allow(dead_code)]
pub struct ProposalBuilder {
    pub id: Option<ProposalID>,
    pub new_view: Option<NewView>,
    pub node_id: Option<NodeID>,
    pub status: Option<ProposalStatus>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for ProposalBuilder {
    fn default() -> Self {
        Self {
            id: None,
            new_view: None,
            node_id: None,
            status: None,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl ProposalBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<Proposal> {
        let id = match self.id {
            Some(id) => id,
            None => ProposalID::new(Test::<NodeID>::new()).await?,
        };
        let new_view = match self.new_view {
            Some(new_view) => new_view,
            None => ViewBuilder::default().prepare(client).await?,
        };

        let params = NewProposal {
            node_id: self.node_id.unwrap_or_else(|| Test::<NodeID>::new()),
            asset_id: new_view.asset_id.clone(),
            id,
            new_view,
        };
        let proposal = Proposal::insert(params, client).await?;
        if let Some(status) = self.status {
            proposal
                .update(
                    UpdateProposal {
                        status: Some(status),
                        ..UpdateProposal::default()
                    },
                    &client,
                )
                .await?;
        }

        Ok(proposal)
    }
}
