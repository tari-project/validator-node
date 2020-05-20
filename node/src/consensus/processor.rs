use super::{communications::*, errors::ConsensusError, ConsensusCommittee};
use crate::{
    config::NodeConfig,
    consensus::LOG_TARGET,
    db::utils::db::db_client,
    types::{consensus::CommitteeState, NodeID},
};
use deadpool_postgres::Client;
use log::{error, info, warn};
use std::{
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
    },
    thread,
    thread::JoinHandle,
    time::Duration,
};

pub struct ConsensusProcessor {
    node_config: NodeConfig,
    worker_threads: Vec<(Sender<()>, JoinHandle<Result<(), ConsensusError>>)>,
    interval: u64,
}

impl ConsensusProcessor {
    pub fn new(node_config: NodeConfig, poll_period_in_secs: u64) -> ConsensusProcessor {
        ConsensusProcessor {
            node_config: node_config.clone(),
            worker_threads: Vec::new(),
            interval: poll_period_in_secs,
        }
    }

    pub async fn find_consensus_events_to_handle(client: &Client) -> Result<bool, ConsensusError> {
        let committee = ConsensusCommittee::find_next_pending_committee(NodeID::stub().inner(), client).await?;

        match committee {
            Some(committee) => {
                match &mut committee.acquire_lock(60, &client).await {
                    Ok(_) => {
                        match committee.state {
                            // All nodes prepare new view, all but leader send to the leader node
                            CommitteeState::PreparingView { pending_instructions } => {
                                let new_view = committee.prepare_new_view(pending_instructions, &client).await?;

                                // TODO: replace node ID stub with the node ID of current node
                                if !committee.is_leader(NodeID::stub()) {
                                    submit_new_view(committee, new_view).await?;
                                }
                            },
                            // Leader listens for view threshold being reached
                            CommitteeState::ViewThresholdReached { views } => {
                                let proposal = committee.create_proposal(views, &client).await?;
                                broadcast_proposal(committee, proposal).await?;
                            },
                            // All but leader receive proposal, confirm instruction set, and sign proposal if accepted
                            CommitteeState::ReceivedLeaderProposal { proposal } => {
                                if committee.confirm_proposal(proposal).await? {
                                    let signed_proposal = proposal.sign(&client).await?;
                                    submit_signed_proposal(committee, signed_proposal).await?;
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
                                    .prepare_aggregate_signature_message(proposal, signed_proposals)
                                    .await?;
                                broadcast_aggregate_signature_message(committee, aggregate_signature_message).await?;

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
                                // TODO: validate signatures

                                // Execute proposal for non leader nodes
                                proposal.execute(false, &client).await?;
                            },
                        }

                        committee.release_lock(client).await?;
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

    pub async fn process(client: &Client, interval: u64, rx: Receiver<()>) -> Result<(), ConsensusError> {
        loop {
            if rx.try_recv().is_ok() {
                info!(target: LOG_TARGET, "Stopping consensus processor");
                break;
            }

            if ConsensusProcessor::find_consensus_events_to_handle(client).await? {
                thread::sleep(Duration::from_secs(interval));
            }
        }
        Ok(())
    }

    pub async fn start(&mut self, run_actions: bool, run_events: bool) {
        // Use this channel to tell the server to shut down
        let (consensus_tx, consensus_rx) = mpsc::channel::<()>();
        let stop_signals = vec![consensus_tx.clone()];
        let client = db_client(&self.node_config).await?;

        // Create a worker thread to handle consensus process
        self.worker_threads.push((
            consensus_tx,
            thread::spawn(move || {
                let res = ConsensusProcessor::process(client, self.interval, consensus_rx).await.map_err(|e| {
                    error!(target: LOG_TARGET, "Validator node failed during consensus: {}", e);
                    e
                });

                for signal in stop_signals {
                    match signal.send(()) {
                        Ok(_) => (),
                        Err(_) => (),
                    }
                }

                res
            }),
        ));
    }

    pub fn stop(&mut self) {
        for w in self.worker_threads.drain(..) {
            w.0.send(()).unwrap();
            w.1.join().unwrap().unwrap();
        }
    }
}
