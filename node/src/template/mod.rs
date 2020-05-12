//! Template module provides a set of traits, which make implementation of smart contracts for Tari
//! ergonomic, safe as to bindings validated during compile time yet flexible.
//!
//! Template is a factory and store for Contracts of AssetContract and TokenContract types.
//! Contracts provides access to Contract by a name.
//! Contract has incoming and outgoing serializable paramters (e.g. RPC call) with validators
//! and provides asynchronous procedures.
//! Context should take care of all the network communication.
//!
//! This is coupled with actix at Phase1 for API simplicity, might be decoupled later

use crate::types::TemplateID;

mod errors;
pub use errors::TemplateError;

pub mod actix;
pub mod single_use_tokens;

mod context;
pub use context::TemplateContext;

pub trait Contracts {
    fn setup_actix_routes(scope: &mut actix_web::web::ServiceConfig);
}

#[async_trait::async_trait]
pub trait Template {
    type AssetContracts: Contracts;
    type TokenContracts: Contracts;

    fn id() -> TemplateID;
}
