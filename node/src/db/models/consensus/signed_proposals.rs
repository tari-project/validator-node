use crate::{
    db::{models::SignedProposalStatus, utils::errors::DBError},
    types::{AssetID, NodeID, ProposalID},
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::Type;

#[derive(Deserialize, Serialize, PostgresMapper, PartialEq, Debug, Clone)]
#[pg_mapper(table = "signed_proposals")]
pub struct SignedProposal {
    pub id: uuid::Uuid,
    pub proposal_id: ProposalID,
    pub node_id: NodeID,
    pub signature: String,
    pub status: SignedProposalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewSignedProposal {
    pub proposal_id: ProposalID,
    pub node_id: NodeID,
    pub signature: String,
}

#[derive(Default, Clone, Debug)]
pub struct UpdateSignedProposal {
    pub status: Option<SignedProposalStatus>,
}

impl SignedProposal {
    pub async fn invalidate(signed_proposals: Vec<SignedProposal>, client: &Client) -> Result<(), DBError> {
        let signed_proposal_ids: Vec<uuid::Uuid> = signed_proposals.into_iter().map(|s| s.id).collect();

        const QUERY: &'static str = "
            UPDATE signed_proposals SET
                status = $2,
                updated_at = NOW()
            WHERE id = ANY ($1)
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::UUID_ARRAY, Type::TEXT]).await?;
        client
            .execute(&stmt, &[&signed_proposal_ids, &SignedProposalStatus::Invalid])
            .await?;

        Ok(())
    }

    /// Load signed proposal from database by ID
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<Self, DBError> {
        let stmt = "SELECT * FROM signed_proposals WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Self::from_row(result)?)
    }

    /// Update aggregate_signature_message state in the database
    ///
    /// Updates subset of fields:
    /// - status
    pub async fn update(&self, data: UpdateSignedProposal, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            UPDATE signed_proposals SET
                status = COALESCE($1, status),
                updated_at = NOW()
            WHERE id = $2
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::TEXT]).await?;
        let row = client.query_one(&stmt, &[&data.status, &self.id]).await?;
        Ok(Self::from_row(row)?)
    }

    pub async fn threshold_met(client: &Client) -> Result<HashMap<AssetID, Vec<SignedProposal>>, DBError> {
        // TODO: logic is currently hardcoded / stubbed for a committee of 1 so a single signed proposal meets the
        // threshold       we will need to iterate on this logic in the future to determine a viable threshold
        // dynamically by asset
        let stmt = "
            SELECT p.asset_id, sp.*
            FROM signed_proposals sp
            JOIN proposals p ON sp.proposal_id = p.id
            JOIN asset_states ast ON ast.asset_id = p.asset_id
            WHERE sp.status = 'Pending'
            AND ast.blocked_until <= now()
            ORDER BY p.asset_id
        ";
        let mut signed_proposal_data: Vec<(AssetID, SignedProposal)> = Vec::new();
        for row in client.query(stmt, &[]).await? {
            signed_proposal_data.push((row.get(0), SignedProposal::from_row(row)?));
        }

        let mut asset_id_signed_proposal_mapping = HashMap::new();
        for (asset_id, signed_proposal_data) in &signed_proposal_data.iter().group_by(|data| data.0.clone()) {
            asset_id_signed_proposal_mapping.insert(
                asset_id.clone(),
                signed_proposal_data.map(|d| d.1.clone()).collect_vec(),
            );
        }

        Ok(asset_id_signed_proposal_mapping)
    }

    /// Load signed proposals from database by ProposalID
    pub async fn load_by_proposal_id(id: ProposalID, client: &Client) -> Result<Vec<Self>, DBError> {
        let stmt = "SELECT * FROM signed_proposals WHERE proposal_id = $1::\"ProposalID\"";
        Ok(client
            .query(stmt, &[&id])
            .await?
            .into_iter()
            .map(|row| Self::from_row(row))
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn insert(params: NewSignedProposal, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            INSERT INTO signed_proposals (
                proposal_id,
                node_id,
                signature
            ) VALUES ($1, $2, $3) RETURNING *";
        let stmt = client.prepare(QUERY).await?;
        let row = client
            .query_one(&stmt, &[&params.proposal_id, &params.node_id, &params.signature])
            .await?;
        Ok(Self::from_row(row)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::{consensus::Proposal, *},
        test::utils::{builders::consensus::*, test_db_client},
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn threshold_met() {
        let (client, _lock) = test_db_client().await;
        let signed_proposal = SignedProposalBuilder::default().build(&client).await.unwrap();
        let signed_proposal2 = SignedProposalBuilder::default().build(&client).await.unwrap();
        let signed_proposal3 = SignedProposalBuilder::default().build(&client).await.unwrap();

        // signed_proposal is ignored if an existing block is present
        let proposal = Proposal::load(signed_proposal.proposal_id, &client).await.unwrap();
        let mut asset_state = AssetState::find_by_asset_id(&proposal.asset_id, &client)
            .await
            .unwrap()
            .unwrap();
        asset_state.acquire_lock(60 as u64, &client).await.unwrap();

        // signed_proposal3 is ignored as it is not pending
        signed_proposal3
            .update(
                UpdateSignedProposal {
                    status: Some(SignedProposalStatus::Validated),
                    ..UpdateSignedProposal::default()
                },
                &client,
            )
            .await
            .unwrap();

        let signed_proposals = SignedProposal::threshold_met(&client).await.unwrap();
        let proposal = Proposal::load(signed_proposal2.proposal_id, &client).await.unwrap();
        assert_eq!(
            json!(signed_proposals),
            json!({ proposal.asset_id.clone(): vec![signed_proposal2] })
        );
    }

    #[actix_rt::test]
    async fn load_by_proposal_id() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let signed_proposal = SignedProposalBuilder {
            proposal_id: Some(proposal.id),
            ..SignedProposalBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();

        assert_eq!(
            SignedProposal::load_by_proposal_id(proposal.id, &client).await.unwrap(),
            vec![signed_proposal]
        );
    }

    #[actix_rt::test]
    async fn invalidate() {
        let (client, _lock) = test_db_client().await;
        let signed_proposal = SignedProposalBuilder::default().build(&client).await.unwrap();
        let signed_proposal2 = SignedProposalBuilder::default().build(&client).await.unwrap();
        let signed_proposal3 = SignedProposalBuilder::default().build(&client).await.unwrap();
        assert_eq!(signed_proposal.status, SignedProposalStatus::Pending);
        assert_eq!(signed_proposal2.status, SignedProposalStatus::Pending);
        assert_eq!(signed_proposal3.status, SignedProposalStatus::Pending);

        SignedProposal::invalidate(vec![signed_proposal.clone(), signed_proposal3.clone()], &client)
            .await
            .unwrap();

        let signed_proposal = SignedProposal::load(signed_proposal.id, &client).await.unwrap();
        let signed_proposal2 = SignedProposal::load(signed_proposal2.id, &client).await.unwrap();
        let signed_proposal3 = SignedProposal::load(signed_proposal3.id, &client).await.unwrap();
        assert_eq!(signed_proposal.status, SignedProposalStatus::Invalid);
        assert_eq!(signed_proposal2.status, SignedProposalStatus::Pending);
        assert_eq!(signed_proposal3.status, SignedProposalStatus::Invalid);
    }

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;

        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let params = NewSignedProposal {
            proposal_id: proposal.id,
            node_id: NodeID::stub(),
            signature: "stub-signature".to_string(),
        };
        let signed_proposal = SignedProposal::insert(params, &client).await.unwrap();
        assert_eq!(signed_proposal.proposal_id, proposal.id);
        assert_eq!(signed_proposal.node_id, NodeID::stub());

        signed_proposal
            .update(
                UpdateSignedProposal {
                    status: Some(SignedProposalStatus::Validated),
                    ..UpdateSignedProposal::default()
                },
                &client,
            )
            .await
            .unwrap();

        let signed_proposal = SignedProposal::load(signed_proposal.id, &client).await.unwrap();
        assert_eq!(signed_proposal.status, SignedProposalStatus::Validated);
    }
}
