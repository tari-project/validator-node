use crate::db::models::consensus::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, PartialEq, Debug, Deserialize)]
pub enum CommitteeState {
    PreparingView {
        pending_instructions: Vec<Instruction>,
    },
    ViewThresholdReached {
        views: Vec<View>,
    },
    ReceivedLeaderProposal {
        proposal: Proposal,
    },
    SignedProposalThresholdReached {
        proposal: Proposal,
        signed_proposals: Vec<SignedProposal>,
    },
    LeaderFinalizedProposalReceived {
        proposal: Proposal,
        aggregate_signature_message: AggregateSignatureMessage,
    },
}
