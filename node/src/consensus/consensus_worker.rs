use super::{communications::*, errors::ConsensusError, ConsensusCommittee};
use crate::{
    config::NodeConfig,
    consensus::LOG_TARGET,
    db::utils::db::db_client,
    types::{consensus::CommitteeState, NodeID},
};
use deadpool_postgres::Client;
use log::{error, warn};

pub struct ConsensusWorker {
    node_config: NodeConfig,
}

impl ConsensusWorker {
    pub fn new(node_config: NodeConfig) -> Result<Self, ConsensusError> {
        Ok(ConsensusWorker { node_config })
    }

    pub async fn work(&self, node_id: NodeID) -> Result<(), ConsensusError> {
        let config = self.node_config.clone();
        let client = db_client(&config)
            .await
            .expect("Validator node unable to load db client");
        actix_rt::spawn(async move {
            if let Err(e) = ConsensusWorker::task(node_id, &client).await {
                error!("ConsensusWorker work error: {}", e)
            };
        });

        Ok(())
    }

    async fn task(node_id: NodeID, client: &Client) -> Result<bool, ConsensusError> {
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
                                proposal.execute(true, &client).await?;
                            },
                            // Leader finalized proposal received, nodes confirm signatures, and apply state.
                            CommitteeState::LeaderFinalizedProposalReceived {
                                proposal,
                                aggregate_signature_message,
                            } => {
                                aggregate_signature_message.validate(&client).await?;

                                // Execute proposal for non leader nodes
                                proposal.execute(false, &client).await?;
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
            *,
        },
        test::utils::{
            builders::consensus::{
                AggregateSignatureMessageBuilder,
                InstructionBuilder,
                ProposalBuilder,
                SignedProposalBuilder,
                ViewBuilder,
            },
            test_db_client,
        },
    };

    #[actix_rt::test]
    async fn task_preparing_view() {
        let (client, _lock) = test_db_client().await;
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        assert!(ConsensusWorker::task(NodeID::stub(), &client).await.unwrap());

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
        assert!(ConsensusWorker::task(NodeID::stub(), &client).await.unwrap());

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
        assert!(ConsensusWorker::task(NodeID::stub(), &client).await.unwrap());

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
        assert!(ConsensusWorker::task(NodeID::stub(), &client).await.unwrap());

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
        assert!(ConsensusWorker::task(NodeID::stub(), &client).await.unwrap());

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
