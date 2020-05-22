use super::{errors::ConsensusError, ConsensusWorker};
use crate::{config::NodeConfig, consensus::LOG_TARGET, types::NodeID};
use log::info;
use std::{future::Future, pin::Pin, sync::mpsc::Receiver, time::Duration};
use tokio::time::delay_for;

pub struct ConsensusProcessor {
    node_config: NodeConfig,
    processor: Option<Pin<Box<dyn Future<Output = Result<(), ConsensusError>> + Send>>>,
    node_id: NodeID,
}

impl ConsensusProcessor {
    pub fn new(node_config: NodeConfig) -> Self {
        Self {
            node_config: node_config.clone(),
            node_id: NodeID::stub(),
            processor: None,
        }
    }

    pub async fn process(
        node_config: NodeConfig,
        node_id: NodeID,
        consensus_receiver: Receiver<()>,
    ) -> Result<(), ConsensusError>
    {
        info!(target: LOG_TARGET, "Starting consensus processor");
        let interval = node_config.consensus.poll_period as u64;
        let consensus_worker = ConsensusWorker::new(node_config)?;
        loop {
            if consensus_receiver.try_recv().is_ok() {
                info!(target: LOG_TARGET, "Stopping consensus processor");
                break;
            }
            // Poll for any updates to consensus state
            consensus_worker.work(node_id)?;

            delay_for(Duration::from_secs(interval)).await;
        }
        Ok(())
    }

    pub async fn start(&mut self, consensus_receiver: Receiver<()>) {
        self.processor = Some(Box::pin(ConsensusProcessor::process(
            self.node_config.clone(),
            self.node_id,
            consensus_receiver,
        )));
    }
}
