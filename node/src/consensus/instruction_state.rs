use super::errors::ConsensusError;
use crate::{
    db::models::{consensus::Instruction, InstructionStatus},
    metrics::{
        events::{InstructionEvent, MetricEvent},
        metrics::Metrics,
    },
    types::*,
};
use actix::Addr;
use deadpool_postgres::Client;
use serde_json::Value;

const LOG_TARGET: &'static str = "tari_validator_node::consensus";

pub struct InstructionTransitionContext {
    pub template_id: TemplateID,
    pub instruction_ids: Vec<InstructionID>,
    pub proposal_id: Option<ProposalID>,
    pub current_status: InstructionStatus,
    pub status: InstructionStatus,
    pub result: Option<Value>,
    pub metrics_addr: Option<Addr<Metrics>>,
}

impl InstructionTransitionContext {
    /// Update [Metrics] Actor (if configured) with instruction update
    fn metrics_update(&self) {
        if let Some(metrics_addr) = self.metrics_addr.as_ref() {
            for instruction_id in &self.instruction_ids {
                let msg: MetricEvent = InstructionEvent {
                    id: instruction_id.clone(),
                    status: self.status,
                }
                .into();
                metrics_addr.do_send(msg);
            }
        }
    }
}

pub async fn transition(context: InstructionTransitionContext, client: &Client) -> Result<(), ConsensusError> {
    log::trace!(
        target: LOG_TARGET,
        "template={}, instructions={:?}",
        context.template_id,
        context.instruction_ids
    );

    // Valid state transitions
    match (context.current_status, context.status) {
        (InstructionStatus::Scheduled, InstructionStatus::Processing) |
        (InstructionStatus::Processing, InstructionStatus::Pending) |
        (InstructionStatus::Processing, InstructionStatus::Invalid) |
        (InstructionStatus::Pending, InstructionStatus::Invalid) |
        (InstructionStatus::Pending, InstructionStatus::Commit) => {},
        (a, b) => {
            return Err(ConsensusError::error(&format!(
                "Invalid Instruction {:?} status {} transition {:?}",
                context.instruction_ids, a, b
            )));
        },
    }

    Instruction::update_instructions_status(
        &context.instruction_ids,
        context.proposal_id,
        context.status,
        context.result.to_owned(),
        &client,
    )
    .await?;
    context.metrics_update();
    Ok(())
}
