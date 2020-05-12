use crate::types::{AssetID, TokenID};
use crate::db::models::{AssetState, Token};
use super::{Template, TemplateContext};
use serde::Deserialize;
use anyhow::Result;
use actix_web::{web, App};


pub struct AssetCallParams(String,String);
impl AssetCallParams {
    pub async fn asset_state<'a>(&self, context: &'a TemplateContext) -> Result<AssetState> {
        let template_id = context.template_id.to_hex();
        let id: AssetID = format!("{}{}.{}", template_id, self.0, self.1).parse()?;
        Ok(context.load_asset(id).await)
    }
}

pub struct TokenCallParams(String,String,String);
impl TokenCallParams {
    pub async fn token<'a>(&self, context: &'a TemplateContext) -> Result<Token> {
        let template_id = context.template_id.to_hex();
        let id: TokenID = format!("{}{}.{}.{}", template_id, self.0, self.1, self.2).into();
        Ok(context.load_token(id).await)
    }
}

pub fn install_template<T: Template>(tpl: T, app: &mut web::ServiceConfig) {
    let mut scope = web::scope(format!("/asset_call/{}/{{features}}/{{hash}}/", tpl.id()));
    let mut scope = T::AssetContracts::configure_routes(scope);
    let mut scope = web::scope(format!("/token_call/{}/{{features}}/{{hash}}/{{id}}", tpl.id()));
    let mut scope = T::AssetContracts::configure_routes(scope);
    app.service(scope);
}
