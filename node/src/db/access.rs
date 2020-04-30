use super::errors::DBError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use tari_wallet::util::emoji::EmojiId;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::{types::Type, Client};

/// Access records for tari validation node
#[derive(Debug, Clone, Serialize, PostgresMapper)]
#[pg_mapper(table = "access")]
pub struct Access {
    pub id: uuid::Uuid,
    pub pub_key: String,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
impl Display for Access {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let emoji = self.emoji_id();
        match emoji {
            Ok(emoji) => write!(f, "{} | {}", self.pub_key, emoji),
            Err(err) => write!(f, "{}: {:?}", self.pub_key, err),
        }
    }
}

/// Query paramteres for adding new user access record
#[derive(Default, Clone, Debug)]
pub struct NewAccess {
    pub pub_key: String,
}

/// Query paramteres for searching access records
#[derive(Default, Clone, Debug)]
pub struct SelectAccess {
    pub id: Option<uuid::Uuid>,
    pub pub_key: Option<String>,
    pub include_deleted: Option<bool>,
}

impl Access {
    /// Return EmojiId for the user record
    pub fn emoji_id(&self) -> Result<EmojiId, DBError> {
        Ok(EmojiId::from_hex(&self.pub_key)?)
    }

    /// Add access record
    pub async fn grant(params: NewAccess, client: &Client) -> Result<u64, DBError> {
        let select_existing_user = SelectAccess {
            id: None,
            pub_key: Some(params.pub_key.clone()),
            include_deleted: Some(true),
        };
        let user_exists = Access::select(select_existing_user.clone(), client).await?;
        if user_exists.len() == 1 {
            // Reinstate the user
            Ok(Access::reinstate(select_existing_user, client).await?)
        } else {
            const QUERY: &'static str = "INSERT INTO access (pub_key) VALUES ($1)";
            let stmt = client.prepare(QUERY).await?;
            Ok(client.execute(&stmt, &[&params.pub_key]).await?)
        }
    }

    /// Search active access records by [`SelectAccessQuery`]
    pub async fn select(params: SelectAccess, client: &Client) -> Result<Vec<Access>, DBError> {
        const QUERY: &'static str = "SELECT * FROM access WHERE ($1 IS NULL OR id = $1) AND ($2 IS NULL OR pub_key = \
                                     $2) AND ($3 = true OR deleted_at IS NULL)";

        let stmt = client
            .prepare_typed(QUERY, &[Type::UUID, Type::TEXT, Type::BOOL])
            .await?;
        Ok(client
            .query(&stmt, &[
                &params.id,
                &params.pub_key,
                &params.include_deleted.unwrap_or(false),
            ])
            .await?
            .into_iter()
            .map(|row| Access::from_row(row))
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Revoke access record
    pub async fn revoke(params: SelectAccess, client: &Client) -> Result<u64, DBError> {
        const QUERY: &'static str = "UPDATE access SET deleted_at = NOW(), updated_at = NOW() WHERE ($1 IS NULL OR id \
                                     = $1) AND ($2 IS NULL OR pub_key = $2)";
        if params.id.is_none() && params.pub_key.is_none() {
            return Err(DBError::bad_query("Revoke access query requires id or pub_key"));
        }
        let stmt = client.prepare_typed(QUERY, &[Type::UUID, Type::TEXT]).await?;
        Ok(client.execute(&stmt, &[&params.id, &params.pub_key]).await?)
    }

    /// Re-instate access record
    async fn reinstate(params: SelectAccess, client: &Client) -> Result<u64, DBError> {
        const QUERY: &'static str = "UPDATE access SET deleted_at = NULL, updated_at = NOW()  WHERE ($1 IS NULL OR id \
                                     = $1) AND ($2 IS NULL OR pub_key = $2)";
        if params.id.is_none() && params.pub_key.is_none() {
            return Err(DBError::bad_query("Revoke access query requires id or pub_key"));
        }
        let stmt = client.prepare_typed(QUERY, &[Type::UUID, Type::TEXT]).await?;
        Ok(client.execute(&stmt, &[&params.id, &params.pub_key]).await?)
    }
}

#[cfg(test)]
mod test {
    use super::{Access, NewAccess, SelectAccess};
    use crate::test_utils::{build_test_config, build_test_pool, reset_db};
    use chrono::Utc;

    const PUBKEY: &'static str = "7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0f";
    const EMOJI: &'static str = "🍉🐭👄🍎🙃🐇💻🙄🆘🐫🍫👕🎌👔👽🍫🤝🍷👤💫🐫🌈😍⛺🤑🛸🎤🎾🤴👖🧦😛📡";

    #[test]
    fn emoji() {
        let access = Access {
            id: uuid::Uuid::nil(),
            pub_key: PUBKEY.to_owned(),
            deleted_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(access.emoji_id().unwrap().to_string(), EMOJI.to_owned());
    }

    #[actix_rt::test]
    async fn crud() -> anyhow::Result<()> {
        dotenv::dotenv().unwrap();
        let db = build_test_pool().unwrap();
        let config = build_test_config().unwrap();
        reset_db(&config, &db).await.unwrap();
        let client = db.get().await.unwrap();

        let new_access_params = NewAccess {
            pub_key: PUBKEY.to_owned(),
        };
        let inserted = Access::grant(new_access_params.clone(), &client).await?;
        assert_eq!(inserted, 1);

        let query_exclude_deleted = SelectAccess {
            id: None,
            pub_key: Some(PUBKEY.to_owned()),
            include_deleted: None,
        };
        let access = Access::select(query_exclude_deleted.clone(), &client).await?;
        assert_eq!(access.len(), 1);
        assert_eq!(access[0].pub_key, PUBKEY.to_owned());

        let deleted = Access::revoke(query_exclude_deleted.clone(), &client).await?;
        assert_eq!(deleted, 1);

        let access = Access::select(query_exclude_deleted.clone(), &client).await?;
        assert_eq!(access.len(), 0);

        let query_include_deleted = SelectAccess {
            id: None,
            pub_key: Some(PUBKEY.to_owned()),
            include_deleted: Some(true),
        };
        let access = Access::select(query_include_deleted.clone(), &client).await?;
        assert_eq!(access.len(), 1);

        let reinstated = Access::grant(new_access_params, &client).await?;
        assert_eq!(reinstated, 1);

        let access = Access::select(query_exclude_deleted, &client).await?;
        assert_eq!(access.len(), 1);
        Ok(())
    }

    #[actix_rt::test]
    async fn delete_constraints() {
        dotenv::dotenv().unwrap();
        let db = build_test_pool().unwrap();
        let client = db.get().await.unwrap();
        let res = Access::revoke(
            SelectAccess {
                pub_key: None,
                id: None,
                include_deleted: None,
            },
            &client,
        )
        .await;
        assert!(res.is_err());
    }
}
