use super::ProposalBuilder;
use crate::{
    db::models::{consensus::*, AggregateSignatureMessageStatus, ProposalStatus},
    test::utils::Test,
    types::{consensus::SignatureData, NodeID, ProposalID},
};
use deadpool_postgres::Client;
use serde_json::json;

#[allow(dead_code)]
pub struct AggregateSignatureMessageBuilder {
    pub proposal_id: Option<ProposalID>,
    pub signature_data: SignatureData,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for AggregateSignatureMessageBuilder {
    fn default() -> Self {
        Self {
            proposal_id: None,
            signature_data: SignatureData {
                signatures: serde_json::from_value(json!([[Test::<NodeID>::new(), "stub-signature"]])).unwrap(),
            },
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl AggregateSignatureMessageBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<AggregateSignatureMessage> {
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
        let params = NewAggregateSignatureMessage {
            proposal_id,
            signature_data: self.signature_data,
            status: AggregateSignatureMessageStatus::Pending,
        };
        Ok(AggregateSignatureMessage::insert(params, client).await?)
    }
}
