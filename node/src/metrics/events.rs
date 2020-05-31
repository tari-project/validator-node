//! Metrics are built based on Events. Events are [Metrics] actor's messages.
//!
//! ```
//! let event: Event = ContractCallEvent {
//!     contract_name: "my_contract".into(),
//! }
//! .into();
//! // tx.send(event)
//! ```

use crate::{db::models::InstructionStatus, types::InstructionID};
use actix::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize, Clone)]
#[rtype(result = "()")]
/// Send events to Metrics actor to display metrics in terminal UI
// TODO: For simplicity Event's time is recorded once it reaces actor, in a proper
// timeseries event should contain timestamp. As this is for demo purpose only
// this solutino is ok, though it might provide not exact data under heavy load
pub enum MetricEvent {
    Call(ContractCallEvent),
    Instruction(InstructionEvent),
}

/// Contract initiated via HTTP
#[derive(Serialize, Deserialize, Clone)]
pub struct ContractCallEvent {
    pub contract_name: String,
}

impl From<ContractCallEvent> for MetricEvent {
    fn from(req: ContractCallEvent) -> Self {
        Self::Call(req)
    }
}

/// Instruction created or changed it's status
#[derive(Serialize, Deserialize, Clone)]
pub struct InstructionEvent {
    pub id: InstructionID,
    pub status: InstructionStatus,
}

impl From<InstructionEvent> for MetricEvent {
    fn from(req: InstructionEvent) -> Self {
        Self::Instruction(req)
    }
}
