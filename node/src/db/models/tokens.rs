use super::TokenStatus;
use crate::{db::utils::errors::DBError, types::TokenID};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use serde::Serialize;
use serde_json::Value;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::Type;

#[derive(Clone, Serialize, PostgresMapper)]
#[pg_mapper(table = "tokens")]
pub struct Token {
    pub id: uuid::Uuid,
    pub issue_number: i64,
    pub owner_pub_key: String,
    pub status: TokenStatus,
    pub token_id: TokenID,
    pub asset_state_id: uuid::Uuid,
    pub additional_data_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Query paramteres for adding new token record
#[derive(Default, Clone, Debug)]
pub struct NewToken {
    pub token_id: TokenID,
    pub owner_pub_key: String,
    pub asset_state_id: uuid::Uuid,
    pub additional_data_json: Value,
}

impl Token {
    /// Add token record
    pub async fn insert(params: NewToken, client: &Client) -> Result<uuid::Uuid, DBError> {
        const QUERY: &'static str = "
            INSERT INTO tokens (
                owner_pub_key,
                asset_state_id,
                additional_data_json,
                token_id
            ) VALUES ($1, $2, $3, $4) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.owner_pub_key,
                &params.asset_state_id,
                &params.additional_data_json,
                &params.token_id,
            ])
            .await?;

        Ok(result.get(0))
    }

    /// Update token into database
    ///
    /// Updates subset of fields:
    /// - owner_pub_key
    /// - additional_data_json
    pub async fn update(&self, client: &Client) -> Result<u64, DBError> {
        const QUERY: &'static str =
            "UPDATE tokens SET owner_pub_key = $2, additional_data_json = $3, updated_at = NOW() WHERE id = $1";
        let stmt = client.prepare(QUERY).await?;
        let updated = client
            .execute(&stmt, &[&self.id, &self.owner_pub_key, &self.additional_data_json])
            .await?;
        Ok(updated)
    }

    /// Load token record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<Token, DBError> {
        let stmt = "SELECT * FROM tokens WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Token::from_row(result)?)
    }

    /// Find token record by token id )
    pub async fn find_by_token_id(token_id: TokenID, client: &Client) -> Result<Option<Token>, DBError> {
        const QUERY: &'static str = "SELECT * FROM tokens WHERE token_id = $1";
        let stmt = client.prepare_typed(QUERY, &[Type::TEXT]).await?;
        let result = client.query_opt(&stmt, &[&token_id]).await?;
        Ok(result.map(Self::from_row).transpose()?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{builders::*, test_db_client};
    use futures::future::try_join_all;
    use serde_json::json;

    const PUBKEY: &'static str = "7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0f";
    const NODE_ID: [u8; 6] = [0, 1, 2, 3, 4, 5];

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let asset2 = AssetStateBuilder::default().build(&client).await.unwrap();

        let params = NewToken {
            owner_pub_key: PUBKEY.to_string(),
            asset_state_id: asset.id,
            additional_data_json: json!({"value": true}),
            token_id: TokenID::new(&asset.asset_id, NODE_ID).unwrap(),
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await.unwrap();
        let token = Token::load(token_id, &client).await.unwrap();
        assert_eq!(token.owner_pub_key, PUBKEY.to_string());
        assert_eq!(token.asset_state_id, asset.id);
        assert_eq!(token.issue_number, 1);

        let params = NewToken {
            owner_pub_key: PUBKEY.to_string(),
            asset_state_id: asset.id,
            additional_data_json: json!({"value": true}),
            token_id: TokenID::new(&asset.asset_id, NODE_ID).unwrap(),
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await.unwrap();
        let token = Token::load(token_id, &client).await.unwrap();
        assert_eq!(token.owner_pub_key, PUBKEY.to_string());
        assert_eq!(token.asset_state_id, asset.id);
        assert_eq!(token.issue_number, 2);

        let params = NewToken {
            owner_pub_key: PUBKEY.to_string(),
            asset_state_id: asset2.id,
            additional_data_json: json!({"value": true}),
            token_id: TokenID::new(&asset.asset_id, NODE_ID).unwrap(),
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await.unwrap();
        let token = Token::load(token_id, &client).await.unwrap();
        assert_eq!(token.owner_pub_key, PUBKEY.to_string());
        assert_eq!(token.asset_state_id, asset2.id);
        assert_eq!(token.issue_number, 1);
    }

    #[actix_rt::test]
    async fn duplicate_token_id() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();

        let params = NewToken {
            owner_pub_key: PUBKEY.to_string(),
            asset_state_id: asset.id,
            additional_data_json: json!({"value": true}),
            token_id: TokenID::new(&asset.asset_id, NODE_ID).unwrap(),
            ..NewToken::default()
        };
        Token::insert(params.clone(), &client).await.unwrap();
        assert!(Token::insert(params, &client).await.is_err());
    }

    #[actix_rt::test]
    async fn issue_number_concurrency() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();

        let params = NewToken {
            owner_pub_key: PUBKEY.to_string(),
            asset_state_id: asset.id,
            additional_data_json: json!({"value": true}),
            ..NewToken::default()
        };
        let client = std::sync::Arc::new(client);
        let futs = (0..100).map(move |_| {
            let client = client.clone();
            let mut params = params.clone();
            params.token_id = TokenID::new(&asset.asset_id, NODE_ID).unwrap();
            async move { Token::insert(params, client.as_ref()).await }
        });
        let res = try_join_all(futs).await.unwrap();
        assert_eq!(res.len(), 100);
    }
}
