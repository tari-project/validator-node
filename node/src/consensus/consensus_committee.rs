use super::errors::ConsensusError;
use crate::{
    db::models::{consensus::*, AggregateSignatureMessageStatus, AssetState, SignedProposalStatus, ViewStatus},
    types::{consensus::*, AssetID, NodeID, ProposalID},
};
use deadpool_postgres::Client;
use uuid::Uuid;

#[derive(Debug, PartialEq)]
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
        node_id: NodeID,
        pending_instructions: &[Instruction],
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
        let new_view = NewView {
            instruction_set,
            invalid_instruction_set,
            append_only_state: AppendOnlyState {
                asset_state,
                token_state,
            },
            asset_id: self.asset_id.clone(),
            initiating_node_id: NodeID::stub(),
            signature: "stub-signature".into(),
        };

        // Leader stores the view
        if self.is_leader(node_id) {
            View::insert(new_view.clone(), NewViewAdditionalParameters::default(), &client).await?;
        }

        Ok(new_view)
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
        proposal.sign(node_id, &client).await?;

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
        client: &Client,
    ) -> Result<NewAggregateSignatureMessage, ConsensusError>
    {
        let mut signatures: Vec<(NodeID, String)> = Vec::new();
        // TODO: validate signatures as part of comms behavior on signed proposal message from replicants as condition
        // of entering database
        for signed_proposal in signed_proposals {
            signatures.push((signed_proposal.node_id, signed_proposal.signature.clone()));
            signed_proposal
                .update(
                    UpdateSignedProposal {
                        status: Some(SignedProposalStatus::Validated),
                    },
                    &client,
                )
                .await?;
        }
        let new_message = NewAggregateSignatureMessage {
            proposal_id: proposal.id,
            signature_data: SignatureData { signatures },
            status: AggregateSignatureMessageStatus::Pending,
        };

        // Save aggregate message in accepted state for leader
        let mut leader_message = new_message.clone();
        leader_message.status = AggregateSignatureMessageStatus::Accepted;
        leader_message.save(&client).await?;

        Ok(new_message)
    }

    /// Validates aggregate signature message contents confirming signatures
    pub async fn validate_aggregate_signature_message(
        &self,
        _proposal: &Proposal,
        _aggregate_signature_message: &AggregateSignatureMessage,
    ) -> Result<(), ConsensusError>
    {
        Ok(())
    }

    /// Checks if this node is the current leader
    pub fn is_leader(&self, current_node_id: NodeID) -> bool {
        self.leader_node_id == current_node_id
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::*,
        test::utils::{
            builders::{
                consensus::{
                    AggregateSignatureMessageBuilder,
                    InstructionBuilder,
                    ProposalBuilder,
                    SignedProposalBuilder,
                    ViewBuilder,
                },
                AssetStateBuilder,
            },
            test_db_client,
        },
    };
    use chrono::Utc;

    #[actix_rt::test]
    async fn find_next_pending_committee() {
        let (client, _lock) = test_db_client().await;
        // Given all model instances exist pending: AggregateSignatureMessage, SignedProposal, Proposal, View,
        // Instruction Committee work finalizing a round always takes precidence over new work in that order
        // Test emphasizes two things:
        // 1) Asset lock removes consensus work from the queue (all 2nd instances of model instances pending)
        // 2) State change in the underlying data removes work from the queue
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let proposal2 = ProposalBuilder::default().build(&client).await.unwrap();
        AssetState::find_by_asset_id(&proposal2.asset_id, &client)
            .await
            .unwrap()
            .unwrap()
            .acquire_lock(60 as u64, &client)
            .await
            .unwrap();

        let aggregate_signature_message = AggregateSignatureMessageBuilder::default()
            .build(&client)
            .await
            .unwrap();
        let _aggregate_signature_message2 = AggregateSignatureMessageBuilder {
            proposal_id: Some(proposal2.id),
            ..AggregateSignatureMessageBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        let signed_proposal = SignedProposalBuilder::default().build(&client).await.unwrap();
        let _signed_proposal2 = SignedProposalBuilder {
            proposal_id: Some(proposal2.id),
            ..SignedProposalBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        let view = ViewBuilder::default().build(&client).await.unwrap();
        let _view2 = ViewBuilder {
            asset_id: Some(proposal2.asset_id.clone()),
            ..ViewBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        let _instruction2 = InstructionBuilder {
            asset_id: Some(proposal2.asset_id.clone()),
            ..InstructionBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();

        // Leader finalized proposal received state
        let found_pending_committee = ConsensusCommittee::find_next_pending_committee(NodeID::stub(), &client)
            .await
            .unwrap();
        assert!(found_pending_committee.is_some());
        let found_pending_committee = found_pending_committee.unwrap();
        let found_proposal = aggregate_signature_message.proposal(&client).await.unwrap();
        assert_eq!(
            found_pending_committee.state,
            CommitteeState::LeaderFinalizedProposalReceived {
                proposal: found_proposal.clone(),
                aggregate_signature_message: aggregate_signature_message.clone(),
            }
        );
        assert_eq!(found_pending_committee.asset_id, found_proposal.asset_id);
        let data = UpdateAggregateSignatureMessage {
            status: Some(AggregateSignatureMessageStatus::Accepted),
            ..UpdateAggregateSignatureMessage::default()
        };
        aggregate_signature_message.update(data, &client).await.unwrap();

        // Signed proposal threshold reached
        let found_pending_committee = ConsensusCommittee::find_next_pending_committee(NodeID::stub(), &client)
            .await
            .unwrap();
        assert!(found_pending_committee.is_some());
        let found_pending_committee = found_pending_committee.unwrap();
        let found_proposal = Proposal::load(signed_proposal.proposal_id, &client).await.unwrap();
        assert_eq!(
            found_pending_committee.state,
            CommitteeState::SignedProposalThresholdReached {
                proposal: found_proposal.clone(),
                signed_proposals: vec![signed_proposal.clone()],
            }
        );
        assert_eq!(found_pending_committee.asset_id, found_proposal.asset_id);
        let data = UpdateSignedProposal {
            status: Some(SignedProposalStatus::Validated),
            ..UpdateSignedProposal::default()
        };
        signed_proposal.update(data, &client).await.unwrap();

        // Proposal pending
        let found_pending_committee = ConsensusCommittee::find_next_pending_committee(NodeID::stub(), &client)
            .await
            .unwrap();
        assert!(found_pending_committee.is_some());
        let found_pending_committee = found_pending_committee.unwrap();
        assert_eq!(found_pending_committee.state, CommitteeState::ReceivedLeaderProposal {
            proposal: proposal.clone(),
        });
        assert_eq!(found_pending_committee.asset_id, proposal.asset_id);
        let data = UpdateProposal {
            status: Some(ProposalStatus::Signed),
            ..UpdateProposal::default()
        };
        proposal.update(data, &client).await.unwrap();

        // View pending
        let found_pending_committee = ConsensusCommittee::find_next_pending_committee(NodeID::stub(), &client)
            .await
            .unwrap();
        assert!(found_pending_committee.is_some());
        let found_pending_committee = found_pending_committee.unwrap();
        assert_eq!(found_pending_committee.state, CommitteeState::ViewThresholdReached {
            views: vec![view.clone()],
        });
        assert_eq!(found_pending_committee.asset_id, view.asset_id);
        let data = UpdateView {
            status: Some(ViewStatus::PreCommit),
            ..UpdateView::default()
        };
        view.update(data, &client).await.unwrap();

        // Instruction pending
        let found_pending_committee = ConsensusCommittee::find_next_pending_committee(NodeID::stub(), &client)
            .await
            .unwrap();
        assert!(found_pending_committee.is_some());
        let found_pending_committee = found_pending_committee.unwrap();
        assert_eq!(found_pending_committee.state, CommitteeState::PreparingView {
            pending_instructions: vec![instruction.clone()],
        });
        assert_eq!(found_pending_committee.asset_id, instruction.asset_id);
        let data = UpdateInstruction {
            status: Some(InstructionStatus::Commit),
            ..UpdateInstruction::default()
        };
        instruction.update(data, &client).await.unwrap();

        let found_pending_committee = ConsensusCommittee::find_next_pending_committee(NodeID::stub(), &client)
            .await
            .unwrap();
        assert!(found_pending_committee.is_none());
    }

    #[actix_rt::test]
    async fn determine_leader_node_id() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let leader_node = ConsensusCommittee::determine_leader_node_id(&asset.asset_id)
            .await
            .unwrap();
        assert_eq!(leader_node, NodeID::stub());
    }

    #[actix_rt::test]
    async fn acquire_and_release_lock() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let asset2 = AssetStateBuilder::default().build(&client).await.unwrap();
        let consensus_committee = test_committee(Some(asset.asset_id), NodeID::stub(), &client).await;
        assert!(asset.blocked_until <= Utc::now());
        assert!(asset2.blocked_until <= Utc::now());

        consensus_committee.acquire_lock(10, &client).await.unwrap();
        let asset = AssetState::load(asset.id, &client).await.unwrap();
        let asset2 = AssetState::load(asset2.id, &client).await.unwrap();
        assert!(asset.blocked_until > Utc::now());
        assert!(asset2.blocked_until <= Utc::now());

        consensus_committee.release_lock(&client).await.unwrap();
        let asset = AssetState::load(asset.id, &client).await.unwrap();
        let asset2 = AssetState::load(asset2.id, &client).await.unwrap();
        assert!(asset.blocked_until <= Utc::now());
        assert!(asset2.blocked_until <= Utc::now());
    }

    #[actix_rt::test]
    async fn prepare_new_view() {
        let (client, _lock) = test_db_client().await;
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        let instructions = vec![instruction.clone()];
        let consensus_committee = test_committee(None, NodeID::stub(), &client).await;
        let new_view = consensus_committee
            .prepare_new_view(NodeID::stub(), &instructions, &client)
            .await
            .unwrap();
        assert_eq!(new_view.asset_id, consensus_committee.asset_id);
        assert_eq!(new_view.instruction_set, vec![instruction.id.0]);
        assert_eq!(new_view.invalid_instruction_set, Vec::new());
        assert_eq!(new_view.append_only_state, AppendOnlyState {
            asset_state: Vec::new(),
            token_state: Vec::new(),
        });
        assert_eq!(new_view.initiating_node_id, NodeID::stub());
    }

    #[actix_rt::test]
    async fn create_proposal() {
        let (client, _lock) = test_db_client().await;
        let view = ViewBuilder::default().build(&client).await.unwrap();
        assert_eq!(view.status, ViewStatus::Prepare);

        let consensus_committee = test_committee(None, NodeID::stub(), &client).await;
        let mut views = vec![view.clone()];
        let node_id = NodeID::stub();

        // Create proposal selects the view, saves a new proposal, and signs a copy
        let proposal = consensus_committee
            .create_proposal(NodeID::stub(), &mut views, &client)
            .await
            .unwrap();
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.node_id, node_id);
        assert_eq!(proposal.new_view, view.clone().into());

        let view = View::load(view.id, &client).await.unwrap();
        assert_eq!(view.status, ViewStatus::PreCommit);

        let signed_proposals = SignedProposal::load_by_proposal_id(proposal.id, &client).await.unwrap();
        assert_eq!(signed_proposals.len(), 1);
        assert_eq!(signed_proposals[0].proposal_id, proposal.id);
        assert_eq!(signed_proposals[0].node_id, node_id);
    }

    #[actix_rt::test]
    async fn select_view() {
        let (client, _lock) = test_db_client().await;
        let view = ViewBuilder::default().build(&client).await.unwrap();
        let view2 = ViewBuilder::default().build(&client).await.unwrap();
        assert_eq!(view.status, ViewStatus::Prepare);
        assert_eq!(view2.status, ViewStatus::Prepare);

        let consensus_committee = test_committee(None, NodeID::stub(), &client).await;
        let mut views = vec![view.clone(), view2.clone()];
        assert_eq!(
            consensus_committee.select_view(&mut views, &client).await.unwrap().id,
            view.id
        );

        let view = View::load(view.id, &client).await.unwrap();
        let view2 = View::load(view2.id, &client).await.unwrap();
        assert_eq!(view.status, ViewStatus::PreCommit);
        assert_eq!(view2.status, ViewStatus::NotChosen);
    }

    #[actix_rt::test]
    async fn confirm_proposal() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let consensus_committee = test_committee(None, NodeID::stub(), &client).await;
        assert!(consensus_committee.confirm_proposal(&proposal).await.unwrap());
    }

    #[actix_rt::test]
    async fn validate_aggregate_signature_message() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let aggregate_signature_message = AggregateSignatureMessageBuilder::default()
            .build(&client)
            .await
            .unwrap();
        let consensus_committee = test_committee(None, NodeID::stub(), &client).await;
        assert!(consensus_committee
            .validate_aggregate_signature_message(&proposal, &aggregate_signature_message)
            .await
            .is_ok());
    }

    #[actix_rt::test]
    async fn is_leader() {
        let (client, _lock) = test_db_client().await;
        let consensus_committee = test_committee(None, NodeID::stub(), &client).await;
        assert!(consensus_committee.is_leader(NodeID::stub()));
        let other_node_id = NodeID([0, 1, 2, 3, 4, 6]);
        assert!(!consensus_committee.is_leader(other_node_id));
    }

    async fn test_committee(asset_id: Option<AssetID>, node_id: NodeID, client: &Client) -> ConsensusCommittee {
        let asset_id: AssetID = match asset_id {
            Some(asset_id) => asset_id.clone(),
            None => AssetStateBuilder::default().build(&client).await.unwrap().asset_id,
        };

        ConsensusCommittee {
            state: CommitteeState::PreparingView {
                pending_instructions: Vec::new(),
            },
            asset_id,
            leader_node_id: node_id,
        }
    }
}
