use crate::{
    db::{
        models::{consensus::*, AssetState, InstructionStatus, ProposalStatus, Token, ViewStatus},
        utils::errors::DBError,
    },
    types::{AssetID, InstructionID, NodeID, ProposalID},
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

    /// Execute the proposal applying append only state to the database
    pub async fn execute(self, leader: bool, client: &Client) -> Result<(), DBError> {
        let view = if leader {
            // Find pending view for asset, switch to commit
            let asset_id = self.new_view.asset_id.clone();
            let found_view = View::find_by_asset_status(&asset_id, ViewStatus::PreCommit, &client)
                .await?
                .first()
                .map(|v| v.clone())
                .ok_or_else(|| DBError::NotFound)?;

            found_view
                .update(
                    UpdateView {
                        status: Some(ViewStatus::Commit),
                        proposal_id: Some(self.id),
                        ..UpdateView::default()
                    },
                    &client,
                )
                .await?
        } else {
            View::insert(
                self.new_view.clone(),
                NewViewAdditionalParameters {
                    status: Some(ViewStatus::Commit),
                    proposal_id: Some(self.id),
                },
                &client,
            )
            .await?
        };

        for asset_state_append_only in &*view.append_only_state.asset_state {
            AssetState::store_append_only_state(&asset_state_append_only, &client).await?;
        }

        for token_state_append_only in &*view.append_only_state.token_state {
            Token::store_append_only_state(&token_state_append_only, &client).await?;
        }

        self.update(
            UpdateProposal {
                status: Some(ProposalStatus::Finalized),
                ..UpdateProposal::default()
            },
            &client,
        )
        .await?;

        let instruction_set: Vec<InstructionID> = view.instruction_set.iter().map(|i| InstructionID(*i)).collect();
        let invalid_instruction_set: Vec<InstructionID> =
            view.invalid_instruction_set.iter().map(|i| InstructionID(*i)).collect();

        Instruction::update_instructions_status(&instruction_set, Some(self.id), InstructionStatus::Commit, &client)
            .await?;
        Instruction::update_instructions_status(
            &invalid_instruction_set,
            Some(self.id),
            InstructionStatus::Invalid,
            &client,
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::{AssetStatus, NewAssetStateAppendOnly, NewTokenStateAppendOnly, TokenStatus},
        test::utils::{
            builders::{
                consensus::{InstructionBuilder, ProposalBuilder, ViewBuilder},
                TokenBuilder,
            },
            test_db_client,
        },
        types::consensus::AppendOnlyState,
    };
    use serde_json::json;

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
    async fn execute() {
        let (client, _lock) = test_db_client().await;
        let mut proposal = ProposalBuilder::default().build(&client).await.unwrap();

        let token = TokenBuilder::default().build(&client).await.unwrap();
        let asset = AssetState::load(token.asset_state_id, &client).await.unwrap();
        let instruction = InstructionBuilder {
            asset_id: Some(asset.asset_id.clone()),
            token_id: Some(token.token_id.clone()),
            ..InstructionBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();

        proposal.new_view.instruction_set = vec![instruction.id.0];
        proposal.new_view.append_only_state = AppendOnlyState {
            asset_state: vec![NewAssetStateAppendOnly {
                asset_id: asset.asset_id.clone(),
                instruction_id: instruction.id,
                status: AssetStatus::Active,
                state_data_json: json!({"asset-value": true, "asset-value2": 1}),
            }],
            token_state: vec![NewTokenStateAppendOnly {
                token_id: token.token_id,
                instruction_id: instruction.id,
                status: TokenStatus::Active,
                state_data_json: json!({"token-value": true, "token-value2": 1}),
            }],
        };

        // Execute as non leader triggering new view commit along with persistence of append only data
        let proposal_id = proposal.id.clone();
        proposal.execute(false, &client).await.unwrap();

        let asset = AssetState::load(token.asset_state_id, &client).await.unwrap();
        assert_eq!(
            asset.additional_data_json,
            json!({"asset-value": true, "asset-value2": 1})
        );
        let token = Token::load(token.id, &client).await.unwrap();
        assert_eq!(
            token.additional_data_json,
            json!({"token-value": true, "token-value2": 1})
        );
        let proposal = Proposal::load(proposal_id, &client).await.unwrap();
        assert_eq!(proposal.status, ProposalStatus::Finalized);
        let view = View::load_for_proposal(proposal.id, &client).await.unwrap();
        assert_eq!(view.status, ViewStatus::Commit);
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
