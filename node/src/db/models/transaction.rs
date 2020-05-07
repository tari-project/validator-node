pub use super::TransactionStatus;
use crate::{db::utils::errors::DBError, types::TemplateID};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use serde::Serialize;
use serde_json::Value;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::Type;

#[derive(Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "contract_transactions")]
pub struct ContractTransaction {
    pub id: uuid::Uuid,
    pub asset_state_id: uuid::Uuid,
    pub token_id: Option<uuid::Uuid>,
    pub template_id: TemplateID,
    pub contract_name: String,
    pub status: TransactionStatus,
    pub params: Value,
    pub result: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Query paramteres for adding new asset record
#[derive(Default, Clone, Debug)]
pub struct NewContractTransaction {
    pub asset_state_id: uuid::Uuid,
    pub token_id: Option<uuid::Uuid>,
    pub template_id: TemplateID,
    pub contract_name: String,
    pub status: TransactionStatus,
    pub params: Value,
    pub result: Value,
}

impl ContractTransaction {
    /// Add digital asset record
    pub async fn insert(params: NewContractTransaction, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            INSERT INTO contract_transactions (
                asset_state_id,
                token_id,
                template_id,
                contract_name,
                status,
                params,
                result
            ) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *";
        let stmt = client
            .prepare_typed(QUERY, &[
                Type::UUID,
                Type::UUID,
                TemplateID::SQL_TYPE,
                Type::TEXT,
                Type::TEXT,
                Type::JSONB,
                Type::JSONB,
            ])
            .await?;
        let row = client
            .query_one(&stmt, &[
                &params.asset_state_id,
                &params.token_id,
                &params.template_id,
                &params.contract_name,
                &params.status,
                &params.params,
                &params.result,
            ])
            .await?;
        Ok(Self::from_row(row)?)
    }

    /// Update transaction state in the database
    ///
    /// Updates subset of fields:
    /// - asset_uid
    /// - token_uid
    /// - status
    /// - result
    pub async fn update(&self, client: &Client) -> Result<u64, DBError> {
        const QUERY: &'static str = "
            UPDATE contract_transactions SET
                asset_state_id = $2,
                token_id = $3,
                status = $4,
                result = $5,
                updated_at = NOW()
            WHERE id = $1";
        let stmt = client
            .prepare_typed(QUERY, &[Type::UUID, Type::UUID, Type::UUID, Type::TEXT, Type::JSONB])
            .await?;
        let updated = client
            .execute(&stmt, &[
                &self.id,
                &self.asset_state_id,
                &self.token_id,
                &self.status,
                &self.result,
            ])
            .await?;
        Ok(updated)
    }

    /// Load transaction record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<Self, DBError> {
        let stmt = "SELECT * FROM contract_transactions WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Self::from_row(result)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{
        builders::{AssetStateBuilder, TokenBuilder},
        test_db_client,
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let params = NewContractTransaction {
            asset_state_id: asset.id,
            template_id: asset.asset_id.template_id(),
            contract_name: "test_contract".into(),
            params: json!({"test_param": 1}),
            ..NewContractTransaction::default()
        };
        let mut transaction = ContractTransaction::insert(params, &client).await.unwrap();
        assert_eq!(transaction.template_id, asset.asset_id.template_id());
        assert_eq!(transaction.params, json!({"test_param": 1}));
        assert!(transaction.result.is_null());
        assert_eq!(transaction.status, TransactionStatus::Prepare);

        let asset2 = AssetStateBuilder::default().build(&client).await.unwrap();
        let token = TokenBuilder::default().build(&client).await.unwrap();
        transaction.asset_state_id = asset2.id;
        transaction.token_id = Some(token.id);
        transaction.status = TransactionStatus::Commit;
        transaction.result = json!({"test_result": "success"});
        let updated = transaction.update(&client).await.unwrap();
        assert_eq!(updated, 1);

        let transaction2 = ContractTransaction::load(transaction.id, &client).await.unwrap();
        assert_eq!(transaction2.id, transaction.id);
        assert_eq!(transaction2.asset_state_id, transaction.asset_state_id);
        assert_eq!(transaction2.token_id, transaction.token_id);
        assert_eq!(transaction2.result, transaction.result);
        assert_eq!(transaction2.status, transaction.status);
        assert!(transaction2.updated_at > transaction.updated_at);
    }
}
