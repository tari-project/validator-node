use super::{Contracts, Template};
use crate::{
    types::{AssetID, TemplateID, TokenID},
};
use actix_web::web;
use anyhow::Result;

pub(crate) struct AssetCallParams(String, String);
impl AssetCallParams {
    pub fn asset_id(&self, tpl: &TemplateID) -> Result<AssetID> {
        let template_id = tpl.to_hex();
        Ok(format!("{}{}.{}", template_id, self.0, self.1).parse()?)
    }
}

pub(crate) struct TokenCallParams(String, String, String);
impl TokenCallParams {
    pub fn token_id(&self, tpl: &TemplateID) -> Result<TokenID> {
        let template_id = tpl.to_hex();
        Ok(format!("{}{}.{}{}", template_id, self.0, self.1, self.2).parse()?)
    }

    pub fn asset_id(&self, tpl: &TemplateID) -> Result<AssetID> {
        AssetCallParams(self.0, self.1).asset_id(tpl)
    }
}

pub fn install_template<T: Template>(tpl: T, app: &mut web::ServiceConfig) {
    let mut scope = web::scope(format!("/asset_call/{}/{{features}}/{{hash}}/", T::id()).as_str()).app_data(T::id());
    let mut scope = scope.configure(<T::AssetContracts as Contracts>::setup_actix_routes);
    app.service(scope);
    let mut scope =
        web::scope(format!("/token_call/{}/{{features}}/{{hash}}/{{id}}", T::id()).as_str()).app_data(T::id());
    let mut scope = scope.configure(<T::TokenContracts as Contracts>::setup_actix_routes);
    app.service(scope);
}
