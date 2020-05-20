use crate::{
    db::{models::ProposalStatus, utils::errors::DBError},
    types::{AssetID, NodeID},
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_pg_mapper::{FromTokioPostgresRow, PostgresMapper};
use tokio_postgres::types::Type;

#[derive(Deserialize, Serialize, PostgresMapper, PartialEq, Debug)]
#[pg_mapper(table = "signed_proposals")]
pub struct SignedProposal {
    pub id: uuid::Uuid,
    pub proposal_id: uuid::Uuid,
    pub node_id: NodeID,
    pub signature: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewSignedProposal {
    pub proposal_id: uuid::Uuid,
    pub node_id: NodeID,
    pub signature: String,
}

impl SignedProposal {
    pub async fn invalidate(signed_proposals: Vec<SignedProposal>, client: &Client) -> Result<(), DBError> {
        let signed_proposal_ids: Vec<uuid::Uuid> = signed_proposals.into_iter().map(|s| s.id).collect();

        const QUERY: &'static str = "
            UPDATE signed_proposals SET
                status = $2,
                updated_at = NOW()
            WHERE id in ($1)
            RETURNING *";
        let stmt = client.prepare_typed(QUERY, &[Type::UUID, Type::TEXT]).await?;
        client
            .execute(&stmt, &[&signed_proposal_ids, ProposalStatus::Invalid])
            .await?;

        Ok(())
    }

    pub async fn threshold_met(client: &Client) -> Result<HashMap<AssetID, Vec<SignedProposal>>, DBError> {
        // TODO: logic is currently hardcoded / stubbed for a committee of 1 so a single signed proposal meets the
        // threshold       we will need to iterate on this logic in the future to determine a viable threshold
        // dynamically by asset
        let stmt = "
            SELECT p.asset_id, sp.*
            FROM signed_proposals sp
            JOIN proposals p ON sp.proposal_id = p.id
            JOIN asset_states as ON as.asset_id = p.asset_id
            WHERE p.status = 'Signed'
            AND as.blocked_until <= now()
            ORDER BY p.asset_id
        ";
        let signed_proposal_data: Vec<(AssetID, SignedProposal)> = client
            .query(stmt, &[])
            .await?
            .into_iter()
            .map(|row| (SignedProposal::from_row(row), row.get(0)))
            .collect();

        let mut asset_id_signed_proposal_mapping = HashMap::new();
        for (asset_id, signed_proposal_data) in &signed_proposal_data.iter().group_by(|data| data.0) {
            asset_id_signed_proposal_mapping.insert(asset_id, signed_proposal_data.map(|d| d.1).collect_vec());
        }

        Ok(asset_id_signed_proposal_mapping)
    }

    pub async fn insert(params: NewSignedProposal, client: &Client) -> Result<Self, DBError> {
        const QUERY: &'static str = "
            INSERT INTO signed_proposals (
                proposal_id,
                node_id,
                signature
            ) VALUES ($1, $2, $3) RETURNING *";
        let stmt = client.prepare(QUERY).await?;
        let row = client
            .query_one(&stmt, &[&params.proposal_id, &params.node_id, &params.signature])
            .await?;
        Ok(Self::from_row(row)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::utils::{builders::ProposalBuilder, test_db_client};
    use serde_json::json;

    #[actix_rt::test]
    async fn crud() {
        let (client, _lock) = test_db_client().await;

        let proposal = ProposalBuilder::default().build(&client).await.unwrap();
        let params = NewSignedProposal {
            proposal_id: proposal.id,
            initiating_node_id: NodeID::stub(),
            signature: "stub-signature",
        };
        let signed_proposal = SignedProposal::insert(params, &client).await.unwrap();
        assert_eq!(signed_proposal.proposal_id, proposal.id);
        assert_eq!(signed_proposal.initiating_node_id, NodeID::stub());
    }
}
