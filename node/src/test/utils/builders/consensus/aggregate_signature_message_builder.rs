use super::ProposalBuilder;
use crate::{db::models::*, types::AssetID};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use rand::prelude::*;
use serde_json::Value;
use uuid::Uuid;

#[allow(dead_code)]
pub struct AggregateSignatureMessageBuilder {
    proposal_id: Option<Uuid>,
}

impl Default for AggregateSignatureMessageBuilder {
    fn default() -> Self {
        Self {
            proposal_id: None,
            signature_data: SignatureData { signatures: serde_json::from_value(json!({NodeID::stub(): "stub-signature"}))};
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl AggregateSignatureMessageBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<AggregateSignatureMessage> {
        let proposal_id = match self.proposal_id {
            Some(proposal_id) => proposal_id,
            None => ProposalBuilder::default().build(client).await?.id,
        };
        let params = NewAggregateSignatureMessage {
            proposal_id,
            signature_data,
        };
        let asset_id = AggregateSignatureMessage::insert(params, client).await?;
        Ok(AggregateSignatureMessage::load(asset_id, client).await?)
    }
}
