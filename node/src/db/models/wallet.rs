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

    /// Update wallet's balance
    // TODO: the whole wallet thing might get info from base layer instead in the future...
    #[allow(dead_code)]
    pub async fn set_balance(&self, balance: i64, client: &Client) -> Result<Wallet, DBError> {
        const QUERY: &'static str = "UPDATE wallet SET updated_at = NOW(), balance = $2 WHERE id = $1 RETURNING *";
        let stmt = client.prepare(QUERY).await?;
        let row = client.query_one(&stmt, &[&self.id, &balance]).await?;
        Ok(Self::from_row(row)?)
    }
}

#[cfg(test)]
mod test {
    use super::{NewWallet, SelectWallet, Wallet};
    use crate::test::utils::{load_env, test_db_client};

    const PUBKEY: &'static str = "7e6f4b801170db0bf86c9257fe562492469439556cba069a12afd1c72c585b0f";

    #[actix_rt::test]
    async fn crud() {
        load_env();
        let (mut client, _lock) = test_db_client().await;

        let new_wallet_params = NewWallet {
            pub_key: PUBKEY.to_owned(),
            ..NewWallet::default()
        };
        let transaction = client.transaction().await.unwrap();
        let inserted = Wallet::insert(new_wallet_params, &transaction).await.unwrap();
        transaction.commit().await.unwrap();

        let query_inserted = SelectWallet {
            id: Some(inserted.id.clone()),
            ..SelectWallet::default()
        };
        let wallets = Wallet::select(query_inserted, &client).await.unwrap();
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].pub_key, PUBKEY.to_owned());

        let query_pub_key = SelectWallet {
            pub_key: Some(PUBKEY.to_owned()),
            ..SelectWallet::default()
        };
        let wallets = Wallet::select(query_pub_key, &client).await.unwrap();
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].id, inserted.id);
    }

    #[actix_rt::test]
    async fn transaction_abort() {
        load_env();
        let (mut client, _lock) = test_db_client().await;

        let new_wallet_params = NewWallet {
            pub_key: PUBKEY.to_owned(),
            ..NewWallet::default()
        };
        let transaction = client.transaction().await.unwrap();
        let inserted = Wallet::insert(new_wallet_params, &transaction).await.unwrap();
        drop(transaction);

        let query_inserted = SelectWallet {
            id: Some(inserted.id.clone()),
            ..SelectWallet::default()
        };
        let wallets = Wallet::select(query_inserted, &client).await.unwrap();
        assert_eq!(wallets.len(), 0);
    }

    #[actix_rt::test]
    async fn insert_duplicate() {
        load_env();
        let (mut client, _lock) = test_db_client().await;

        let new_wallet_params = NewWallet {
            pub_key: PUBKEY.to_owned(),
            ..NewWallet::default()
        };

        let transaction = client.transaction().await.unwrap();
        let inserted1 = Wallet::insert(new_wallet_params.clone(), &transaction).await.unwrap();
        transaction.commit().await.unwrap();
        let transaction = client.transaction().await.unwrap();
        let inserted2 = Wallet::insert(new_wallet_params.clone(), &transaction).await.unwrap();
        transaction.commit().await.unwrap();
        assert_eq!(inserted1.id, inserted2.id);

        let query_pub_key = SelectWallet {
            pub_key: Some(PUBKEY.to_owned()),
            ..SelectWallet::default()
        };
        let wallets = Wallet::select(query_pub_key, &client).await.unwrap();
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].id, inserted1.id);
    }

    #[actix_rt::test]
    async fn set_balance() {
        load_env();
        let (mut client, _lock) = test_db_client().await;

        let new_wallet_params = NewWallet {
            pub_key: PUBKEY.to_owned(),
            ..NewWallet::default()
        };

        let transaction = client.transaction().await.unwrap();
        let wallet = Wallet::insert(new_wallet_params.clone(), &transaction).await.unwrap();
        transaction.commit().await.unwrap();
        assert_eq!(wallet.balance, 0);
        wallet.set_balance(100, &client).await.unwrap();
        let wallet = Wallet::select_by_key(&wallet.pub_key, &client).await.unwrap();
        assert_eq!(wallet.balance, 100);
    }
}
