use super::AssetStateBuilder;
use crate::db::models::*;
use rand::prelude::*;
use serde_json::Value;
use tokio_postgres::Client;
use uuid::Uuid;

#[allow(dead_code)]
pub struct TokenBuilder<'a> {
    owner_pub_key: String,
    asset_state_id: Option<Uuid>,
    additional_data_json: Value,
    client: &'a Client,
}

#[allow(dead_code)]
impl<'a> TokenBuilder<'a> {
    pub fn new(client: &'a Client) -> Self {
        let x: u32 = random();
        TokenBuilder {
            owner_pub_key: format!("7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0{}", x).into(),
            asset_state_id: None,
            additional_data_json: serde_json::from_str("{}").unwrap(),
            client,
        }
    }

    pub fn with_owner_pub_key(mut self, owner_pub_key: String) -> Self {
        self.owner_pub_key = owner_pub_key;
        self
    }

    pub fn with_asset_state_id(mut self, asset_state_id: Uuid) -> Self {
        self.asset_state_id = Some(asset_state_id);
        self
    }

    pub fn with_additional_data_json(mut self, additional_data_json: Value) -> Self {
        self.additional_data_json = additional_data_json;
        self
    }

    pub async fn finish(&self) -> anyhow::Result<Token> {
        let asset_state_id = match self.asset_state_id {
            Some(asset_state_id) => asset_state_id,
            None => AssetStateBuilder::new(self.client).finish().await?.id,
        };

        let params = NewToken {
            owner_pub_key: self.owner_pub_key.to_owned(),
            additional_data_json: self.additional_data_json.to_owned(),
            asset_state_id,
        };
        let token_id = Token::insert(params, self.client).await?;
        Ok(Token::load(token_id, self.client).await?)
    }
}
