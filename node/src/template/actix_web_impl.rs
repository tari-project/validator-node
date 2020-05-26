use super::{Contracts, Template, LOG_TARGET};
use crate::{
    api::errors::{ApiError},
    types::{AssetID, TemplateID, TokenID},
};
use actix_web::web;
use log::info;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AssetCallParams {
    features: String,
    raid_id: String,
    hash: String,
}
impl AssetCallParams {
    pub fn asset_id(&self, tpl: TemplateID) -> Result<AssetID, ApiError> {
        let template_id = tpl.to_hex();
        Ok(format!("{}{}{}.{}", template_id, self.features, self.raid_id, self.hash).parse()?)
    }
}

#[derive(Deserialize)]
pub struct TokenCallParams {
    features: String,
    raid_id: String,
    hash: String,
    uid: String,
}
impl TokenCallParams {
    pub fn token_id(&self, tpl: TemplateID) -> Result<TokenID, ApiError> {
        Ok(format!("{}{}", self.asset_id(tpl)?, self.uid).parse()?)
    }

    pub fn asset_id(&self, tpl: TemplateID) -> Result<AssetID, ApiError> {
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

pub trait ActixTemplate: Template {
    /// Creates web::Scope with routes for template
    fn actix_scopes() -> Vec<actix_web::Scope> {
        let id: TemplateID = Self::id();

        let asset_root = format!("/asset_call/{}/{{features}}/{{raid_id}}/{{hash}}", id);
        info!(
            target: LOG_TARGET,
            "template={}, installing assets API root {}", id, asset_root
        );
        let asset_scope = web::scope(asset_root.as_str())
            .data(id)
            .configure(|app| <Self::AssetContracts as Contracts>::setup_actix_routes(id, app));
        let token_root = format!("/token_call/{}/{{features}}/{{raid_id}}/{{hash}}/{{uid}}", id);
        info!(
            target: LOG_TARGET,
            "template={}, installing tokens API root {}", id, token_root
        );
        let token_scope = web::scope(token_root.as_str())
            .data(id)
            .configure(|app| <Self::TokenContracts as Contracts>::setup_actix_routes(id, app));

        vec![asset_scope, token_scope]
    }
}

impl<A: Template> ActixTemplate for A {}

/// TemplateContext can be retrieved from actix web requests at given path
// impl FromRequest for TemplateContext {
//     type Config = ();
//     type Error = ApiError;
//     type Future = Ready<Result<Self, Self::Error>>;

//     #[inline]
//     fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
//         // initialize whole context in this module - would make moduel quite complex but easier to debug
//         // middleware might pass parameters via .extensions() or .app_data()
//         // TODO: this is not secure as we allow routes configuration via Contracts trait
//         // potentially template might configure a route to get access to the Pool and WalletStore
//         let pool = req.app_data::<Arc<Pool>>().expect("Failed to retrieve DB pool").clone();
//         let wallets = req
//             .app_data::<Arc<Mutex<WalletStore>>>()
//             .expect("Failed to retrieve WalletStore")
//             .clone();
//         let config = req.app_data::<NodeConfig>().expect("Failed to retrieve NodeConfig");
//         let address = config
//             .public_address
//             .clone()
//             .expect("Public address is not configured for Node");
//         let template_id: TemplateID = match req.app_data::<Data<TemplateID>>() {
//             Some(id) => id.get_ref().clone(),
//             None => return err(ApplicationError::bad_request("Template data not found by this path").into()),
//         };

//         ok(TemplateContext {
//             pool,
//             template_id,
//             wallets,
//             address,
//         })
//     }
// }

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::consensus::instructions::*,
        types::{InstructionID, NodeID},
        template::*
    };
    use crate::test::utils::{Test, actix::TestAPIServer, builders::*, test_db_client};
    use actix_web::{http::StatusCode, web, HttpResponse, FromRequest, Result, dev::Payload};

    #[actix_rt::test]
    async fn requests() {
        let (client, _lock) = test_db_client().await;
        let asset = AssetStateBuilder::default().build(&client).await.unwrap();

        let request = HttpRequestBuilder::default()
            .asset_call(&asset.asset_id, "test_contract")
            .build()
            .to_http_request();
        let context = web::Data::<TemplateContext<TestTemplate>>::from_request(&request, &mut Payload::None)
            .await
            .unwrap();
        assert_eq!(context.template_id(), asset.asset_id.template_id());
        context
            .create_instruction(NewInstruction {
                id: InstructionID::new(NodeID::stub()).unwrap(),
                asset_id: asset.asset_id,
                status: InstructionStatus::Scheduled,
                ..Default::default()
            })
            .await
            .unwrap();
    }

    // *** Test template implementation - low level API testins *****

    // Asset contracts
    async fn asset_handler(path: web::Path<AssetCallParams>, tpl: web::Data<TemplateID>) -> Result<HttpResponse> {
        Ok(HttpResponse::Ok().body(path.asset_id(**tpl)?.to_string()))
    }
    enum AssetConracts {}
    impl Contracts for AssetConracts {
        fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig) {
            log::info!("template={}, registering asset routes", tpl);
            scope.service(web::resource("test").route(web::post().to(asset_handler)));
        }
    }
    // Token contracts
    async fn token_handler(path: web::Path<TokenCallParams>, tpl: web::Data<TemplateID>) -> Result<HttpResponse> {
        Ok(HttpResponse::Ok().body(path.token_id(**tpl)?.to_string()))
    }
    enum TokenConracts {}
    impl Contracts for TokenConracts {
        fn setup_actix_routes(_: TemplateID, scope: &mut web::ServiceConfig) {
            scope.service(web::resource("test").route(web::post().to(token_handler)));
        }
    }
    #[derive(Clone)]
    struct TestTemplate;
    impl Template for TestTemplate {
        type AssetContracts = AssetConracts;
        type TokenContracts = TokenConracts;

        fn id() -> TemplateID {
            65536.into()
        }
    }
    // *** End of Test template implementation *****

    #[actix_rt::test]
    async fn test_actix_template_routes() {
        let srv = TestAPIServer::new(TestTemplate::actix_scopes);

        use actix_web::http::Method;
        let tpl = TestTemplate::id();
        let req_resp = [
            // root path
            (Method::GET, "/".to_string(), StatusCode::NOT_FOUND),
            (Method::POST, "/".to_string(), StatusCode::NOT_FOUND),
            // asset routes
            (Method::POST, format!("/asset_call/{}/test", tpl), StatusCode::NOT_FOUND),
            (
                Method::POST,
                format!("/asset_call/{}/{:04X}/{:015X}/{:032X}/test", tpl, 1, 2, 3),
                StatusCode::OK,
            ),
            (
                Method::GET,
                format!("/asset_call/{}/{:04X}/{:015X}/{:032X}/test", tpl, 1, 2, 3),
                StatusCode::METHOD_NOT_ALLOWED,
            ),
            (
                Method::POST,
                format!("/asset_call/{}/{:04X}/{:015X}/{:032X}/", tpl, 1, 2, 3),
                StatusCode::NOT_FOUND,
            ),
            (Method::POST, "/asset_call/".to_string(), StatusCode::NOT_FOUND),
            (Method::POST, format!("/asset_call/{}", tpl), StatusCode::NOT_FOUND),
            (
                Method::POST,
                format!("/asset_call/{}/{:04X}/{:015X}/{:032X}/test", "1.0", 1, 2, 3),
                StatusCode::NOT_FOUND,
            ),
            (
                Method::POST,
                format!("/asset_call/{}/a/b/c/test", tpl),
                StatusCode::BAD_REQUEST,
            ),
            // token routes
            (
                Method::POST,
                format!("/token_call/{}/{:04X}/{:015X}/{:032X}/{:032X}/test", tpl, 1, 2, 3, 4),
                StatusCode::OK,
            ),
            (
                Method::GET,
                format!("/token_call/{}/{:04X}/{:015X}/{:032X}/{:032X}/test", tpl, 1, 2, 3, 4),
                StatusCode::METHOD_NOT_ALLOWED,
            ),
            (
                Method::POST,
                format!("/token_call/{}/{:04X}/{:015X}/{:032X}/{:032X}/", tpl, 1, 2, 3, 4),
                StatusCode::NOT_FOUND,
            ),
            (Method::POST, "/token_call/".to_string(), StatusCode::NOT_FOUND),
            (Method::POST, format!("/token_call/{}", tpl), StatusCode::NOT_FOUND),
            (
                Method::POST,
                format!("/token_call/{}/{:04X}/{:015X}/{:032X}/{:032X}/test", "1.0", 1, 2, 3, 4),
                StatusCode::NOT_FOUND,
            ),
            (
                Method::POST,
                format!("/token_call/{}/a/b/c/d/test", tpl),
                StatusCode::BAD_REQUEST,
            ),
        ];

        for (method, uri, code) in &req_resp {
            let resp = srv
                .request((*method).clone(), srv.url(uri.as_str()))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), *code, "{} {}", method, uri);
        }
    }

    #[actix_rt::test]
    async fn full_stack_server() {
        let srv = TestAPIServer::new(TestTemplate::actix_scopes);

        let tpl = TestTemplate::id();
        let asset: AssetID = format!("{}{:04X}{:015X}.{:032X}", tpl.to_hex(), 1, 2, 3)
            .parse()
            .unwrap();
        let token: TokenID = format!("{}{:04X}{:015X}.{:032X}{:032X}", tpl.to_hex(), 1, 2, 3, 4)
            .parse()
            .unwrap();

        let mut resp = srv.asset_call(&asset, "test").send().await.unwrap();
        assert!(resp.status().is_success());
        assert_eq!(resp.body().await.unwrap(), asset.to_string());

        let mut resp = srv.token_call(&token, "test").send().await.unwrap();
        assert!(resp.status().is_success());
        assert_eq!(resp.body().await.unwrap(), token.to_string());
    }

    // *** Test TemplateContext *****

    // Asset contracts
    async fn asset_handler_context(path: web::Path<AssetCallParams>, ctx: web::Data<TemplateContext<TestTemplateContext>>) -> Result<HttpResponse> {
        Ok(HttpResponse::Ok().body(path.asset_id(ctx.template_id())?.to_string()))
    }
    enum AssetConractsContext {}
    impl Contracts for AssetConractsContext {
        fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig) {
            log::info!("template={}, registering asset routes", tpl);
            scope.service(web::resource("test").route(web::post().to(asset_handler_context)));
        }
    }
    #[derive(Clone)]
    struct TestTemplateContext;
    impl Template for TestTemplateContext {
        type AssetContracts = AssetConractsContext;
        type TokenContracts = ();

        fn id() -> TemplateID {
            65537.into()
        }
    }
    //*** End of Test template implementation *****

    #[actix_rt::test]
    async fn template_context_full_stack() {
        let srv = TestAPIServer::new(TestTemplateContext::actix_scopes);

        let tpl = TestTemplateContext::id();
        let asset_id = Test::<AssetID>::from_template(tpl);

        let mut resp = srv.asset_call(&asset_id, "test").send().await.unwrap();
        assert!(resp.status().is_success());
        assert_eq!(resp.body().await.unwrap(), asset_id.to_string());
    }
}
