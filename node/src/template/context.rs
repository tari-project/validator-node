//! TemplateContext provides access to contextual request data
//!
//! TemplateContext is always supplied as first parameter to Smart Contract implementation

use crate::{
    api::utils::errors::ApiError,
    db::{
        models::{AssetState, NewToken, Token},
        utils::errors::DBError,
    },
    types::{AssetID, TemplateID, TokenID},
};
use actix_web::{dev::Payload, FromRequest, HttpRequest, web::Data};
use deadpool_postgres::{Client, Pool, Transaction};
use futures::future::{err, FutureExt, LocalBoxFuture};
use std::ops::Deref;

/// Smart contract request context
///
/// Fields:
/// - TemplateID
/// - (private) DB connection
pub struct TemplateContext {
    pub template_id: TemplateID,
    client: Client,
    // TODO:    transaction: Transaction,
}

impl TemplateContext {
    pub async fn create_token(&self, data: NewToken) -> Result<Token, DBError> {
        let id = Token::insert(data, &self.client).await?;
        Token::load(id, &self.client).await
    }

    pub async fn update_token(&self, token: Token) -> Result<Token, DBError> {
        Token::update(token, &self.client).await
    }

    pub async fn load_token(&self, id: TokenID) -> Result<Option<Token>, DBError> {
        Token::find_by_token_id(id, &self.client).await
    }

    pub async fn load_asset(&self, id: AssetID) -> Result<Option<AssetState>, DBError> {
        AssetState::find_by_asset_id(id, &self.client).await
    }
}

/// Within Actix request retrieve
impl FromRequest for TemplateContext {
    type Config = ();
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // initialize whole context in this module - would make moduel quite complex but easier to debug
        // middleware might pass parameters via .extensions() or .app_data()
        let pool = req.app_data::<Data<Pool>>().expect("Failed to retrieve DB pool");
        let template_id: TemplateID = match req.app_data::<Data<TemplateID>>() {
            Some(id) => id.get_ref().clone(),
            None => return err(ApiError::bad_request("Template data not found by this path")).boxed_local(),
        };

        pool.clone()
            .get()
            .map(|res| {
                res.map(|client| TemplateContext {
                    client,
                    template_id,
                })
                .map_err(|err| DBError::from(err).into())
            })
            .boxed_local()
    }
}

/// Smart contract request context for asset contracts
///
/// Fields:
/// - TemplateContext
/// - AssetState
pub struct AssetTemplateContext {
    context: TemplateContext,
    pub asset: AssetState,
}

impl Deref for AssetTemplateContext {
    type Target = TemplateContext;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl AssetTemplateContext {
    pub fn new(context: TemplateContext, asset: AssetState) -> Self {
        Self { context, asset }
    }
}

/// Smart contract request context for asset contracts
///
/// Fields:
/// - TemplateContext
/// - AssetState
/// - Token
pub struct TokenTemplateContext {
    context: TemplateContext,
    pub asset: AssetState,
    pub token: Token,
}

impl Deref for TokenTemplateContext {
    type Target = TemplateContext;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl TokenTemplateContext {
    pub fn new(context: TemplateContext, asset: AssetState, token: Token) -> Self {
        Self { context, asset, token }
    }
}
