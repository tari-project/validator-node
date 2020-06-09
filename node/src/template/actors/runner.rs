use crate::{
    config::NodeConfig,
    metrics::Metrics,
    template::{Template, TemplateContext},
    types::TemplateID,
    wallet::WalletStore,
};
use actix::{fut, prelude::*};
use deadpool_postgres::{Client, Pool};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Implements [Actor] for Template
/// Executes instruction code within [TemplateContext]
pub struct TemplateRunner<T: Template + Clone + 'static> {
    context: TemplateContext<T>,
    // This DB client is available for non-transactional operations
    client: Option<Arc<Client>>,
    pub(super) bandwidth: Arc<Semaphore>,
}

impl<T: Template + Clone> TemplateRunner<T> {
    #[inline]
    pub fn template_id() -> TemplateID {
        T::id()
    }

    /// Validates if [TemplateContext] is connected to this [actix::Actor]
    pub fn connected(&self) -> bool {
        match self.context.actor_addr.as_ref() {
            Some(addr) => addr.connected(),
            None => false,
        }
    }

    /// Creates TemplateRunner
    ///
    /// ## Panics
    /// It will panic if NodeConfig.public_address is missing or failed to create WalletStore,
    /// as TemplateRunner won't be able to function properly
    pub fn create(pool: Arc<Pool>, config: NodeConfig, metrics_addr: Option<Addr<Metrics>>) -> Self {
        let path = config.wallets_keys_path.clone();
        let wallets = WalletStore::init(path.clone()).expect(
            format!(
                "Failed to create TemplateRunner {}: WalletStore at {:?}:",
                T::id(),
                path
            )
            .as_str(),
        );
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
            actor_addr: None,
            metrics_addr,
        };
        let bandwidth = Arc::new(Semaphore::new(config.template.runner_max_jobs));
        Self {
            context,
            client: None,
            bandwidth,
        }
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
        context.actor_addr = Some(Actor::start(self));
        context
    }

    /// Retrieve current [TemplateContext] for this TemplateRunner
    #[inline]
    pub fn context(&self) -> TemplateContext<T> {
        self.context.clone()
    }

    /// Get shared DB client
    ///
    /// Shared DB client is using query pipelining, it can be used for all DB operations
    /// which can be performed on not mutable reference to postgres client (query, execute).
    /// It is available opportunistically and helping to save DB pool of draining
    /// significantly minimizing number of required open DB connections.
    pub fn get_shared_db_client(&mut self) -> Option<Arc<Client>> {
        if let Some(client) = self.client.take() {
            if !client.is_closed() {
                self.client = Some(client);
            }
        }
        if self.client.is_none() {
            self.context.addr().do_send(UpdateSharedClient);
            None
        } else {
            self.client.clone()
        }
    }
}

impl<T: Template + Clone + 'static> Unpin for TemplateRunner<T> {}

impl<T: Template + Clone + 'static> Actor for TemplateRunner<T> {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.context.actor_addr = Some(ctx.address());
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct UpdateSharedClient;

/// Actor is accepting TokenCallMsg and tries to perform activity
impl<T> Handler<UpdateSharedClient> for TemplateRunner<T>
where T: Template + 'static
{
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _: UpdateSharedClient, _ctx: &mut Context<Self>) -> Self::Result {
        if self.client.is_none() {
            let pool = self.context.pool.clone();
            let pool_fut = async move { pool.get().await };
            let fut = fut::wrap_future(pool_fut).map(|res, actor: &mut Self, _ctx| {
                match res {
                    Ok(client) => {
                        actor.client = Some(Arc::new(client));
                    },
                    _ => {},
                };
            });
            Box::pin(fut)
        } else {
            Box::pin(fut::ready(()))
        }
    }
}
