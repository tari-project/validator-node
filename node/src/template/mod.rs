//! Template module provides a set of traits, which make implementation of smart contracts for Tari
//! ergonomic, safe, with inputs/output validated during compile time, yet flexible.
//!
//! [Template] is effectively a configurator of routes for [Contracts]
//! ,where AssetContracts and TokenContracts are Template's associated types
//! ensuring proper setup of RPC routes. E.g. AssetContracts require AssetID
//! and TokenContracts require TokenID as part of the call.
//!
//! [TemplateContext] provides access to data and network.
//!
//! Contracts trait derived on enums matching variants to contract implementation (as async fn),
//! all the boilerplate largely is provided via macros `#[derive(Contracts)]` and `#[contract]`.
//!
//! NOTE: coupled with actix at Phase1 for API simplicity, might be decoupled later.

// TODO: Potentially via unsafe code Template still might acquire access to the database connection
// we shall provide some custom build script which disallows installing templates using unsafe on a node

use crate::types::TemplateID;

mod errors;
pub use errors::TemplateError;

pub mod actix;
pub mod single_use_tokens;

mod context;
pub use context::{AssetTemplateContext, TemplateContext, TokenTemplateContext};

pub trait Contracts {
    fn setup_actix_routes(scope: &mut actix_web::web::ServiceConfig);
}

#[async_trait::async_trait]
pub trait Template {
    type AssetContracts: Contracts;
    type TokenContracts: Contracts;

    fn id() -> TemplateID;
}
