use super::{Contracts, Template};
use crate::types::{AssetID, TemplateID, TokenID};
use actix_web::web;
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct AssetCallParams {
    raid_id: String,
    features: String,
    hash: String,
}
impl AssetCallParams {
    pub fn asset_id(&self, tpl: &TemplateID) -> Result<AssetID> {
        let template_id = tpl.to_hex();
        Ok(format!("{}{}{}.{}", template_id, self.features, self.raid_id, self.hash).parse()?)
    }
}

#[derive(Deserialize)]
pub(crate) struct TokenCallParams {
    raid_id: String,
    features: String,
    hash: String,
    uid: String,
}
impl TokenCallParams {
    pub fn token_id(&self, tpl: &TemplateID) -> Result<TokenID> {
        Ok(format!("{}{}", self.asset_id(tpl)?, self.uid).parse()?)
    }

    pub fn asset_id(&self, tpl: &TemplateID) -> Result<AssetID> {
        AssetCallParams::from(self).asset_id(tpl)
    }
}
impl From<&TokenCallParams> for AssetCallParams {
    fn from(token: &TokenCallParams) -> Self {
        AssetCallParams {
            raid_id: token.raid_id.clone(),
            features: token.features.clone(),
            hash: token.hash.clone(),
        }
    }
}

pub fn install_template<T: Template>(app: &mut web::ServiceConfig) {
    app.service(
        web::scope(format!("/asset_call/{}/{{features}}/{{raid_id}}/{{hash}}/", T::id()).as_str())
            .app_data(T::id())
            .configure(<T::AssetContracts as Contracts>::setup_actix_routes),
    );

    app.service(
        web::scope(format!("/token_call/{}/{{features}}/{{raid_id}}/{{hash}}/{{uid}}", T::id()).as_str())
            .app_data(T::id())
            .configure(<T::TokenContracts as Contracts>::setup_actix_routes),
    );
}
