use super::TokenStatus;
use crate::db::utils::errors::DBError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::Client;

#[derive(Serialize, PostgresMapper)]
#[pg_mapper(table = "tokens_view")]
pub struct Token {
    pub id: uuid::Uuid,
    pub issue_number: i64,
    pub owner_pub_key: String,
    pub status: TokenStatus,
    pub asset_state_id: uuid::Uuid,
    pub initial_data_json: Value,
    pub append_only_after: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub additional_data_json: Value,
}

/// Query parameters for adding new token record
#[derive(Default, Clone, Debug)]
pub struct NewToken {
    pub owner_pub_key: String,
    pub asset_state_id: uuid::Uuid,
    pub initial_data_json: Value,
    pub append_only_after: Option<DateTime<Utc>>,
}

/// Query parameters for adding new token state append only
#[derive(Default, Clone, Debug)]
pub struct NewTokenAppendOnly {
    pub token_id: uuid::Uuid,
    pub state_instruction: Value,
}

impl Token {
    /// Add token record
    pub async fn insert(params: NewToken, client: &Client) -> Result<uuid::Uuid, DBError> {
        const QUERY: &'static str = "
            INSERT INTO tokens (
                owner_pub_key,
                asset_state_id,
                initial_data_json,
                append_only_after
            ) VALUES ($1, $2, $3, $4) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.owner_pub_key,
                &params.asset_state_id,
                &params.initial_data_json,
                &params.append_only_after.unwrap_or(Utc::now()),
            ])
            .await?;

        Ok(result.get(0))
    }

    /// Load token record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<Token, DBError> {
        let stmt = "SELECT * FROM tokens_view WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Token::from_row(result)?)
    }

    // Store append only state
    pub async fn store_append_only_state(params: NewTokenAppendOnly, client: &Client) -> Result<uuid::Uuid, DBError> {
        const QUERY: &'static str = "
            INSERT INTO token_state_append_only (
                token_id,
                state_instruction
            ) VALUES ($1, $2) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[&params.token_id, &params.state_instruction])
            .await?;

        Ok(result.get(0))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{builders::*, load_env, test_db_client};
    use serde_json::json;
    use std::collections::HashMap;
    const PUBKEY: &'static str = "7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0f";

    #[actix_rt::test]
    async fn crud() -> anyhow::Result<()> {
        load_env();
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await?;
        let asset2 = AssetStateBuilder::default().build(&client).await?;
        let mut initial_data = HashMap::new();
        initial_data.insert("value", true);

        let params = NewToken {
            owner_pub_key: PUBKEY.to_string(),
            asset_state_id: asset.id,
            initial_data_json: serde_json::to_value(initial_data.clone())?,
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await?;
        let token = Token::load(token_id, &client).await?;
        assert_eq!(token.owner_pub_key, PUBKEY.to_string());
        assert_eq!(token.asset_state_id, asset.id);
        assert_eq!(token.issue_number, 1);

        let params = NewToken {
            owner_pub_key: PUBKEY.to_string(),
            asset_state_id: asset.id,
            initial_data_json: serde_json::to_value(initial_data.clone())?,
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await?;
        let token = Token::load(token_id, &client).await?;
        assert_eq!(token.owner_pub_key, PUBKEY.to_string());
        assert_eq!(token.asset_state_id, asset.id);
        assert_eq!(token.issue_number, 2);

        let params = NewToken {
            owner_pub_key: PUBKEY.to_string(),
            asset_state_id: asset2.id,
            initial_data_json: serde_json::to_value(initial_data)?,
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await?;
        let token = Token::load(token_id, &client).await?;
        assert_eq!(token.owner_pub_key, PUBKEY.to_string());
        assert_eq!(token.asset_state_id, asset2.id);
        assert_eq!(token.issue_number, 1);

        Ok(())
    }

    #[actix_rt::test]
    async fn store_append_only_state() -> anyhow::Result<()> {
        load_env();
        let (client, _lock) = test_db_client().await;
        let mut initial_data: HashMap<&str, Value> = HashMap::new();
        initial_data.insert("value", json!(true));
        initial_data.insert("value2", json!(4));
        let token = TokenBuilder {
            initial_data_json: json!(initial_data.clone()),
            ..TokenBuilder::default()
        }
        .build(&client)
        .await?;
        assert_eq!(json!(initial_data), token.initial_data_json);
        assert_eq!(json!(initial_data), token.additional_data_json);

        let mut state_instruction: HashMap<&str, Value> = HashMap::new();
        state_instruction.insert("value", Value::Null);
        state_instruction.insert("value2", json!(8));
        state_instruction.insert("value3", json!(2));
        Token::store_append_only_state(
            NewTokenAppendOnly {
                token_id: token.id,
                state_instruction: json!(state_instruction),
            },
            &client,
        )
        .await?;
        let mut expected_data = initial_data.clone();
        expected_data.insert("value", Value::Null);
        expected_data.insert("value2", json!(8));
        expected_data.insert("value3", json!(2));
        let token = Token::load(token.id, &client).await?;
        assert_eq!(json!(expected_data), token.additional_data_json);

        let mut state_instruction: HashMap<&str, Value> = HashMap::new();
        state_instruction.insert("value", json!(false));
        state_instruction.insert("value3", Value::Null);
        Token::store_append_only_state(
            NewTokenAppendOnly {
                token_id: token.id,
                state_instruction: json!(state_instruction),
            },
            &client,
        )
        .await?;
        expected_data.insert("value", json!(false));
        expected_data.insert("value2", json!(8));
        expected_data.insert("value3", Value::Null);
        let token = Token::load(token.id, &client).await?;
        assert_eq!(json!(expected_data), token.additional_data_json);

        // Ignore any asset append only additions from the past causing additional_data_json to equal initial_data_json
        let stmt = "update tokens set append_only_after = now() + INTERVAL '1 MINUTE' WHERE id = $1;";
        client.query(stmt, &[&token.id]).await?;
        let token = Token::load(token.id, &client).await?;
        assert_eq!(json!(initial_data), token.additional_data_json);

        Ok(())
    }
}
