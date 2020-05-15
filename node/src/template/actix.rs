use super::{Contracts, Template, TemplateContext};
use crate::{
    api::errors::{ApiError, ApplicationError},
    types::{AssetID, TemplateID, TokenID},
    db::utils::errors::DBError,
};
use actix_web::{dev::Payload, web, web::Data, FromRequest, HttpRequest};
use anyhow::Result;
use deadpool_postgres::Pool;
use futures::future::{err, FutureExt, LocalBoxFuture};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct AssetCallParams {
    features: [char; 4],
    raid_id: [char; 15],
    hash: [char; 32],
}
impl AssetCallParams {
    pub fn asset_id(&self, tpl: &TemplateID) -> Result<AssetID> {
        let template_id = tpl.to_hex();
        let features: String = self.features.iter().collect();
        let raid_id: String = self.raid_id.iter().collect();
        let hash: String = self.hash.iter().collect();
        Ok(format!("{}{}{}.{}", template_id, features, raid_id, hash).parse()?)
    }
}

#[derive(Deserialize)]
pub(crate) struct TokenCallParams {
    features: [char; 4],
    raid_id: [char; 15],
    hash: [char; 32],
    uid: [char; 32],
}
impl TokenCallParams {
    pub fn token_id(&self, tpl: &TemplateID) -> Result<TokenID> {
        let uid: String = self.uid.iter().collect();
        Ok(format!("{}{}", self.asset_id(tpl)?, uid).parse()?)
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

/// TemplateContext can be retrieved from actix web requests at given path
impl<'a> FromRequest for TemplateContext<'a> {
    type Config = ();
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // initialize whole context in this module - would make moduel quite complex but easier to debug
        // middleware might pass parameters via .extensions() or .app_data()
        let pool = req
            .app_data::<Data<Pool>>()
            .expect("Failed to retrieve DB pool")
            .as_ref();
        let template_id: TemplateID = match req.app_data::<Data<TemplateID>>() {
            Some(id) => id.get_ref().clone(),
            None => return err(ApplicationError::bad_request("Template data not found by this path").into()).boxed_local(),
        };

        let pool = pool.clone();
        async move {
            match pool.get().await {
                Ok(client) => Ok(TemplateContext {
                    client,
                    template_id,
                    transaction: None,
                }),
                Err(err) => Err(DBError::from(err).into()),
            }
        }
        .boxed_local()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::models::tokens::*;
    use crate::test_utils::{builders::*, test_db_client};

    const NODE_ID: [u8; 6] = [0, 1, 2, 3, 4, 5];

    #[actix_rt::test]
    async fn requests() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();
        let tokens = (0..3)
            .map(|_| TokenID::new(&asset.asset_id, NODE_ID).unwrap())
            .map(|token_id| NewToken { asset_state_id: asset.id, token_id, ..NewToken::default() });

        let request = HttpRequestBuilder::default().asset_call(&asset.asset_id, "test_contract").build().to_http_request();
        let context = TemplateContext::from_request(&request, &mut Payload::None).await.unwrap();
        assert_eq!(context.template_id, asset.asset_id.template_id());
        for token in tokens {
            let created = context.create_token(token.clone()).await.unwrap();
            assert_eq!(token.token_id, created.token_id);
        }
    }
}
