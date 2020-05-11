use crate::types::AssetID;
use super::errors::TemplateError;
use deadpool_postgres::Client as DbClient;
use serde::Deserialize;

#[derive(Deserialize)]
struct AssetCallParams {
    #[serde(with = "serde_with::rust::display_fromstr")]
    id: AssetID,
    contract_name: String,
}
impl AssetCallParams {
    pub async fn asset_state(&self, db: &DbClient) -> Result<AssetState, TemplateError> {
        AssetState::find_by_asset_id(self.id.to_string()).await
    }
}

// without supplying template id in the url we can't provide handlers beforehand
#[post(/asset_call/{template_id}/{features}/{hash}/contract)]
pub async fn asset_call_wrapper(params: Path<(AssetCallParams)>, db: DbClient) -> Result<impl Responder, TemplateError> {
    let asset = params.asset_state(&db).await?;
    let template = templates.get(asset.template_id);
    template.call()
}

use actix_web::{web, App};

async fn install_template<T: Template>(tpl: T, app: App) {
    let mut scope = web::scope(format!("/asset_call/{}/\{features\}/\{hash\}/", tpl.id()));
    let mut scope = T::AssetContracts::configure_routes(scope);
    app.service(scope);
}
