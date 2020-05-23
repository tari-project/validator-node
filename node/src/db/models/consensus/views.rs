use crate::{
    db::{models::ViewStatus, utils::errors::DBError},
    types::{consensus::AppendOnlyState, AssetID, NodeID, ProposalID},
};
use bytes::BytesMut;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, error::Error};
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::{
    types::{accepts, to_sql_checked, FromSql, IsNull, Json, ToSql, Type},
    Client,
};

#[derive(Deserialize, Serialize, Clone, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "views")]
pub struct View {
    pub id: uuid::Uuid,
    pub asset_id: AssetID,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub instruction_set: Vec<uuid::Uuid>,
    pub invalid_instruction_set: Vec<uuid::Uuid>,
    pub append_only_state: AppendOnlyState,
    pub status: ViewStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub proposal_id: Option<ProposalID>,
}

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct NewView {
    pub asset_id: AssetID,
    pub initiating_node_id: NodeID,
    pub signature: String,
    pub instruction_set: Vec<uuid::Uuid>,
    pub invalid_instruction_set: Vec<uuid::Uuid>,
    pub append_only_state: AppendOnlyState,
}

/// Additional parameters that may supplied by the node but not serialized as part of a proposal
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct NewViewAdditionalParameters {
    pub status: Option<ViewStatus>,
    pub proposal_id: Option<ProposalID>,
}

#[derive(Default, Clone, Debug)]
pub struct UpdateView {
    pub status: Option<ViewStatus>,
    pub proposal_id: Option<ProposalID>,
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
        client.execute(&stmt, &[&view_ids, &ViewStatus::Invalid]).await?;
        Ok(())
    }

    pub async fn threshold_met(client: &Client) -> Result<HashMap<AssetID, Vec<View>>, DBError> {
        // TODO: logic is currently hardcoded / stubbed for a committee of 1 so a single view meets the
        // threshold... we will need to iterate on this logic in the future to determine a viable threshold
        // dynamically by asset
        let stmt = "
            SELECT v.*
            FROM views v
            JOIN asset_states ast ON ast.asset_id = v.asset_id
            WHERE v.status = 'Prepare'
            AND ast.blocked_until <= now()
            ORDER BY v.asset_id
        ";

        let mut asset_id_view_mapping = HashMap::new();
        let views: Vec<View> = client
            .query(stmt, &[])
            .await?
            .into_iter()
            .map(|v| View::from_row(v))
            .collect::<Result<Vec<_>, _>>()?;

        for (asset_id, views) in &views.into_iter().group_by(|view| view.asset_id.clone()) {
            asset_id_view_mapping.insert(asset_id.clone(), views.collect_vec().clone());
        }

        Ok(asset_id_view_mapping)
    }

    pub async fn insert(
        params: NewView,
        additional_params: NewViewAdditionalParameters,
        client: &Client,
    ) -> Result<Self, DBError>
    {
        const QUERY: &'static str = "
            INSERT INTO views (
                asset_id,
                initiating_node_id,
                signature,
                instruction_set,
                invalid_instruction_set,
                append_only_state,
                status,
                proposal_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *";
        let stmt = client
            .prepare_typed(QUERY, &[
                AssetID::SQL_TYPE,
                Type::BYTEA,
                Type::TEXT,
                Type::UUID_ARRAY,
                Type::UUID_ARRAY,
                Type::JSONB,
                Type::TEXT,
            ])
            .await?;
        let row = client
            .query_one(&stmt, &[
                &params.asset_id,
                &params.initiating_node_id,
                &params.signature,
                &params.instruction_set,
                &params.invalid_instruction_set,
                &params.append_only_state,
                &additional_params.status.unwrap_or(ViewStatus::Prepare),
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
                proposal_id = COALESCE($3::\"ProposalID\", proposal_id),
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
        view_ids: &[uuid::Uuid],
        status: ViewStatus,
        client: &Client,
    ) -> Result<(), DBError>
    {
        const QUERY: &'static str = "
            UPDATE views SET
                status = $2,
                updated_at = NOW()
            WHERE id = ANY ($1)
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::UUID_ARRAY, Type::TEXT]).await?;
        client.execute(&stmt, &[&view_ids, &status]).await?;

        Ok(())
    }

    /// Load view record
    pub async fn load(id: uuid::Uuid, client: &Client) -> Result<Self, DBError> {
        let stmt = "SELECT * FROM views WHERE id = $1";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Self::from_row(result)?)
    }

    /// Load view record
    pub async fn load_for_proposal(id: ProposalID, client: &Client) -> Result<Self, DBError> {
        let stmt = "SELECT * FROM views WHERE proposal_id = $1::\"ProposalID\"";
        let result = client.query_one(stmt, &[&id]).await?;
        Ok(Self::from_row(result)?)
    }

    pub async fn find_by_asset_status(
        asset_id: AssetID,
        status: ViewStatus,
        client: &Client,
    ) -> Result<Vec<View>, DBError>
    {
        const QUERY: &'static str = "SELECT * FROM views WHERE asset_id = $1 and status = $2";

        let stmt = client.prepare_typed(QUERY, &[Type::UUID, Type::TEXT]).await?;
        Ok(client
            .query(&stmt, &[&asset_id, &status])
            .await?
            .into_iter()
            .map(|row| View::from_row(row))
            .collect::<Result<Vec<_>, _>>()?)
    }
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

impl From<View> for NewView {
    fn from(view: View) -> Self {
        NewView {
            asset_id: view.asset_id,
            initiating_node_id: view.initiating_node_id,
            signature: view.signature.to_owned(),
            instruction_set: view.instruction_set.to_owned(),
            invalid_instruction_set: view.invalid_instruction_set.to_owned(),
            append_only_state: AppendOnlyState {
                asset_state: view.append_only_state.asset_state.to_owned(),
                token_state: view.append_only_state.token_state.to_owned(),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::utils::{builders::AssetStateBuilder, test_db_client};

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let params = NewView {
            asset_id: asset.asset_id.clone(),
            initiating_node_id: NodeID::stub(),
            signature: "stub-signature".to_string(),
            instruction_set: Vec::new(),
            invalid_instruction_set: Vec::new(),
            append_only_state: AppendOnlyState {
                asset_state: Vec::new(),
                token_state: Vec::new(),
            },
        };
        let view = View::insert(params, NewViewAdditionalParameters::default(), &client)
            .await
            .unwrap();
        assert_eq!(view.asset_id, asset.asset_id);
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
        assert_eq!(view2.asset_id, updated.asset_id);
        assert_eq!(view2.status, ViewStatus::Commit);
        assert!(view2.updated_at > initial_updated_at);
    }
}
