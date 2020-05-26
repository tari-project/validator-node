use crate::{
    db::models::{consensus::*, InstructionStatus},
    types::{AssetID, InstructionID, NodeID, TemplateID, TokenID},
};
use crate::test::utils::{Test, builders::AssetStateBuilder};
use deadpool_postgres::Client;
use serde_json::{json, Value};

#[allow(dead_code)]
pub struct InstructionBuilder {
    pub id: Option<InstructionID>,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub asset_id: Option<AssetID>,
    pub token_id: Option<TokenID>,
    pub template_id: TemplateID,
    pub contract_name: String,
    pub status: InstructionStatus,
    pub params: Value,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for InstructionBuilder {
    fn default() -> Self {
        Self {
            id: None,
            initiating_node_id: Test::<NodeID>::new(),
            signature: "stub-signature".to_string(),
            asset_id: None,
            token_id: None,
            template_id: 999.into(),
            contract_name: "test_contract".into(),
            status: InstructionStatus::Pending,
            params: json!({}),
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl InstructionBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<Instruction> {
        let asset_id = match self.asset_id {
            Some(asset_id) => asset_id,
            None => AssetStateBuilder::default().build(client).await?.asset_id,
        };

        let id = match self.id {
            Some(id) => id,
            None => InstructionID::new(self.initiating_node_id)?,
        };

        let params = NewInstruction {
            id,
            asset_id,
            initiating_node_id: self.initiating_node_id,
            signature: self.signature,
            token_id: self.token_id,
            template_id: self.template_id,
            contract_name: self.contract_name,
            status: self.status,
            params: self.params,
        };
        Ok(Instruction::insert(params, client).await?)
    }
}
