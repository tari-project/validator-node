use crate::{
    db::{
        models::{consensus::*, ProposalStatus},
        utils::errors::DBError,
    },
    types::{AssetID, NodeID, ProposalID},
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use serde::{Deserialize, Serialize};
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::Type;

#[derive(Clone, Deserialize, Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "proposals")]
pub struct Proposal {
    pub id: ProposalID,
    pub new_view: NewView,
    pub asset_id: AssetID,
    pub node_id: NodeID,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewProposal {
    pub id: ProposalID,
    pub new_view: NewView,
    pub asset_id: AssetID,
    pub node_id: NodeID,
}

/// Query parameters for optionally updating instruction fields
#[derive(Default, Clone, Debug)]
pub struct UpdateProposal {
    pub status: Option<ProposalStatus>,
}

impl Proposal {
    pub async fn find_pending(client: &Client) -> Result<Option<Self>, DBError> {
        let stmt = "
            SELECT p.*
            FROM proposals p
            JOIN asset_states ast ON ast.asset_id = p.asset_id
            WHERE p.status = 'Pending'
            AND ast.blocked_until <= now()
            LIMIT 1
        ";

        Ok(client.query_opt(stmt, &[]).await?.map(Proposal::from_row).transpose()?)
    }

    pub async fn mark_invalid(&self, client: &Client) -> Result<(), DBError> {
        self.update(
            UpdateProposal {
                status: Some(ProposalStatus::Invalid),
                ..UpdateProposal::default()
            },
            &client,
        )
        .await?;

        Ok(())
    }

    pub async fn insert(params: NewProposal, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            INSERT INTO proposals (
                id,
                new_view,
                asset_id,
                node_id
            ) VALUES ($1, $2, $3, $4) RETURNING *";
        let stmt = client.prepare(QUERY).await?;
        let row = client
            .query_one(&stmt, &[
                &params.id,
                &params.new_view,
                &params.asset_id,
                &params.node_id,
            ])
            .await?;
        Ok(Self::from_row(row)?)
    }

    /// Update proposal state in the database
    ///
    /// Updates subset of fields:
    /// - status
    pub async fn update(&self, data: UpdateProposal, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            UPDATE proposals SET
                status = COALESCE($1, status),
                updated_at = NOW()
            WHERE id = $2::\"ProposalID\"
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::TEXT]).await?;
        let updated = client.query_one(&stmt, &[&data.status, &self.id]).await?;
        Ok(Self::from_row(updated)?)
    }

    /// Load proposal from database by ID
    pub async fn load(id: ProposalID, client: &Client) -> Result<Self, DBError> {
        let stmt = "SELECT * FROM proposals WHERE id = $1::\"ProposalID\"";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Self::from_row(result)?)
    }

    /// Creates partial signature
    pub async fn create_partial_signature(&self) -> Result<String, DBError> {
        Ok("stub-signature".to_string())
    }

    /// Signs the proposal
    pub async fn sign(&self, node_id: NodeID, client: &Client) -> Result<SignedProposal, DBError> {
        let params = NewSignedProposal {
            node_id,
            proposal_id: self.id,
            signature: "stub-signature".to_string(),
        };
        self.update(
            UpdateProposal {
                status: Some(ProposalStatus::Signed),
                ..UpdateProposal::default()
            },
            &client,
        )
        .await?;

        Ok(SignedProposal::insert(params, &client).await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::AssetState,
        test::utils::{
            builders::consensus::{ProposalBuilder, ViewBuilder},
            test_db_client,
        },
    };

    #[actix_rt::test]
    async fn find_pending() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let proposal2 = ProposalBuilder::default().build(&client).await.unwrap();
        let proposal3 = ProposalBuilder::default().build(&client).await.unwrap();

        // proposal is ignored if an existing block is present
        let mut asset_state = AssetState::find_by_asset_id(&proposal.asset_id, &client)
            .await
            .unwrap()
            .unwrap();
        asset_state.acquire_lock(60 as u64, &client).await.unwrap();

        // proposal3 is ignored as it is not pending
        proposal3
            .update(
                UpdateProposal {
                    status: Some(ProposalStatus::Signed),
                    ..UpdateProposal::default()
                },
                &client,
            )
            .await
            .unwrap();

        let proposals = Proposal::find_pending(&client).await.unwrap();
        assert_eq!(proposals, Some(proposal2));
    }

    #[actix_rt::test]
    async fn create_partial_signature() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        assert_eq!(
            proposal.create_partial_signature().await.unwrap(),
            "stub-signature".to_string()
        );
    }

    #[actix_rt::test]
    async fn mark_invalid() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        proposal.mark_invalid(&client).await.unwrap();

        let proposal = Proposal::load(proposal.id, &client).await.unwrap();
        assert_eq!(proposal.status, ProposalStatus::Invalid);
    }

    #[actix_rt::test]
    async fn sign() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let signed_proposal = proposal.sign(NodeID::stub(), &client).await.unwrap();

        assert_eq!(signed_proposal.proposal_id, proposal.id);
    }

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let id = ProposalID::new(NodeID::stub()).await.unwrap();

        let new_view = ViewBuilder::default().prepare(&client).await.unwrap();
        let params = NewProposal {
            id,
            node_id: NodeID::stub(),
            asset_id: new_view.asset_id.clone(),
            new_view: new_view.clone(),
        };
        let proposal = Proposal::insert(params, &client).await.unwrap();
        assert_eq!(proposal.id, id);
        assert_eq!(proposal.new_view, new_view);

        proposal
            .update(
                UpdateProposal {
                    status: Some(ProposalStatus::Signed),
                    ..UpdateProposal::default()
                },
                &client,
            )
            .await
            .unwrap();

        let proposal = Proposal::load(proposal.id, &client).await.unwrap();
        assert_eq!(proposal.status, ProposalStatus::Signed);
    }
}
