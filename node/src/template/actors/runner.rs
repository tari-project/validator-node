use crate::{
    config::NodeConfig,
    template::{Template, TemplateContext},
    types::TemplateID,
    wallet::WalletStore,
};
use actix::prelude::*;
use deadpool_postgres::Pool;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Implements [Actor] for Template
/// Executes instruction code within [TemplateContext]
pub struct TemplateRunner<T: Template + Clone + 'static> {
    context: TemplateContext<T>,
}

impl<T: Template + Clone> TemplateRunner<T> {
    #[inline]
    pub fn template_id() -> TemplateID {
        T::id()
    }

    /// Validates if [TemplateContext] is connected to this [actix::Actor]
    pub fn connected(&self) -> bool {
        match self.context.actor_address.as_ref() {
            Some(addr) => addr.connected(),
            None => false,
        }
    }

    /// Creates TemplateRunner
    ///
    /// ## Panics
    /// It will panic if NodeConfig.public_address is missing or failed to create WalletStore,
    /// as TemplateRunner won't be able to function properly
    pub fn create(pool: Arc<Pool>, config: NodeConfig) -> Self {
        let path = config.wallets_keys_path.clone();
        let wallets = WalletStore::init(path)
            .expect(format!("Failed to create TemplateRunner {}: WalletStore:", T::id()).as_str());
        let wallets = Arc::new(Mutex::new(wallets));
        let node_address = config.public_address.clone().expect(
            format!(
                "Failed to create TemplateRunner {}, missing public_address config: {:?}",
                T::id(),
                config
            )
            .as_str(),
        );
        let context = TemplateContext {
            pool,
            wallets,
            node_address,
            actor_address: None,
        };
        Self { context }
    }

    /// Start Actor returning TemplateContext
    ///
    /// ## Panics
    /// It will panic if is already connected
    pub fn start(self) -> TemplateContext<T> {
        if self.connected() {
            panic!("Failed to start already running TemplateRunner<{}>", T::id());
        }
        let mut context = self.context.clone();
        context.actor_address = Some(Actor::start(self));
        context
    }

    /// Retrieve current [TemplateContext] for this TemplateRunner
    #[inline]
    pub fn context(&self) -> TemplateContext<T> {
        self.context.clone()
    }
}

impl<T: Template + Clone + 'static> Unpin for TemplateRunner<T> {}

impl<T: Template + Clone + 'static> Actor for TemplateRunner<T> {
    type Context = Context<Self>;
}
