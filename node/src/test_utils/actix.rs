use super::{actix_test_pool, load_env};
use crate::types::{AssetID, TokenID};
use actix_web::{client::ClientRequest, middleware::Logger, test, App, Scope};
use std::ops::Deref;

/// Full stack API server for templates testing purposes
///
/// Supports methods for posting assets and tokens instructions
/// Also impls Deref into actix [test::TestServer]
pub struct TestAPIServer {
    server: test::TestServer,
}

impl TestAPIServer {
    pub fn new<F>(scopes: F) -> Self
    where F: (FnOnce() -> Vec<Scope>) + Clone + Send + 'static {
        load_env();
        let _ = pretty_env_logger::try_init();
        let server = test::start(move || {
            let app = App::new().app_data(actix_test_pool()).wrap(Logger::default());
            scopes.clone()()
                .into_iter()
                .fold(app, |app, scope| app.service(scope.app_data(actix_test_pool())))
        });
        Self { server }
    }

    pub fn asset_call(&self, id: &AssetID, instruction: &str) -> ClientRequest {
        let uri = format!(
            "/asset_call/{}/{:04X}/{}/{}/{}",
            id.template_id(),
            id.features(),
            id.raid_id().to_base58(),
            id.hash(),
            instruction
        );
        self.server.post(uri)
    }

    pub fn token_call(&self, id: &TokenID, instruction: &str) -> ClientRequest {
        let asset_id = id.asset_id();
        let uri = format!(
            "/token_call/{}/{:04X}/{}/{}/{}/{}",
            asset_id.template_id(),
            asset_id.features(),
            asset_id.raid_id().to_base58(),
            asset_id.hash(),
            id.uid().to_simple(),
            instruction
        );
        self.server.post(uri)
    }
}

impl Deref for TestAPIServer {
    type Target = test::TestServer;

    fn deref(&self) -> &Self::Target {
        &self.server
    }
}
