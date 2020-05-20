use super::AssetStateBuilder;
use crate::{db::models::*, types::TemplateID};
use chrono::Local;
use deadpool_postgres::Client;
use serde_json::{json, Value};
use uuid::{
    v1::{Context, Timestamp},
    Uuid,
};

#[allow(dead_code)]
pub struct ViewBuilder {
    pub asset_id: Option<AssetID>,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub instruction_set: Vec<Uuid>,
    pub asset_state_append_only: Vec<NewAssetStateAppendOnly>,
    pub token_state_append_only: Vec<NewTokenStateAppendOnly>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for ViewBuilder {
    fn default() -> Self {
        Self {
            asset_id: None,
            initiating_node_id: NodeID::stub(),
            signature: "signature",
            instruction_set: Vec::new(),
            asset_state_append_only: Vec::new(),
            token_state_append_only: Vec::new(),
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl ViewBuilder {
    pub async fn prepare(self, client: &Client) -> anyhow::Result<NewView> {
        let asset_id = match self.asset_id {
            Some(asset_id) => asset_id,
            None => AssetStateBuilder::default().build(client).await?.asset_id,
        };

        NewView {
            asset_id,
            initiating_node_id: self.initiating_node_id,
            signature: self.signature,
            instruction_set: self.instruction_set,
            asset_state_append_only: self.asset_state_append_only,
            token_state_append_only: self.token_state_append_only,
        }
    }

    pub async fn build(self, client: &Client) -> anyhow::Result<View> {
        let asset_id = View::insert(self.prepare(client), ViewStatus::Prepare, client).await?;
        Ok(View::load(asset_id, client).await?)
    }
}
