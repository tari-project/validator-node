use super::AssetStateBuilder;
use crate::{db::models::*, types::TemplateID};
use chrono::Local;
use deadpool_postgres::Client;
use serde_json::{json, Value};
use uuid::{
    v1::{Context, Timestamp},
    Uuid,
};

#[allow(dead_code)]
pub struct SignedProposalBuilder {
    pub node_id: NodeID,
    pub signature: Option<String>,
    pub proposal_id: Option<Uuid>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for SignedProposalBuilder {
    fn default() -> Self {
        let x: u32 = random();
        Self {
            node_id: NodeID::stub(),
            signature: "stub-signature",
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
            None => ProposalBuilder::default().build(client).await?,
        };
        let params = NewSignedProposal {
            proposal_id,
            node_id: self.node_id,
            signature: self.signature,
        };
        let signed_proposal_id = SignedProposal::insert(params, client).await?;
        Ok(SignedProposal::load(signed_proposal_id, client).await?)
    }
}
