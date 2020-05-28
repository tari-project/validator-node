use super::ConsensusWorker;
use crate::{config::NodeConfig, consensus::LOG_TARGET, types::NodeID};
use log::{error, info};
use std::{sync::mpsc::Receiver, time::Duration};
use tokio::time::delay_for;

pub struct ConsensusProcessor {
    node_config: NodeConfig,
    node_id: NodeID,
}

impl ConsensusProcessor {
    pub fn new(node_config: NodeConfig) -> Self {
        Self {
            node_config: node_config.clone(),
            node_id: NodeID::stub(),
        }
    }

    pub async fn start(&mut self, kill_receiver: Receiver<()>) {
        info!(target: LOG_TARGET, "Starting consensus processor");
        let interval = self.node_config.consensus.poll_period as u64;
        let consensus_worker = ConsensusWorker::new(self.node_config.clone()).unwrap();

        loop {
            if kill_receiver.try_recv().is_ok() {
                info!(target: LOG_TARGET, "Stopping consensus processor");
                break;
            }
            // Poll for any updates to consensus state
            if let Err(e) = consensus_worker.work(self.node_id).await {
                error!(target: LOG_TARGET, "Consensus error: {}", e);
            };

            delay_for(Duration::from_secs(interval)).await;
        }
    }
}
