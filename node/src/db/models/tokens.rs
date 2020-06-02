use super::{consensus::Instruction, TokenStatus};
use crate::{
    db::utils::errors::DBError,
    types::{InstructionID, TokenID},
};
use bytes::BytesMut;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use serde::{Deserialize, Serialize};
use serde_json::{
    json,
    map::Map,
    Value::{self, Object},
};
use std::error::Error;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::{accepts, to_sql_checked, FromSql, IsNull, Json, ToSql, Type};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, PostgresMapper)]
#[pg_mapper(table = "tokens_view")]
pub struct Token {
    pub id: uuid::Uuid,
    pub issue_number: i64,
    pub status: TokenStatus,
    pub token_id: TokenID,
    pub asset_state_id: uuid::Uuid,
    pub initial_data_json: Value,
    pub created_at: DateTime<Utc>,
    // TODO: switch view to use latest of append only or tokens updated_at
    pub updated_at: DateTime<Utc>,
    pub additional_data_json: Value,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayToken {
    pub token_id: TokenID,
    pub issue_number: i64,
    pub status: TokenStatus,
    pub initial_data_json: Value,
    pub additional_data_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Token> for DisplayToken {
    fn from(token: Token) -> Self {
        Self {
            token_id: token.token_id,
            issue_number: token.issue_number,
            status: token.status,
            initial_data_json: token.initial_data_json,
            additional_data_json: token.additional_data_json,
            created_at: token.created_at,
            updated_at: token.updated_at,
        }
    }
}

/// Query parameters for adding new token record
#[derive(Default, Clone, Debug)]
pub struct NewToken {
    pub token_id: TokenID,
    pub asset_state_id: uuid::Uuid,
    pub initial_data_json: Value,
}

/// Query parameters for adding new token state append only
#[derive(PartialEq, Deserialize, Serialize, Default, Clone, Debug)]
pub struct NewTokenStateAppendOnly {
    pub token_id: TokenID,
    pub instruction_id: InstructionID,
    pub status: TokenStatus,
    pub state_data_json: Value,
}

/// Query parameters for adding new token state append only
#[derive(Default, Clone, Debug)]
pub struct UpdateToken {
    pub status: Option<TokenStatus>,
    pub append_state_data_json: Option<Value>,
}

impl Token {
    /// Add token record
    pub async fn insert(params: NewToken, client: &Client) -> Result<uuid::Uuid, DBError> {
        const QUERY: &'static str = "
            INSERT INTO tokens (
                asset_state_id,
                initial_data_json,
                token_id
            ) VALUES ($1, $2, $3) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.asset_state_id,
                &params.initial_data_json,
                &params.token_id,
            ])
            .await?;

        Ok(result.get(0))
    }

    /// Update token into database
    ///
    /// Merges subset of fields with UpdateToken:
    /// - status
    /// - additional_data_json merged with UpdateToken::append_state_data_json
    // TODO: this is very expensive - think on optimization later
    pub async fn update(self, data: UpdateToken, instruction: &Instruction, client: &Client) -> Result<Self, DBError> {
        let mut token = Self::load(self.id, &client).await?;
        let state_data_json: Value = match data.append_state_data_json {
            Some(Object(mut update)) => {
                let mut obj = Map::<String, Value>::new();
                if let Some(previous) = token.additional_data_json.as_object_mut() {
                    obj.append(previous);
                }
                obj.append(&mut update);
                obj.into()
            },
            _ => token.additional_data_json.clone(),
        };
        let state = NewTokenStateAppendOnly {
            token_id: token.token_id.clone(),
            instruction_id: instruction.id,
            status: data.status.unwrap_or_else(|| token.status.clone()),
            state_data_json,
        };
        Self::store_append_only_state(&state, client).await?;
        Self::load(token.id, &client).await
    }

    /// Load token record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<Token, DBError> {
        let stmt = "SELECT * FROM tokens_view WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Token::from_row(result)?)
    }

    /// Find token record by token id
    pub async fn find_by_token_id(token_id: &TokenID, client: &Client) -> Result<Option<Token>, DBError> {
        const QUERY: &'static str = "SELECT * FROM tokens_view WHERE token_id = $1";
        let stmt = client.prepare(QUERY).await?;
        let result = client.query_opt(&stmt, &[&token_id]).await?;
        Ok(result.map(Self::from_row).transpose()?)
    }

    /// Find token records by asset state id
    pub async fn find_by_asset_state_id(asset_state_id: uuid::Uuid, client: &Client) -> Result<Vec<Token>, DBError> {
        const QUERY: &'static str = "SELECT * FROM tokens_view WHERE asset_state_id = $1";
        let stmt = client.prepare(QUERY).await?;
        let results = client.query(&stmt, &[&asset_state_id]).await?;
        Ok(results
            .into_iter()
            .map(Token::from_row)
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Store append only state
    ///
    /// NOTE: This call will not merge new values provided, they are stored as is
    pub async fn store_append_only_state(
        params: &NewTokenStateAppendOnly,
        client: &Client,
    ) -> Result<uuid::Uuid, DBError>
    {
        const QUERY: &'static str = "
            INSERT INTO token_state_append_only (
                token_id,
                state_data_json,
                instruction_id,
                status
            ) VALUES ($1, $2, $3, $4) RETURNING id";
        let stmt = client.prepare(QUERY).await?;
        let result = client
            .query_one(&stmt, &[
                &params.token_id,
                &params.state_data_json,
                &params.instruction_id,
                &params.status,
            ])
            .await?;

        Ok(result.get(0))
    }
}

impl<'a> ToSql for NewTokenStateAppendOnly {
    accepts!(JSON, JSONB);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        json!(self).to_sql(ty, w)
    }
}

impl<'a> FromSql<'a> for NewTokenStateAppendOnly {
    accepts!(JSON, JSONB);

    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(serde_json::from_value(
            Json::<Value>::from_sql(ty, raw).map(|json| json.0)?,
        )?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::{AssetState, InstructionStatus},
        test::utils::{
            builders::{consensus::InstructionBuilder, AssetStateBuilder, TokenBuilder},
            test_db_client,
            Test,
        },
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let asset2 = AssetStateBuilder::default().build(&client).await.unwrap();

        let params = NewToken {
            asset_state_id: asset.id,
            initial_data_json: json!({"value": true}),
            token_id: Test::from_asset(&asset.asset_id),
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await.unwrap();
        let token = Token::load(token_id, &client).await.unwrap();
        assert_eq!(token.asset_state_id, asset.id);
        assert_eq!(token.issue_number, 1);

        let params = NewToken {
            asset_state_id: asset.id,
            initial_data_json: json!({"value": true}),
            token_id: Test::from_asset(&asset.asset_id),
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await.unwrap();
        let token = Token::load(token_id, &client).await.unwrap();
        assert_eq!(token.asset_state_id, asset.id);
        assert_eq!(token.issue_number, 2);

        let params = NewToken {
            asset_state_id: asset2.id,
            initial_data_json: json!({"value": true}),
            token_id: Test::from_asset(&asset.asset_id),
            ..NewToken::default()
        };
        let token_id = Token::insert(params, &client).await.unwrap();
        let token = Token::load(token_id, &client).await.unwrap();
        assert_eq!(token.asset_state_id, asset2.id);
        assert_eq!(token.issue_number, 1);
    }

    #[actix_rt::test]
    async fn duplicate_token_id() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();

        let params = NewToken {
            asset_state_id: asset.id,
            initial_data_json: json!({"value": true}),
            token_id: Test::from_asset(&asset.asset_id),
            ..NewToken::default()
        };
        Token::insert(params.clone(), &client).await.unwrap();
        assert!(Token::insert(params, &client).await.is_err());
    }

    #[actix_rt::test]
    async fn find_by_asset_state_id() {
        let (client, _lock) = test_db_client().await;
        let token = TokenBuilder::default().build(&client).await.unwrap();
        let token2 = TokenBuilder::default().build(&client).await.unwrap();

        assert_eq!(
            vec![token.clone()],
            Token::find_by_asset_state_id(token.asset_state_id, &client)
                .await
                .unwrap()
        );
        assert_eq!(
            vec![token2.clone()],
            Token::find_by_asset_state_id(token2.asset_state_id, &client)
                .await
                .unwrap()
        );
    }

    #[actix_rt::test]
    async fn find_by_token_id() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();

        let params = NewToken {
            asset_state_id: asset.id,
            token_id: Test::from_asset(&asset.asset_id),
            ..NewToken::default()
        };
        let id = Token::insert(params.clone(), &client).await.unwrap();
        let token = Token::find_by_token_id(&params.token_id, &client)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(token.id, id);
    }

    #[actix_rt::test]
    async fn default_state() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();

        let params = NewToken {
            asset_state_id: asset.id,
            token_id: Test::from_asset(&asset.asset_id),
            ..NewToken::default()
        };
        let id = Token::insert(params, &client).await.unwrap();
        let token = Token::load(id, &client).await.unwrap();
        assert_eq!(token.status, TokenStatus::default());
    }

    #[actix_rt::test]
    async fn store_append_only_state() {
        let (client, _lock) = test_db_client().await;
        let initial_data = json!({"value": true, "value2": 4});
        let token = TokenBuilder {
            initial_data_json: initial_data.clone(),
            ..TokenBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        let asset = AssetState::load(token.asset_state_id, &client).await.unwrap();
        assert_eq!(json!(initial_data), token.initial_data_json);
        assert_eq!(json!(initial_data), token.additional_data_json);

        let instruction = InstructionBuilder {
            asset_id: Some(asset.asset_id.clone()),
            status: InstructionStatus::Commit,
            ..Default::default()
        }
        .build(&client)
        .await
        .unwrap();
        let empty_value: Option<String> = None;
        let state_data_json = json!({"value": empty_value.clone(), "value2": 8, "value3": 2});
        Token::store_append_only_state(
            &NewTokenStateAppendOnly {
                token_id: token.token_id,
                state_data_json: state_data_json.clone(),
                status: token.status,
                instruction_id: instruction.id.clone(),
            },
            &client,
        )
        .await
        .unwrap();
        let token = Token::load(token.id, &client).await.unwrap();
        assert_eq!(state_data_json, token.additional_data_json);

        let state_data_json = json!({"value": false, "value3": empty_value.clone()});
        Token::store_append_only_state(
            &NewTokenStateAppendOnly {
                token_id: token.token_id,
                state_data_json: state_data_json.clone(),
                status: token.status,
                instruction_id: instruction.id,
            },
            &client,
        )
        .await
        .unwrap();
        let token = Token::load(token.id, &client).await.unwrap();
        assert_eq!(state_data_json.clone(), token.additional_data_json);
    }

    #[actix_rt::test]
    async fn updates() {
        let (client, _lock) = test_db_client().await;
        let token: Token = TokenBuilder {
            initial_data_json: json!({"value": true, "value2": 4}),
            ..TokenBuilder::default()
        }
        .build(&client)
        .await
        .unwrap();
        let asset = AssetState::load(token.asset_state_id, &client).await.unwrap();
        let instruction = InstructionBuilder {
            asset_id: Some(asset.asset_id),
            ..Default::default()
        }
        .build(&client)
        .await
        .unwrap();

        let update = UpdateToken::default();
        let token2 = token.clone().update(update, &instruction, &client).await.unwrap();
        assert_eq!(token.id, token2.id);
        assert_eq!(token.status, token2.status);
        assert_eq!(token.additional_data_json, token2.additional_data_json);
        assert_eq!(token.asset_state_id, token2.asset_state_id);

        let update = UpdateToken {
            append_state_data_json: Some(json!({"append_initial": true})),
            ..UpdateToken::default()
        };
        let token = token.update(update, &instruction, &client).await.unwrap();
        assert_eq!(
            token.additional_data_json,
            json!({"value": true, "value2": 4, "append_initial": true})
        );
        assert_eq!(token.status, token2.status);

        let update = UpdateToken {
            append_state_data_json: Some(json!({"append_additional": true})),
            ..UpdateToken::default()
        };
        let token = token.update(update, &instruction, &client).await.unwrap();
        assert_eq!(
            token.additional_data_json,
            json!({"value": true, "value2": 4, "append_initial": true, "append_additional": true})
        );
        assert_eq!(token.status, token2.status);

        let update = UpdateToken {
            status: Some(TokenStatus::Retired),
            ..UpdateToken::default()
        };
        let token2 = token
            .clone()
            .update(update.clone(), &instruction, &client)
            .await
            .unwrap();
        assert_eq!(token2.status, TokenStatus::Retired);
        assert_eq!(token2.additional_data_json, token.additional_data_json);
    }
}
