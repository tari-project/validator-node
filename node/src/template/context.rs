use crate::types::TemplateID;
use deadpool_postgres::{Client, Pool, Transaction};
use actix_web::{HttpRequest, dev::Payload, FromRequest};
use futures::future::{LocalBoxFuture, FutureExt, err};
use crate::api::utils::errors::ApiError;
use crate::db::utils::errors::DBError;
use crate::db::models::{NewToken, Token, UpdateToken, AssetState};
use crate::types::{TokenID, AssetID};

pub struct TemplateContext {
    pub template_id: TemplateID,
    client: Client,
//TODO:    transaction: Transaction,
}

impl TemplateContext {
    pub async fn create_token(&self, data: NewToken) -> Result<Token, DBError> {
        let id = Token::insert(data, self.client).await?;
        Token::load(id, self.client).await?
    }
    pub async fn update_token(&self, id: TokenID, data: UpdateToken) -> Result<Token, DBError> {
        Token::insert(data, self.client).await
    }
}

impl FromRequest for TemplateContext {
    type Config = ();
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // initialize whole context in this module - would make moduel quite complex but easier to debug
        // middleware might pass parameters via .extensions() or .app_data()
        let pool = req.app_data::<Pool>()
            .expect("Failed to retrieve DB pool");
        let template_id = match req.app_data::<TemplateID>() {
            Some(id) => id.clone(),
            None => return err(ApiError::bad_request("Template data not found by this path"))
                .boxed_local(),
        };

        pool.get()
            .map(|res| res
                .map(|client| TemplateContext { client, template_id })
                .map_err(|err| DBError::from(err).into())
            )
            .boxed_local()
    }
}