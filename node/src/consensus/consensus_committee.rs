use super::errors::ConsensusError;
use crate::{
    db::models::{consensus::*, AssetState, ViewStatus},
    types::{consensus::*, AssetID, NodeID, ProposalID},
};
use deadpool_postgres::Client;
use uuid::Uuid;

pub struct ConsensusCommittee {
    pub state: CommitteeState,
    pub asset_id: AssetID,
    pub leader_node_id: NodeID,
}

impl ConsensusCommittee {
    /// Returns next pending committee data for the purposes of the consensus state processing
    /// TODO: This is currently hardcoded for a committee of 1
    ///       We will need further build this out as we expand into real committees / just a stub
    pub async fn find_next_pending_committee(
        node_id: NodeID,
        client: &Client,
    ) -> Result<Option<ConsensusCommittee>, ConsensusError>
    {
        // Note: asset_states includes a blocked_until field which prevents any activity on the asset
        //       This is to ensure only one worker at a time processes the current state
        // Note: Logic favors handling an in progress consensus step prior to processing new instructions

        // Find any pending signature messages indicating a state is pending finalization
        if let Some(aggregate_signature_message) = AggregateSignatureMessage::find_pending(&client).await? {
            let proposal = aggregate_signature_message.proposal(&client).await?;
            let leader_node_id = ConsensusCommittee::determine_leader_node_id(&proposal.asset_id).await?;

            return Ok(Some(ConsensusCommittee {
                leader_node_id,
                asset_id: proposal.asset_id.clone(),
                state: CommitteeState::LeaderFinalizedProposalReceived {
                    proposal,
                    aggregate_signature_message,
                },
            }));
        }

        // Find any mappings of asset id to signed proposals where the threshold is met
        // This node must the current leader to accept these signed proposals or they are thrown out
        // Only the first valid asset ID where the current node is the leader is returned
        let asset_id_signed_proposal_mapping = SignedProposal::threshold_met(&client).await?;
        for (asset_id, signed_proposals) in asset_id_signed_proposal_mapping {
            let leader_node_id = ConsensusCommittee::determine_leader_node_id(&asset_id).await?;
            let proposal_id = signed_proposals[0].proposal_id;
            let proposal = Proposal::load(proposal_id, &client).await?;

            if leader_node_id == node_id {
                return Ok(Some(ConsensusCommittee {
                    asset_id,
                    leader_node_id,
                    state: CommitteeState::SignedProposalThresholdReached {
                        proposal,
                        signed_proposals,
                    },
                }));
            } else {
                // We are not the leader, invalidate pending signed proposals
                SignedProposal::invalidate(signed_proposals, &client).await?;
            }
        }

        // Find any pending proposal
        if let Some(proposal) = Proposal::find_pending(&client).await? {
            let leader_node_id = ConsensusCommittee::determine_leader_node_id(&proposal.asset_id).await?;

            if proposal.node_id == leader_node_id {
                return Ok(Some(ConsensusCommittee {
                    leader_node_id,
                    asset_id: proposal.asset_id.clone(),
                    state: CommitteeState::ReceivedLeaderProposal { proposal },
                }));
            } else {
                // This proposal came from a node not currently viewed as the leader, mark it invalid
                proposal.mark_invalid(&client).await?
            }
        }

        // Find any mappings of asset id to new views where the threshold is met
        // This node must the current leader to accept these views or they are thrown out
        // Only the first valid asset ID where the current node is the leader is returned
        let asset_id_view_mapping = View::threshold_met(&client).await?;
        for (asset_id, views) in asset_id_view_mapping {
            let leader_node_id = ConsensusCommittee::determine_leader_node_id(&asset_id).await?;

            if leader_node_id == node_id {
                return Ok(Some(ConsensusCommittee {
                    asset_id,
                    leader_node_id,
                    state: CommitteeState::ViewThresholdReached { views },
                }));
            } else {
                // We are not the leader, invalidate pending views
                View::invalidate(views, &client).await?;
            }
        }

        if let Some((asset_id, pending_instructions)) = Instruction::find_pending(&client).await? {
            let leader_node_id = ConsensusCommittee::determine_leader_node_id(&asset_id).await?;
            return Ok(Some(ConsensusCommittee {
                asset_id,
                leader_node_id,
                state: CommitteeState::PreparingView { pending_instructions },
            }));
        }

        Ok(None)
    }

    // Determines leader node ID for this round of consensus
    pub async fn determine_leader_node_id(_asset_id: &AssetID) -> Result<NodeID, ConsensusError> {
        Ok(NodeID::stub())
    }

    /// Aquires a lock on the asset state table preventing other consensus workers from working on these
    /// instructions in tandem
    pub async fn acquire_lock(&self, lock_period: u64, client: &Client) -> Result<(), ConsensusError> {
        match AssetState::find_by_asset_id(&self.asset_id, &client).await? {
            Some(mut asset_state) => Ok(asset_state.acquire_lock(lock_period, &client).await?),
            None => Err(ConsensusError::error("Failed to load asset state")),
        }
    }

    /// Removes time lock on asset state allowing other consensus workers to handle next state transition
    pub async fn release_lock(&self, client: &Client) -> Result<(), ConsensusError> {
        match AssetState::find_by_asset_id(&self.asset_id, &client).await? {
            Some(asset_state) => Ok(asset_state.release_lock(&client).await?),
            None => Err(ConsensusError::error("Failed to load asset state")),
        }
    }

    /// Prepares new view that includes append only state data for the purpose of broadcasting to the leader
    pub async fn prepare_new_view(
        &self,
        pending_instructions: Vec<Instruction>,
        client: &Client,
    ) -> Result<NewView, ConsensusError>
    {
        let mut instruction_set = Vec::new();
        let mut invalid_instruction_set = Vec::new();
        let mut asset_state = Vec::new();
        let mut token_state = Vec::new();

        for pending_instruction in pending_instructions {
            match pending_instruction.execute(&client).await {
                Ok((mut new_asset_state, mut new_token_state)) => {
                    instruction_set.push(pending_instruction.id.0);
                    asset_state.append(&mut new_asset_state);
                    token_state.append(&mut new_token_state);
                },
                Err(_) => {
                    // Instruction failed to execute
                    invalid_instruction_set.push(pending_instruction.id.0)
                },
            }
        }

        Ok(NewView {
            instruction_set,
            invalid_instruction_set,
            append_only_state: AppendOnlyState {
                asset_state,
                token_state,
            },
            asset_id: self.asset_id.clone(),
            initiating_node_id: NodeID::stub(),
            signature: "stub-signature".into(),
        })
    }

    /// Leader creates proposal
    pub async fn create_proposal(
        &self,
        node_id: NodeID,
        views: &mut [View],
        client: &Client,
    ) -> Result<Proposal, ConsensusError>
    {
        let view = self.select_view(views, &client).await?;
        let params = NewProposal {
            id: ProposalID::new(node_id).await?,
            node_id: NodeID::stub(),
            asset_id: view.asset_id.clone(),
            new_view: view.into(),
        };
        let proposal = Proposal::insert(params, &client).await.unwrap();

        // Leader signs proposal and stores record so their approval is included in the supermajority
        proposal.sign(&client).await?;

        Ok(proposal)
    }

    /// Select view from set of views provided by committee
    pub async fn select_view(&self, views: &mut [View], client: &Client) -> Result<View, ConsensusError> {
        // TODO: this logic needs to be adjusted for logic to select the winning view to propose
        // Hardcoded to the last view currently.
        let (first_view, remaining_views) = views
            .split_first()
            .ok_or_else(|| ConsensusError::error("No view available for selection"))?;

        // Update state of view to PreCommit
        let data = UpdateView {
            status: Some(ViewStatus::PreCommit),
            ..UpdateView::default()
        };
        let first_view = first_view.clone().update(data, &client).await?;

        // Update state of other views to NotChosen
        let view_ids: Vec<Uuid> = remaining_views.into_iter().map(|v| v.id).collect();
        View::update_views_status(&view_ids, ViewStatus::NotChosen, &client).await?;

        Ok(first_view)
    }

    /// Confirm proposal provided by leader node checking the resulting state
    pub async fn confirm_proposal(&self, _proposal: &Proposal) -> Result<bool, ConsensusError> {
        // TODO: Should the logic fetch any missing instructions it sees in the proposal from its peers at this point?
        //       Or immediately fail and take part in the next consensus period?

        Ok(true)
    }

    /// Prepares aggregate signature for broadcasting to committee members to finalize state
    pub async fn prepare_aggregate_signature_message(
        &self,
        proposal: &Proposal,
        signed_proposals: &[SignedProposal],
    ) -> Result<NewAggregateSignatureMessage, ConsensusError>
    {
        let mut signatures: Vec<(NodeID, String)> = Vec::new();
        // TODO: validate signatures as part of comms behavior on signed proposal message from replicants as condition
        // of entering database
        for signed_proposal in signed_proposals {
            signatures.push((signed_proposal.node_id, signed_proposal.signature.clone()));
        }
        Ok(NewAggregateSignatureMessage {
            proposal_id: proposal.id,
            signature_data: SignatureData { signatures },
        })
    }

    /// Validates aggregate signature message contents confirming signatures
    pub async fn validate_aggregate_signature_message(
        &self,
        _proposal: Proposal,
        _aggregate_signature_message: AggregateSignatureMessage,
    ) -> Result<(), ConsensusError>
    {
        Ok(())
    }

    /// Checks if this node is the current leader
    pub fn is_leader(&self, current_node_id: NodeID) -> bool {
        self.leader_node_id == current_node_id
    }
}
