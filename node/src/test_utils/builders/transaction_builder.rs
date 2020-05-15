use super::AssetStateBuilder;
use crate::{db::models::*, types::TemplateID};
use deadpool_postgres::Client;
use serde_json::{json, Value};

#[allow(dead_code)]
pub struct ContractTransactionBuilder {
    pub asset_state_id: Option<uuid::Uuid>,
    pub token_id: Option<uuid::Uuid>,
    pub template_id: TemplateID,
    pub contract_name: String,
    pub status: TransactionStatus,
    pub params: Value,
    pub result: Value,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for ContractTransactionBuilder {
    fn default() -> Self {
        Self {
            asset_state_id: None,
            token_id: None,
            template_id: 999.into(),
            contract_name: "test_contract".into(),
            status: TransactionStatus::PreCommit,
            params: json!({}),
            result: json!({}),
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl ContractTransactionBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<ContractTransaction> {
        let asset_state_id = match self.asset_state_id {
            Some(asset_state_id) => asset_state_id,
            None => AssetStateBuilder::default().build(client).await?.id,
        };
        let params = NewContractTransaction {
            asset_state_id,
            token_id: self.token_id,
            template_id: self.template_id,
            contract_name: self.contract_name,
            status: self.status,
            params: self.params,
            result: self.result,
        };
        Ok(ContractTransaction::insert(params, client).await?)
    }
}
