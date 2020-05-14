use super::AssetStateBuilder;
use crate::db::models::*;
use rand::prelude::*;
use serde_json::Value;
use tokio_postgres::Client;
use uuid::Uuid;

#[allow(dead_code)]
pub struct TokenBuilder {
    pub owner_pub_key: String,
    pub asset_state_id: Option<Uuid>,
    pub initial_data_json: Value,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for TokenBuilder {
    fn default() -> Self {
        let x: u32 = random();
        Self {
            owner_pub_key: format!("7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0{}", x).into(),
            asset_state_id: None,
            initial_data_json: serde_json::from_str("{}").unwrap(),
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl TokenBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<Token> {
        let asset_state_id = match self.asset_state_id {
            Some(asset_state_id) => asset_state_id,
            None => AssetStateBuilder::default().build(client).await?.id,
        };

        let params = NewToken {
            owner_pub_key: self.owner_pub_key.to_owned(),
            initial_data_json: self.initial_data_json.to_owned(),
            asset_state_id,
        };
        let token_id = Token::insert(params, client).await?;
        Ok(Token::load(token_id, client).await?)
    }
}
