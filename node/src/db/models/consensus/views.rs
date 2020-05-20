use crate::{
    db::{
        models::{NewAssetStateAppendOnly, NewTokenStateAppendOnly, ViewStatus},
        utils::errors::DBError,
    },
    types::{AssetID, NodeID},
};
use bytes::BytesMut;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, error::Error};
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::{
    types::{accepts, to_sql_checked, FromSql, IsNull, Json, ToSql, Type},
    Client,
};

#[derive(Deserialize, Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "views")]
pub struct View {
    pub id: uuid::Uuid,
    pub asset_id: AssetID,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub instruction_set: Vec<uuid::Uuid>,
    pub invalid_instruction_set: Vec<uuid::Uuid>,
    pub asset_state_append_only: Vec<NewAssetStateAppendOnly>,
    pub token_state_append_only: Vec<NewTokenStateAppendOnly>,
    pub status: ViewStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct NewView {
    pub asset_id: AssetID,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub instruction_set: Vec<uuid::Uuid>,
    pub invalid_instruction_set: Vec<uuid::Uuid>,
    pub asset_state_append_only: Vec<NewAssetStateAppendOnly>,
    pub token_state_append_only: Vec<NewTokenStateAppendOnly>,
}

/// Additional parameters that may supplied by the node but not serialized as part of a proposal
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct NewViewAdditionalParameters {
    pub status: Option<ViewStatus>,
    pub proposal_id: Option<uuid::Uuid>,
}

#[derive(Default, Clone, Debug)]
pub struct UpdateView {
    pub status: Option<ViewStatus>,
    pub proposal_id: Option<uuid::Uuid>,
}

impl View {
    pub async fn invalidate(views: Vec<View>, client: &Client) -> Result<(), DBError> {
        let view_ids: Vec<uuid::Uuid> = views.into_iter().map(|s| s.id).collect();

        const QUERY: &'static str = "
            UPDATE views SET
                status = $2,
                updated_at = NOW()
            WHERE id in ($1)
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::UUID, Type::TEXT]).await?;
        client.execute(&stmt, &[&view_ids, ViewStatus::Invalid]).await?;
    }

    pub async fn threshold_met(client: &Client) -> Result<HashMap<AssetID, Vec<View>>, DBError> {
        Ok(HashMap::new())
    }

    pub async fn insert(
        params: NewView,
        additional_params: NewViewAdditionalParameters,
        client: &Client,
    ) -> Result<Self, DBError>
    {
        const QUERY: &'static str = "
            INSERT INTO views (
                initiating_node_id,
                signature,
                instruction_set,
                invalid_instruction_set,
                asset_state_append_only,
                token_state_append_only,
                status,
                proposal_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *";
        let stmt = client
            .prepare_typed(QUERY, &[
                Type::BYTEA,
                Type::TEXT,
                Type::UUID,
                Type::UUID,
                Type::JSONB,
                Type::JSONB,
                Type::TEXT,
                Type::UUID,
            ])
            .await?;
        let row = client
            .query_one(&stmt, &[
                &params.initiating_node_id,
                &params.signature,
                &params.instruction_set,
                &params.invalid_instruction_set,
                &params.asset_state_append_only,
                &params.token_state_append_only,
                &additional_params.status,
                &additional_params.proposal_id,
            ])
            .await?;
        Ok(Self::from_row(row)?)
    }

    /// Update views state in the database
    ///
    /// Updates subset of fields:
    /// - status
    pub async fn update(self, data: UpdateView, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            UPDATE views SET
                status = COALESCE($2, status),
                proposal_id = COALESCE($3, proposal_id),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::UUID, Type::TEXT]).await?;
        let updated = client
            .query_one(&stmt, &[&self.id, &data.status, &data.proposal_id])
            .await?;
        Ok(Self::from_row(updated)?)
    }

    /// Marks set of views as given status
    pub async fn update_views_status(
        view_ids: Vec<uuid::Uuid>,
        status: ViewStatus,
        client: &Client,
    ) -> Result<(), DBError>
    {
        const QUERY: &'static str = "
            UPDATE views SET
                status = $2,
                updated_at = NOW()
            WHERE id in ($1)
            RETURNING *";
        let stmt = client
            .prepare_typed(QUERY, &[Type::UUID, Type::TEXT])
            .await?;
        client.execute(&stmt, &[&view_ids, &status]).await?;

        Ok(())
    }

    /// Load view record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<Self, DBError> {
        let stmt = "SELECT * FROM views WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Self::from_row(result)?)
    }

    pub async fn find_by_asset_status()
}

impl<'a> ToSql for NewView {
    accepts!(JSON, JSONB);

    to_sql_checked!();

    fn to_sql(&self, ty: &Type, w: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        json!(self).to_sql(ty, w)
    }
}

impl<'a> FromSql<'a> for NewView {
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
    use crate::test::utils::{
        builders::{AssetStateBuilder, InstructionBuilder},
        test_db_client,
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let params = NewView {
            asset_id: asset.asset_id,
            initiating_node_id: NodeID::stub(),
            signature: "stub-signature",
            instruction_set: Vec::new(),
            asset_state_append_only: Vec::new(),
            token_state_append_only: Vec::new(),
        };
        let view = View::insert(params, ViewStatus::Prepare, &client).await.unwrap();
        assert_eq!(view.asset_id, asset.asset_id);
        assert_eq!(view.initiating_node_id, asset.initiating_node_id);
        assert_eq!(view.status, ViewStatus::Prepare);

        let initial_updated_at = view.updated_at;
        let data = UpdateView {
            status: Some(ViewStatus::Commit),
            ..UpdateView::default()
        };
        let updated = view.update(data, &client).await.unwrap();
        assert_eq!(updated.status, ViewStatus::Commit);

        let view2 = View::load(updated.id, &client).await.unwrap();
        assert_eq!(view2.id, updated.id);
        assert_eq!(view2.asset_id, asset.asset_id);
        assert_eq!(view2.initiating_node_id, asset.initiating_node_id);
        assert_eq!(view2.status, ViewStatus::Commit);
        assert!(instruction2.updated_at > initial_updated_at);
    }
}
