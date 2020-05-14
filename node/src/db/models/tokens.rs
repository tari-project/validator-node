use super::{AppendOnlyStatus, TokenStatus};
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
}

/// Query parameters for adding new token state append only
#[derive(Default, Clone, Debug)]
pub struct NewTokenAppendOnly {
    pub token_id: uuid::Uuid,
    pub status: AppendOnlyStatus,
    pub state_data_json: Value,
}

impl Token {
    /// Add token record
    pub async fn insert(params: NewToken, client: &Client) -> Result<uuid::Uuid, DBError> {
        const QUERY: &'static str = "
            INSERT INTO tokens (
                owner_pub_key,
                asset_state_id,
                initial_data_json
            ) VALUES ($1, $2, $3) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.owner_pub_key,
                &params.asset_state_id,
                &params.initial_data_json,
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
                state_data_json,
                status
            ) VALUES ($1, $2, $3) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[&params.token_id, &params.state_data_json, &params.status])
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
        let initial_data = json!({"value": true, "value2": 4});
        let token = TokenBuilder {
            initial_data_json: initial_data.clone(),
            ..TokenBuilder::default()
        }
        .build(&client)
        .await?;
        assert_eq!(json!(initial_data), token.initial_data_json);
        assert_eq!(json!(initial_data), token.additional_data_json);

        let empty_value: Option<String> = None;
        let state_data_json = json!({"value": empty_value.clone(), "value2": 8, "value3": 2});
        Token::store_append_only_state(
            NewTokenAppendOnly {
                token_id: token.id,
                state_data_json: state_data_json.clone(),
                status: AppendOnlyStatus::Commit,
            },
            &client,
        )
        .await?;
        let token = Token::load(token.id, &client).await?;
        assert_eq!(state_data_json, token.additional_data_json);

        let state_data_json = json!({"value": false, "value3": empty_value.clone()});
        Token::store_append_only_state(
            NewTokenAppendOnly {
                token_id: token.id,
                state_data_json: state_data_json.clone(),
                status: AppendOnlyStatus::Commit,
            },
            &client,
        )
        .await?;
        let token = Token::load(token.id, &client).await?;
        assert_eq!(state_data_json.clone(), token.additional_data_json);

        let pre_commit_state_data_json = json!({"value": true, "value3": 1});
        Token::store_append_only_state(
            NewTokenAppendOnly {
                token_id: token.id,
                state_data_json: pre_commit_state_data_json,
                status: AppendOnlyStatus::PreCommit,
            },
            &client,
        )
        .await?;
        let token = Token::load(token.id, &client).await?;
        assert_eq!(state_data_json, token.additional_data_json);

        Ok(())
    }
}
