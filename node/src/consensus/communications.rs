use super::errors::ConsensusError;
use crate::{
    consensus::ConsensusCommittee,
    db::models::consensus::{AggregateSignatureMessage, NewView, Proposal, SignedProposal},
};

// TODO: these stubbed methods just exists to flesh out the consensus worker logic
//       we will need to further iterate as we hook in the tari comms layer / flesh out node communication

pub async fn submit_new_view(_committee: ConsensusCommittee, _new_view: NewView) -> Result<(), ConsensusError> {
    Ok(())
}

pub async fn broadcast_proposal(_committee: ConsensusCommittee, _proposal: Proposal) -> Result<(), ConsensusError> {
    Ok(())
}

pub async fn submit_signed_proposal(
    _committee: ConsensusCommittee,
    _signed_proposal: SignedProposal,
) -> Result<(), ConsensusError>
{
    Ok(())
}

pub async fn broadcast_aggregate_signature_message(
    _committee: ConsensusCommittee,
    _aggregate_signature_message: NewAggregateSignatureMessage,
) -> Result<(), ConsensusError>
{
    Ok(())
}

pub async fn submit_partial_signature(
    _committee: ConsensusCommittee,
    _signature: String,
) -> Result<(), ConsensusError>
{
    Ok(())
}
