use super::{actix_test_pool, build_test_config, load_env};
use crate::{
    template::{actix_web_impl::ActixTemplate, Template, TemplateContext, TemplateRunner},
    types::{AssetID, TokenID},
};
use actix_web::{client::ClientRequest, middleware::Logger, test, App};
use std::ops::Deref;

/// Full stack API server for templates testing purposes
///
/// Supports methods for posting assets and tokens instructions
/// Also impls Deref into actix [test::TestServer]
pub struct TestAPIServer<T: Template + 'static> {
    server: test::TestServer,
    context: TemplateContext<T>,
}

impl<T: Template + 'static> TestAPIServer<T> {
    pub fn new() -> Self {
        load_env();
        let _ = pretty_env_logger::try_init();
        let pool = actix_test_pool();
        let config = build_test_config().unwrap();
        let runner = TemplateRunner::<T>::create(pool, config);
        let context = runner.start();
        let srv_context = context.clone();
        let server = test::start(move || {
            let app = App::new().wrap(Logger::default());
            T::actix_scopes()
                .into_iter()
                .fold(app, |app, scope| app.service(scope.data(srv_context.clone())))
        });
        Self { context, server }
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

    pub fn context(&self) -> &TemplateContext<T> {
        &self.context
    }
}

impl<T: Template + 'static> Deref for TestAPIServer<T> {
    type Target = test::TestServer;

    fn deref(&self) -> &Self::Target {
        &self.server
    }
}
