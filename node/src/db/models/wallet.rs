use crate::db::utils::errors::DBError;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Transaction};
use serde::{Deserialize, Serialize};
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::Type;

/// Wallet records registered for tari validation node
/// This model has only referential information about wallet
/// [Wallet] should be used instead
#[derive(Serialize, Deserialize, Debug, Clone, PostgresMapper)]
#[pg_mapper(table = "wallet")]
pub(crate) struct Wallet {
    pub id: uuid::Uuid,
    pub pub_key: String,
    pub balance: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Query paramteres for adding new wallet record
#[derive(Default, Clone, Debug)]
pub(crate) struct NewWallet {
    pub pub_key: String,
    pub name: String,
}

/// Query paramteres for searching wallet records
#[derive(Default, Clone, Debug)]
pub(crate) struct SelectWallet {
    pub id: Option<uuid::Uuid>,
    pub pub_key: Option<String>,
    pub name: Option<String>,
}

impl Wallet {
    /// Add wallet record
    pub async fn insert<'t>(params: NewWallet, client: &Transaction<'t>) -> Result<Wallet, DBError> {
        const QUERY: &'static str = "INSERT INTO wallet (pub_key, name) VALUES ($1,$2)
            ON CONFLICT (pub_key) DO UPDATE SET updated_at = NOW() RETURNING *";
        let stmt = client.prepare(QUERY).await?;
        Ok(client
            .query_one(&stmt, &[&params.pub_key, &params.name])
            .await
            .map(|row| Wallet::from_row(row))??)
    }

    /// Search wallet records by [`SelectWallet`]
    pub async fn select(params: SelectWallet, client: &Client) -> Result<Vec<Wallet>, DBError> {
        const QUERY: &'static str = "SELECT * FROM wallet WHERE ($1 IS NULL OR id = $1) AND ($2 IS NULL OR pub_key = \
                                     $2) AND ($3 IS NULL OR name = $3)";

        let stmt = client
            .prepare_typed(QUERY, &[Type::UUID, Type::TEXT, Type::TEXT])
            .await?;
        Ok(client
            .query(&stmt, &[&params.id, &params.pub_key, &params.name])
            .await?
            .into_iter()
            .map(|row| Wallet::from_row(row))
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Search wallet records by wallet's public key
    pub async fn select_by_key(pubkey: &String, client: &Client) -> Result<Wallet, DBError> {
        const QUERY: &'static str = "SELECT * FROM wallet WHERE pub_key = $1";

        let stmt = client.prepare_typed(QUERY, &[Type::TEXT]).await?;
        Ok(client
            .query_one(&stmt, &[pubkey])
            .await
            .map(|row| Wallet::from_row(row))??)
    }
}

#[cfg(test)]
mod test {
    use super::{NewWallet, SelectWallet, Wallet};
    use crate::test::utils::{load_env, test_db_client};

    const PUBKEY: &'static str = "7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0f";

    #[actix_rt::test]
    async fn crud() -> anyhow::Result<()> {
        load_env();
        let (mut client, _lock) = test_db_client().await;

        let new_wallet_params = NewWallet {
            pub_key: PUBKEY.to_owned(),
            ..NewWallet::default()
        };
        let transaction = client.transaction().await?;
        let inserted = Wallet::insert(new_wallet_params, &transaction).await?;
        transaction.commit().await?;

        let query_inserted = SelectWallet {
            id: Some(inserted.id.clone()),
            ..SelectWallet::default()
        };
        let wallets = Wallet::select(query_inserted, &client).await?;
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].pub_key, PUBKEY.to_owned());

        let query_pub_key = SelectWallet {
            pub_key: Some(PUBKEY.to_owned()),
            ..SelectWallet::default()
        };
        let wallets = Wallet::select(query_pub_key, &client).await?;
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].id, inserted.id);

        Ok(())
    }

    #[actix_rt::test]
    async fn transaction_abort() -> anyhow::Result<()> {
        load_env();
        let (mut client, _lock) = test_db_client().await;

        let new_wallet_params = NewWallet {
            pub_key: PUBKEY.to_owned(),
            ..NewWallet::default()
        };
        let transaction = client.transaction().await?;
        let inserted = Wallet::insert(new_wallet_params, &transaction).await?;
        drop(transaction);

        let query_inserted = SelectWallet {
            id: Some(inserted.id.clone()),
            ..SelectWallet::default()
        };
        let wallets = Wallet::select(query_inserted, &client).await?;
        assert_eq!(wallets.len(), 0);
        Ok(())
    }

    #[actix_rt::test]
    async fn insert_duplicate() -> anyhow::Result<()> {
        load_env();
        let (mut client, _lock) = test_db_client().await;

        let new_wallet_params = NewWallet {
            pub_key: PUBKEY.to_owned(),
            ..NewWallet::default()
        };

        let transaction = client.transaction().await?;
        let inserted1 = Wallet::insert(new_wallet_params.clone(), &transaction).await?;
        transaction.commit().await?;
        let transaction = client.transaction().await?;
        let inserted2 = Wallet::insert(new_wallet_params.clone(), &transaction).await?;
        transaction.commit().await?;
        assert_eq!(inserted1.id, inserted2.id);

        let query_pub_key = SelectWallet {
            pub_key: Some(PUBKEY.to_owned()),
            ..SelectWallet::default()
        };
        let wallets = Wallet::select(query_pub_key, &client).await?;
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].id, inserted1.id);

        Ok(())
    }
}
