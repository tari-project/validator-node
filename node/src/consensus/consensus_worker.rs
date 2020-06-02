use super::{communications::*, errors::ConsensusError, ConsensusCommittee};
use crate::{
    config::NodeConfig,
    consensus::{instruction_state, instruction_state::InstructionTransitionContext, LOG_TARGET},
    db::{
        models::{consensus::*, AssetState, ProposalStatus, Token, ViewStatus},
        utils::{db::db_client, errors::DBError},
    },
    metrics::Metrics,
    types::{consensus::CommitteeState, InstructionID, NodeID},
};

use actix::Addr;
use deadpool_postgres::Client;
use log::{error, warn};

pub struct ConsensusWorker {
    node_config: NodeConfig,
    metrics_addr: Option<Addr<Metrics>>,
}

impl ConsensusWorker {
    pub fn new(node_config: NodeConfig, metrics_addr: Option<Addr<Metrics>>) -> Result<Self, ConsensusError> {
        Ok(ConsensusWorker {
            node_config,
            metrics_addr,
        })
    }

    pub async fn work(&self, node_id: NodeID) -> Result<(), ConsensusError> {
        let config = self.node_config.clone();
        let metrics_address = self.metrics_addr.clone();
        let client = db_client(&config)
            .await
            .expect("Validator node unable to load db client");
        actix_rt::spawn(async move {
            if let Err(e) = ConsensusWorker::task(node_id, metrics_address, &client).await {
                error!("ConsensusWorker work error: {}", e)
            };
        });

        Ok(())
    }

    pub(crate) async fn execute_proposal(
        proposal: Proposal,
        leader: bool,
        metrics_addr: Option<Addr<Metrics>>,
        client: &Client,
    ) -> Result<(), ConsensusError>
    {
        let view = if leader {
            // Find pending view for asset, switch to commit
            let asset_id = proposal.new_view.asset_id.clone();
            let found_view = View::find_by_asset_status(&asset_id, ViewStatus::PreCommit, &client)
                .await?
                .first()
                .map(|v| v.clone())
                .ok_or_else(|| DBError::NotFound)?;

            found_view
                .update(
                    UpdateView {
                        status: Some(ViewStatus::Commit),
                        proposal_id: Some(proposal.id),
                        ..UpdateView::default()
                    },
                    &client,
                )
                .await?
        } else {
            View::insert(
                proposal.new_view.clone(),
                NewViewAdditionalParameters {
                    status: Some(ViewStatus::Commit),
                    proposal_id: Some(proposal.id),
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

        let proposal = proposal
            .update(
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

        instruction_state::transition(
            InstructionTransitionContext {
                template_id: proposal.asset_id.template_id(),
                instruction_ids: instruction_set,
                proposal_id: Some(proposal.id),
                current_status: InstructionStatus::Pending,
                status: InstructionStatus::Commit,
                result: None,
                metrics_addr: metrics_addr.clone(),
            },
            &client,
        )
        .await?;

        instruction_state::transition(
            InstructionTransitionContext {
                template_id: proposal.asset_id.template_id(),
                instruction_ids: invalid_instruction_set,
                proposal_id: Some(proposal.id),
                current_status: InstructionStatus::Pending,
                status: InstructionStatus::Invalid,
                result: None,
                metrics_addr: metrics_addr.clone(),
            },
            &client,
        )
        .await?;

        Ok(())
    }

    async fn task(
        node_id: NodeID,
        metrics_addr: Option<Addr<Metrics>>,
        client: &Client,
    ) -> Result<bool, ConsensusError>
    {
        let committee = ConsensusCommittee::find_next_pending_committee(node_id, &client).await?;
        match committee {
            Some(committee) => {
                match &mut committee.acquire_lock(60 as u64, &client).await {
                    Ok(_) => {
                        match committee.state.clone() {
                            // All nodes prepare new view, all but leader send to the leader node
                            CommitteeState::PreparingView { pending_instructions } => {
                                let new_view = committee
                                    .prepare_new_view(node_id, &pending_instructions, &client)
                                    .await?;
                                if !committee.is_leader(node_id) {
                                    submit_new_view(&committee, &new_view).await?;
                                }
                            },
                            // Leader listens for view threshold being reached
                            CommitteeState::ViewThresholdReached { mut views } => {
                                let proposal = committee.create_proposal(node_id, &mut views, &client).await?;
                                broadcast_proposal(&committee, &proposal).await?;
                            },
                            // All but leader receive proposal, confirm instruction set, and sign proposal if accepted
                            CommitteeState::ReceivedLeaderProposal { proposal } => {
                                if committee.confirm_proposal(&proposal).await? {
                                    let signed_proposal = proposal.sign(node_id, &client).await?;
                                    submit_signed_proposal(&committee, &signed_proposal).await?;
                                } else {
                                    warn!(
                                        target: LOG_TARGET,
                                        "Committee proposal failed consensus, asset_id: {}", committee.asset_id
                                    );
                                }
                            },
                            // Leader has supermajority threshold met for signatures, prepare aggregate signature and
                            // send to other nodes
                            CommitteeState::SignedProposalThresholdReached {
                                proposal,
                                signed_proposals,
                            } => {
                                let aggregate_signature_message = committee
                                    .prepare_aggregate_signature_message(&proposal, &signed_proposals, &client)
                                    .await?;
                                broadcast_aggregate_signature_message(&committee, &aggregate_signature_message).await?;

                                // Execute proposal for leader (other nodes will receive signed proposal and execute
                                // upon validating supermajority signatures)
                                ConsensusWorker::execute_proposal(proposal, true, metrics_addr, &client).await?;
                            },
                            // Leader finalized proposal received, nodes confirm signatures, and apply state.
                            CommitteeState::LeaderFinalizedProposalReceived {
                                proposal,
                                aggregate_signature_message,
                            } => {
                                aggregate_signature_message.validate(&client).await?;

                                // Execute proposal for non leader nodes
                                ConsensusWorker::execute_proposal(proposal, false, metrics_addr, &client).await?;
                            },
                        }

                        committee.release_lock(&client).await?;
                    },
                    _ => {
                        // Failed to acquire lock
                        return Ok(false);
                    },
                }

                Ok(true)
            },
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::{
            consensus::{AggregateSignatureMessage, Instruction, Proposal, SignedProposal, View},
            AssetStatus,
            NewAssetStateAppendOnly,
            NewTokenStateAppendOnly,
            TokenStatus,
            *,
        },
        test::utils::{
            builders::{
                consensus::{
                    AggregateSignatureMessageBuilder,
                    InstructionBuilder,
                    ProposalBuilder,
                    SignedProposalBuilder,
                    ViewBuilder,
                },
                TokenBuilder,
            },
            test_db_client,
        },
        types::consensus::AppendOnlyState,
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn execute_proposal() {
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
        ConsensusWorker::execute_proposal(proposal, false, None, &client)
            .await
            .unwrap();

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
    async fn task_preparing_view() {
        let (client, _lock) = test_db_client().await;
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        assert!(ConsensusWorker::task(NodeID::stub(), None, &client).await.unwrap());

        let view_response = View::threshold_met(&client).await.unwrap();
        let (_, views) = view_response.iter().next().unwrap();
        assert_eq!(views.len(), 1);
        let view = &views[0];
        assert_eq!(view.instruction_set, vec![instruction.id.0]);

        let instruction = Instruction::load(instruction.id, &client).await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Pending);
    }

    #[actix_rt::test]
    async fn task_view_threshold_reached() {
        let (client, _lock) = test_db_client().await;
        let view = ViewBuilder::default().build(&client).await.unwrap();
        assert!(ConsensusWorker::task(NodeID::stub(), None, &client).await.unwrap());

        // Leader signs proposal immediately so fetch proposal through signed proposal pending
        let signed_proposal_data = SignedProposal::threshold_met(&client).await.unwrap();
        let (_, signed_proposals) = signed_proposal_data.iter().next().unwrap();
        let signed_proposal = &signed_proposals[0];

        let proposal = Proposal::load(signed_proposal.proposal_id, &client).await.unwrap();
        assert_eq!(proposal.status, ProposalStatus::Signed);
        assert_eq!(proposal.new_view, view.into());
    }

    #[actix_rt::test]
    async fn task_received_leader_proposal() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        assert!(ConsensusWorker::task(NodeID::stub(), None, &client).await.unwrap());

        let signed_proposal_data = SignedProposal::threshold_met(&client).await.unwrap();
        let (_, signed_proposals) = signed_proposal_data.iter().next().unwrap();
        let signed_proposal = &signed_proposals[0];
        assert_eq!(signed_proposal.status, SignedProposalStatus::Pending);
        assert_eq!(signed_proposal.proposal_id, proposal.id);

        let proposal = Proposal::load(proposal.id, &client).await.unwrap();
        assert_eq!(proposal.status, ProposalStatus::Signed);
    }

    #[actix_rt::test]
    async fn task_signed_proposal_threshold_reached() {
        let (client, _lock) = test_db_client().await;
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        let view = ViewBuilder {
            status: Some(ViewStatus::PreCommit),
            instruction_set: vec![instruction.id.0],
            ..ViewBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        let proposal = ProposalBuilder {
            new_view: Some(view.clone().into()),
            ..ProposalBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        let signed_proposal = SignedProposalBuilder {
            proposal_id: Some(proposal.id),
            ..SignedProposalBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        assert!(ConsensusWorker::task(NodeID::stub(), None, &client).await.unwrap());

        let aggregate_signature_messages = AggregateSignatureMessage::load_by_proposal_id(proposal.id, &client)
            .await
            .unwrap();
        assert_eq!(
            aggregate_signature_messages[0].status,
            AggregateSignatureMessageStatus::Accepted
        );

        let signed_proposal = SignedProposal::load(signed_proposal.id, &client).await.unwrap();
        assert_eq!(signed_proposal.status, SignedProposalStatus::Validated);
        let view = View::load(view.id, &client).await.unwrap();
        assert_eq!(view.status, ViewStatus::Commit);
        let instruction = Instruction::load(instruction.id, &client).await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Commit);
    }

    #[actix_rt::test]
    async fn task_leader_finalized_proposal_received() {
        let (client, _lock) = test_db_client().await;
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        let view = ViewBuilder {
            instruction_set: vec![instruction.id.0],
            ..ViewBuilder::default()
        }
        .prepare(&client)
        .await
        .unwrap();
        let proposal = ProposalBuilder {
            new_view: Some(view.clone()),
            ..ProposalBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        let aggregate_signature_message = AggregateSignatureMessageBuilder {
            proposal_id: Some(proposal.id),
            ..AggregateSignatureMessageBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        assert!(ConsensusWorker::task(NodeID::stub(), None, &client).await.unwrap());

        let aggregate_signature_message = AggregateSignatureMessage::load(aggregate_signature_message.id, &client)
            .await
            .unwrap();
        assert_eq!(
            aggregate_signature_message.status,
            AggregateSignatureMessageStatus::Accepted
        );
        let view = View::load_for_proposal(proposal.id, &client).await.unwrap();
        assert_eq!(view.status, ViewStatus::Commit);
        let instruction = Instruction::load(instruction.id, &client).await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Commit);
    }
}
