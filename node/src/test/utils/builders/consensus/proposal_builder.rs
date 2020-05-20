use super::AssetStateBuilder;
use crate::{db::models::*, types::TemplateID};
use chrono::Local;
use deadpool_postgres::Client;
use serde_json::{json, Value};
use uuid::{
    v1::{Context, Timestamp},
    Uuid,
};

#[allow(dead_code)]
pub struct ProposalBuilder {
    id: Option<Uuid>,
    new_view: Option<NewView>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for ProposalBuilder {
    fn default() -> Self {
        let x: u32 = random();
        Self {
            id: None,
            new_view: None,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl ProposalBuilder {
    pub async fn build(self, client: &Client) -> anyhow::Result<Proposal> {
        let id = match self.id {
            Some(id) => id,
            None => {
                let time = Local::now();
                let context: Context = Context::new(1);
                let ts = Timestamp::from_unix(&*context, time.timestamp() as u64, time.timestamp_subsec_nanos());
                Uuid::new_v1(ts, &node_id)?
            },
        };
        let new_view = match self.new_view {
            Some(new_view) => new_view,
            None => ViewBuilder::default().prepare(client).await?,
        };

        let params = NewProposal { id, new_view, node_id: NodeID::stub().inner(), asset_id: new_view.asset_id };
        let proposal_id = Proposal::insert(params, client).await?;
        Ok(Proposal::load(proposal_id, client).await?)
    }
}
