use super::Proposal;
use crate::{
    db::{models::AggregateSignatureMessageStatus, utils::errors::DBError},
    types::{consensus::SignatureData, ProposalID},
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use serde::{Deserialize, Serialize};
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::Type;

#[derive(Clone, Deserialize, Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "aggregate_signature_messages")]
pub struct AggregateSignatureMessage {
    pub id: uuid::Uuid,
    pub proposal_id: ProposalID,
    pub signature_data: SignatureData,
    pub status: AggregateSignatureMessageStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewAggregateSignatureMessage {
    pub proposal_id: ProposalID,
    pub signature_data: SignatureData,
    pub status: AggregateSignatureMessageStatus,
}

#[derive(Default, Clone, Debug)]
pub struct UpdateAggregateSignatureMessage {
    pub status: Option<AggregateSignatureMessageStatus>,
}

impl AggregateSignatureMessage {
    pub async fn find_pending(client: &Client) -> Result<Option<Self>, DBError> {
        let stmt = "
            SELECT asm.*
            FROM aggregate_signature_messages asm
            JOIN (
                SELECT asm.proposal_id
                FROM aggregate_signature_messages asm
                JOIN proposals p ON asm.proposal_id = p.id
                JOIN asset_states ast ON ast.asset_id = p.asset_id
                WHERE asm.status = 'Pending'
                AND ast.blocked_until <= now()
            ) asm2 ON asm.proposal_id = asm2.proposal_id
            AND asm.status = 'Pending'
            LIMIT 1
        ";

        let aggregate_signature_message: Option<AggregateSignatureMessage> = match client.query_opt(stmt, &[]).await? {
            Some(row) => Some(AggregateSignatureMessage::from_row(row)?),
            None => None,
        };
        Ok(aggregate_signature_message)
    }

    /// Load aggregate signature message from database by ID
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<Self, DBError> {
        let stmt = "SELECT * FROM aggregate_signature_messages WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Self::from_row(result)?)
    }

    pub async fn validate(&self, client: &Client) -> Result<(), DBError> {
        // Stub, always validates as valid
        self.update(
            UpdateAggregateSignatureMessage {
                status: Some(AggregateSignatureMessageStatus::Accepted),
            },
            client,
        )
        .await?;

        Ok(())
    }

    /// Update aggregate_signature_message state in the database
    ///
    /// Updates subset of fields:
    /// - status
    pub async fn update(&self, data: UpdateAggregateSignatureMessage, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            UPDATE aggregate_signature_messages SET
                status = COALESCE($1, status),
                updated_at = NOW()
            WHERE id = $2
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::TEXT]).await?;
        let row = client.query_one(&stmt, &[&data.status, &self.id]).await?;
        Ok(Self::from_row(row)?)
    }

    /// Load aggregate signature messages from database by ProposalID
    pub async fn load_by_proposal_id(id: ProposalID, client: &Client) -> Result<Vec<Self>, DBError> {
        let stmt = "SELECT * FROM aggregate_signature_messages WHERE proposal_id = $1::\"ProposalID\"";
        Ok(client
            .query(stmt, &[&id])
            .await?
            .into_iter()
            .map(|row| Self::from_row(row))
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn proposal(&self, client: &Client) -> Result<Proposal, DBError> {
        Proposal::load(self.proposal_id, client).await
    }

    pub async fn insert(params: NewAggregateSignatureMessage, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            INSERT INTO aggregate_signature_messages (
                proposal_id,
                signature_data,
                status
            ) VALUES ($1, $2, $3) RETURNING *";
        let stmt = client.prepare(QUERY).await?;
        let row = client
            .query_one(&stmt, &[&params.proposal_id, &params.signature_data, &params.status])
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
        db::models::AssetState,
        test::utils::{
            builders::consensus::{AggregateSignatureMessageBuilder, ProposalBuilder},
            test_db_client,
        },
        types::NodeID,
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn find_pending() {
        let (client, _lock) = test_db_client().await;
        let aggregate_signature_message = AggregateSignatureMessageBuilder::default()
            .build(&client)
            .await
            .unwrap();
        let aggregate_signature_message2 = AggregateSignatureMessageBuilder::default()
            .build(&client)
            .await
            .unwrap();
        let aggregate_signature_message3 = AggregateSignatureMessageBuilder::default()
            .build(&client)
            .await
            .unwrap();

        // Status not pending
        aggregate_signature_message2
            .update(
                UpdateAggregateSignatureMessage {
                    status: Some(AggregateSignatureMessageStatus::Accepted),
                    ..UpdateAggregateSignatureMessage::default()
                },
                &client,
            )
            .await
            .unwrap();

        // Asset is locked
        let proposal = Proposal::load(aggregate_signature_message3.proposal_id, &client)
            .await
            .unwrap();
        let mut asset_state = AssetState::find_by_asset_id(&proposal.asset_id, &client)
            .await
            .unwrap()
            .unwrap();
        asset_state.acquire_lock(60 as u64, &client).await.unwrap();

        let found_aggregate_signature_messages = AggregateSignatureMessage::find_pending(&client).await.unwrap();
        assert_eq!(found_aggregate_signature_messages, Some(aggregate_signature_message));
    }

    #[actix_rt::test]
    async fn load() {
        let (client, _lock) = test_db_client().await;
        let aggregate_signature_message = AggregateSignatureMessageBuilder::default()
            .build(&client)
            .await
            .unwrap();

        let found_aggregate_signature_message =
            AggregateSignatureMessage::load(aggregate_signature_message.id, &client)
                .await
                .unwrap();
        assert_eq!(aggregate_signature_message, found_aggregate_signature_message);
    }

    #[actix_rt::test]
    async fn validate() {
        let (client, _lock) = test_db_client().await;
        let aggregate_signature_message = AggregateSignatureMessageBuilder::default()
            .build(&client)
            .await
            .unwrap();
        // TODO: augment test logic for passing and failing validation once no longer a stub
        assert!(aggregate_signature_message.validate(&client).await.is_ok());
    }

    #[actix_rt::test]
    async fn load_by_proposal_id() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let aggregate_signature_message = AggregateSignatureMessageBuilder {
            proposal_id: Some(proposal.id),
            ..AggregateSignatureMessageBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();

        let aggregate_signature_messages = AggregateSignatureMessage::load_by_proposal_id(proposal.id, &client)
            .await
            .unwrap();
        assert_eq!(aggregate_signature_messages, vec![aggregate_signature_message]);
    }

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let signature_data = SignatureData {
            signatures: serde_json::from_value(json!([[NodeID::stub(), "stub-signature"]])).unwrap(),
        };
        let params = NewAggregateSignatureMessage {
            proposal_id: proposal.id,
            signature_data: signature_data.clone(),
            status: AggregateSignatureMessageStatus::Pending,
        };
        let mut aggregate_signature_message = AggregateSignatureMessage::insert(params, &client).await.unwrap();
        assert_eq!(aggregate_signature_message.proposal_id, proposal.id);
        assert_eq!(aggregate_signature_message.signature_data, signature_data);
        assert_eq!(
            aggregate_signature_message.status,
            AggregateSignatureMessageStatus::Pending
        );

        aggregate_signature_message = aggregate_signature_message
            .update(
                UpdateAggregateSignatureMessage {
                    status: Some(AggregateSignatureMessageStatus::Accepted),
                    ..UpdateAggregateSignatureMessage::default()
                },
                &client,
            )
            .await
            .unwrap();
        assert_eq!(aggregate_signature_message.proposal_id, proposal.id);
        assert_eq!(aggregate_signature_message.signature_data, signature_data);
        assert_eq!(
            aggregate_signature_message.status,
            AggregateSignatureMessageStatus::Accepted
        );
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
            signatures: serde_json::from_value(json!([[NodeID::stub(), "stub-signature"]])).unwrap(),
        };
        let params = NewAggregateSignatureMessage {
            proposal_id: proposal.id,
            signature_data: signature_data.clone(),
            status: AggregateSignatureMessageStatus::Pending,
        };
        let aggregate_signature_message = params.save(&client).await.unwrap();
        assert_eq!(aggregate_signature_message.proposal_id, proposal.id);
        assert_eq!(aggregate_signature_message.signature_data, signature_data);
    }
}
