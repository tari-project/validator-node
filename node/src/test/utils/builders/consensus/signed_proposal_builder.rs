use crate::{
    db::models::{consensus::*, ProposalStatus},
    test::utils::builders::consensus::ProposalBuilder,
    types::{NodeID, ProposalID},
};
use deadpool_postgres::Client;

#[allow(dead_code)]
pub struct SignedProposalBuilder {
    pub node_id: NodeID,
    pub signature: String,
    pub proposal_id: Option<ProposalID>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for SignedProposalBuilder {
    fn default() -> Self {
        Self {
            node_id: NodeID::stub(),
            signature: "stub-signature".to_string(),
            proposal_id: None,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl SignedProposalBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<SignedProposal> {
        let proposal_id = match self.proposal_id {
            Some(proposal_id) => proposal_id,
            None => {
                ProposalBuilder {
                    status: Some(ProposalStatus::Signed),
                    ..ProposalBuilder::default()
                }
                .build(client)
                .await?
                .id
            },
        };
        let params = NewSignedProposal {
            proposal_id,
            node_id: self.node_id,
            signature: self.signature,
        };
        Ok(SignedProposal::insert(params, client).await?)
    }
}
