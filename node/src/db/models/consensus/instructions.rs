use crate::{
    db::{
        models::{InstructionStatus, NewAssetStateAppendOnly, NewTokenStateAppendOnly},
        utils::errors::DBError,
    },
    types::{AssetID, InstructionID, NodeID, ProposalID, TemplateID, TokenID},
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::Type;

#[derive(Clone, Deserialize, Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "instructions")]
pub struct Instruction {
    pub id: InstructionID,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub asset_id: AssetID,
    pub token_id: Option<TokenID>,
    pub template_id: TemplateID,
    pub contract_name: String,
    pub status: InstructionStatus,
    pub params: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub proposal_id: Option<ProposalID>,
}

/// Query parameters for adding new instruction record
#[derive(Default, Clone, Debug)]
pub struct NewInstruction {
    pub id: InstructionID,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub asset_id: AssetID,
    pub token_id: Option<TokenID>,
    pub template_id: TemplateID,
    pub contract_name: String,
    pub status: InstructionStatus,
    pub params: Value,
}

/// Query parameters for optionally updating instruction fields
#[derive(Default, Clone, Debug)]
pub struct UpdateInstruction {
    pub status: Option<InstructionStatus>,
    pub proposal_id: Option<ProposalID>,
}

impl Instruction {
    pub async fn find_pending(client: &Client) -> Result<Option<(AssetID, Vec<Self>)>, DBError> {
        let stmt = "
            SELECT i.*
            FROM instructions i
            JOIN (
                SELECT i.asset_id
                FROM instructions i
                JOIN asset_states ast ON ast.asset_id = i.asset_id
                WHERE i.status = 'Pending'
                AND ast.blocked_until <= now()
                LIMIT 1
            ) i2 ON i.asset_id = i2.asset_id
            AND i.status = 'Pending'
        ";

        let instructions: Vec<Instruction> = client
            .query(stmt, &[])
            .await?
            .into_iter()
            .map(|row| Instruction::from_row(row))
            .collect::<Result<Vec<_>, _>>()?;

        if instructions.len() > 0 {
            Ok(Some((instructions[0].asset_id.clone(), instructions)))
        } else {
            Ok(None)
        }
    }

    /// Add digital asset record
    pub async fn insert(params: NewInstruction, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            INSERT INTO instructions (
                initiating_node_id,
                signature,
                asset_id,
                token_id,
                template_id,
                contract_name,
                status,
                params,
                id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *";
        let stmt = client
            .prepare_typed(QUERY, &[
                NodeID::SQL_TYPE,
                Type::TEXT,
                AssetID::SQL_TYPE,
                TokenID::SQL_TYPE,
                TemplateID::SQL_TYPE,
                Type::TEXT,
                Type::TEXT,
                Type::JSONB,
            ])
            .await?;

        let row = client
            .query_one(&stmt, &[
                &params.initiating_node_id,
                &params.signature,
                &params.asset_id,
                &params.token_id,
                &params.template_id,
                &params.contract_name,
                &params.status,
                &params.params,
                &params.id,
            ])
            .await?;
        Ok(Self::from_row(row)?)
    }

    /// Marks set of instructions as given status and sets proposal id for reference if provided
    pub async fn update_instructions_status(
        instruction_ids: &[InstructionID],
        proposal_id: Option<ProposalID>,
        status: InstructionStatus,
        client: &Client,
    ) -> Result<(), DBError>
    {
        const QUERY: &'static str = "
            UPDATE instructions SET
                status = $2,
                proposal_id = $3,
                updated_at = NOW()
            WHERE id::uuid = ANY ($1)";
        let stmt = client.prepare_typed(QUERY, &[Type::UUID_ARRAY, Type::TEXT]).await?;
        client
            .execute(&stmt, &[
                &instruction_ids.iter().map(|i| i.0).collect::<Vec<uuid::Uuid>>(),
                &status,
                &proposal_id,
            ])
            .await?;

        Ok(())
    }

    /// Update instruction state in the database
    ///
    /// Updates subset of fields:
    /// - status
    /// - proposal_id
    pub async fn update(self, data: UpdateInstruction, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            UPDATE instructions SET
                status = COALESCE($1, status),
                proposal_id = $2::\"ProposalID\",
                updated_at = NOW()
            WHERE id = $3::\"InstructionID\"
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::TEXT]).await?;
        let row = client
            .query_one(&stmt, &[&data.status, &data.proposal_id, &self.id])
            .await?;
        Ok(Self::from_row(row)?)
    }

    /// Load instruction record
    pub async fn load(id: InstructionID, client: &Client) -> Result<Self, DBError> {
        let stmt = "SELECT * FROM instructions WHERE id = $1::\"InstructionID\"";
        let row = client.query_one(stmt, &[&id]).await?;
        Ok(Self::from_row(row)?)
    }

    /// Execute the instruction returning append only state
    pub async fn execute(
        &self,
        _client: &Client,
    ) -> Result<(Vec<NewAssetStateAppendOnly>, Vec<NewTokenStateAppendOnly>), DBError>
    {
        // TODO: we will need to encapsulate the running of an instruction somehow so that nodes can compare view state
        // to expected state
        Ok((Vec::new(), Vec::new()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::*,
        test::utils::{
            builders::{
                consensus::{InstructionBuilder, ProposalBuilder},
                AssetStateBuilder,
            },
            test_db_client,
        },
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn find_pending() {
        let (client, _lock) = test_db_client().await;
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        let instruction2 = InstructionBuilder::default().build(&client).await.unwrap();
        let instruction3 = InstructionBuilder::default().build(&client).await.unwrap();

        // instruction is ignored if an existing block is present
        let mut asset_state = AssetState::find_by_asset_id(&instruction.asset_id, &client)
            .await
            .unwrap()
            .unwrap();
        asset_state.acquire_lock(60 as u64, &client).await.unwrap();

        // instruction3 is ignored as it is not pending
        instruction3
            .update(
                UpdateInstruction {
                    status: Some(InstructionStatus::Commit),
                    ..UpdateInstruction::default()
                },
                &client,
            )
            .await
            .unwrap();

        let instructions = Instruction::find_pending(&client).await.unwrap();
        assert_eq!(instructions, Some((instruction2.asset_id.clone(), vec![instruction2])));
    }

    #[actix_rt::test]
    async fn update_instructions_status() {
        let (client, _lock) = test_db_client().await;
        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        let instruction2 = InstructionBuilder::default().build(&client).await.unwrap();
        let instruction3 = InstructionBuilder::default().build(&client).await.unwrap();

        Instruction::update_instructions_status(
            &vec![instruction.id, instruction2.id],
            Some(proposal.id),
            InstructionStatus::Commit,
            &client,
        )
        .await
        .unwrap();

        // Reload instructions confirming state changes where expected
        let instruction = Instruction::load(instruction.id, &client).await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Commit);
        assert_eq!(instruction.proposal_id, Some(proposal.id));

        let instruction2 = Instruction::load(instruction2.id, &client).await.unwrap();
        assert_eq!(instruction2.status, InstructionStatus::Commit);
        assert_eq!(instruction2.proposal_id, Some(proposal.id));

        // Expects unchanged
        let instruction3 = Instruction::load(instruction3.id, &client).await.unwrap();
        assert_eq!(instruction3.status, InstructionStatus::Pending);
        assert!(instruction3.proposal_id.is_none());
    }

    #[actix_rt::test]
    async fn execute() {
        let (client, _lock) = test_db_client().await;
        let instruction = InstructionBuilder::default().build(&client).await.unwrap();
        let (new_asset_state_append_only, new_token_state_append_only) = instruction.execute(&client).await.unwrap();
        assert_eq!(new_asset_state_append_only, Vec::new());
        assert_eq!(new_token_state_append_only, Vec::new());
    }

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let params = NewInstruction {
            asset_id: asset.asset_id.clone(),
            template_id: asset.asset_id.template_id(),
            contract_name: "test_contract".into(),
            params: json!({"test_param": 1}),
            ..NewInstruction::default()
        };
        let instruction = Instruction::insert(params, &client).await.unwrap();
        assert_eq!(instruction.template_id, asset.asset_id.template_id());
        assert_eq!(instruction.params, json!({"test_param": 1}));
        assert_eq!(instruction.status, InstructionStatus::Pending);

        let initial_updated_at = instruction.updated_at;
        let data = UpdateInstruction {
            status: Some(InstructionStatus::Commit),
            ..UpdateInstruction::default()
        };
        let updated = instruction.update(data, &client).await.unwrap();
        assert_eq!(updated.status, InstructionStatus::Commit);

        let instruction2 = Instruction::load(updated.id, &client).await.unwrap();
        assert_eq!(instruction2.id, updated.id);
        assert_eq!(instruction2.asset_id, updated.asset_id);
        assert_eq!(instruction2.token_id, updated.token_id);
        assert_eq!(instruction2.status, updated.status);
        assert!(instruction2.updated_at > initial_updated_at);
    }
}
