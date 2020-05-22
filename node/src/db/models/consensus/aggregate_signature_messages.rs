use super::Proposal;
use crate::{
    db::utils::errors::DBError,
    types::{consensus::SignatureData, ProposalID},
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use serde::{Deserialize, Serialize};
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};

#[derive(Clone, Deserialize, Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "aggregate_signature_messages")]
pub struct AggregateSignatureMessage {
    pub id: uuid::Uuid,
    pub proposal_id: ProposalID,
    pub signature_data: SignatureData,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewAggregateSignatureMessage {
    pub proposal_id: ProposalID,
    pub signature_data: SignatureData,
}

impl AggregateSignatureMessage {
    pub async fn find_pending(client: &Client) -> Result<Option<Self>, DBError> {
        Ok(None)
    }

    pub async fn validate(&self) -> Result<(), DBError> {
        // Stub
        Ok(())
    }

    pub async fn proposal(&self, client: &Client) -> Result<Proposal, DBError> {
        Proposal::load(self.proposal_id, client).await
    }

    pub async fn insert(params: NewAggregateSignatureMessage, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            INSERT INTO aggregate_signature_messages (
                proposal_id,
                signature_data
            ) VALUES ($1, $2) RETURNING *";
        let stmt = client.prepare(QUERY).await?;
        let row = client
            .query_one(&stmt, &[&params.proposal_id, &params.signature_data])
            .await?;
        Ok(Self::from_row(row)?)
    }
}

impl NewAggregateSignatureMessage {
    pub async fn save(&self, client: &Client) -> Result<AggregateSignatureMessage, DBError> {
        Ok(AggregateSignatureMessage::insert(self.clone(), &client).await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        test::utils::{
            builders::consensus::{AggregateSignatureMessageBuilder, ProposalBuilder},
            test_db_client,
        },
        types::NodeID,
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let signature_data = SignatureData {
            signatures: serde_json::from_value(json!({NodeID::stub(): "stub-signature"})).unwrap(),
        };
        let params = NewAggregateSignatureMessage {
            proposal_id: proposal.id,
            signature_data,
        };
        let aggregate_signature_message = AggregateSignatureMessage::insert(params, &client).await.unwrap();
        assert_eq!(aggregate_signature_message.proposal_id, proposal.id);
        assert_eq!(aggregate_signature_message.signature_data, signature_data);
    }

    #[actix_rt::test]
    async fn proposal() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let aggregate_signature_message = AggregateSignatureMessageBuilder {
            proposal_id: Some(proposal.id),
            ..AggregateSignatureMessageBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();

        let found_proposal = aggregate_signature_message.proposal(&client).await.unwrap();
        assert_eq!(found_proposal, proposal);
    }

    #[actix_rt::test]
    async fn save() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let signature_data = SignatureData {
            signatures: serde_json::from_value(json!({NodeID::stub(): "stub-signature"})).unwrap(),
        };
        let params = NewAggregateSignatureMessage {
            proposal_id: proposal.id,
            signature_data,
        };
        let aggregate_signature_message = params.save(&client).await.unwrap();
        assert_eq!(aggregate_signature_message.proposal_id, proposal.id);
        assert_eq!(aggregate_signature_message.signature_data, signature_data);
    }
}
