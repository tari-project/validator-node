use super::{communications::*, errors::ConsensusError, ConsensusCommittee};
use crate::{
    config::NodeConfig,
    consensus::LOG_TARGET,
    db::utils::db::db_client,
    types::{consensus::CommitteeState, NodeID},
};
use log::warn;
use tokio::runtime;

pub struct ConsensusWorker {
    node_config: NodeConfig,
    tokio_runtime: runtime::Runtime,
}

impl ConsensusWorker {
    pub fn new(node_config: NodeConfig) -> Result<Self, ConsensusError> {
        let mut tokio_runtime = runtime::Builder::new();

        if let Some(worker_count) = node_config.consensus.workers {
            tokio_runtime.core_threads(worker_count);
        }

        Ok(ConsensusWorker {
            node_config,
            tokio_runtime: tokio_runtime.threaded_scheduler().build()?,
        })
    }

    pub fn work(&self, node_id: NodeID) -> Result<(), ConsensusError> {
        self.tokio_runtime
            .spawn(ConsensusWorker::task(node_id, self.node_config.clone()));

        Ok(())
    }

    async fn task(node_id: NodeID, node_config: NodeConfig) -> Result<bool, ConsensusError> {
        let client = db_client(&node_config)
            .await
            .expect("Validator node unable to load db client");

        let committee = ConsensusCommittee::find_next_pending_committee(node_id, &client).await?;
        match committee {
            Some(committee) => {
                match &mut committee.acquire_lock(60, &client).await {
                    Ok(_) => {
                        match committee.state.clone() {
                            // All nodes prepare new view, all but leader send to the leader node
                            CommitteeState::PreparingView { pending_instructions } => {
                                let new_view = committee.prepare_new_view(&pending_instructions, &client).await?;
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
                                    .prepare_aggregate_signature_message(&proposal, &signed_proposals)
                                    .await?;
                                broadcast_aggregate_signature_message(&committee, &aggregate_signature_message).await?;

                                // Save aggregate message
                                aggregate_signature_message.save(&client).await?;
                                // Execute proposal for leader (other nodes will receive signed proposal and execute
                                // upon validating supermajority signatures)
                                proposal.execute(true, &client).await?;
                            },
                            // Leader finalized proposal received, nodes confirm signatures, and apply state.
                            CommitteeState::LeaderFinalizedProposalReceived {
                                proposal,
                                aggregate_signature_message,
                            } => {
                                aggregate_signature_message.validate().await?;

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
