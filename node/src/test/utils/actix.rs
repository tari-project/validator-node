use super::{actix_test_pool, build_test_config, load_env};
use crate::{
    metrics::Metrics,
    template::{self, actix_web_impl::ActixTemplate, Template, TemplateContext, TemplateRunner},
    types::{AssetID, TokenID},
};
use actix::{Actor, Addr};
use actix_web::{client::ClientRequest, middleware::Logger, test, App};
use std::ops::Deref;

/// Full stack API server for templates testing purposes
///
/// Supports methods for posting assets and tokens instructions
/// Also impls Deref into actix [test::TestServer]
pub struct TestAPIServer<T: Template + 'static> {
    server: test::TestServer,
    context: TemplateContext<T>,
    pub metrics: Addr<Metrics>,
}

impl<T: Template + 'static> TestAPIServer<T> {
    pub fn new() -> Self {
        load_env();
        let _ = pretty_env_logger::try_init();
        let pool = actix_test_pool();
        let config = build_test_config().unwrap();
        let metrics = Metrics::default().start();
        let runner = TemplateRunner::<T>::create(pool, config, Some(metrics.clone()));
        let context = runner.start();
        let srv_context = context.clone();
        let server = test::start(move || {
            let app = App::new().wrap(Logger::default());
            T::actix_scopes()
                .into_iter()
                .fold(app, |app, scope| app.service(scope.data(srv_context.clone())))
        });
        Self {
            context,
            server,
            metrics,
        }
    }

    pub fn asset_call(&self, id: &AssetID, instruction: &str) -> ClientRequest {
        let uri = template::asset_call_path(id, instruction);
        self.server.post(uri)
    }

    pub fn token_call(&self, id: &TokenID, instruction: &str) -> ClientRequest {
        let uri = template::token_call_path(id, instruction);
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
