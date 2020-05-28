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
//! See [actors] module for details and single_use_tokens for example of TokenContracts implementation.
//!
//! NOTE: coupled with actix at Phase1 for API simplicity, might be decoupled later.
//!
//! ### Notes:
//! - [TemplateRunner] implements [actix::Actor] and is executing Contract code in async Actor
//! - Derive macros from [tari_validator_derive] allow token contracts auto-implementation,
//! generating actix-web interface and Actor Handler with Message implementation
//! - [ContractCallMsg] trait is a message triggering contract execution: Handler for TemplateRunner
//! is auto-implemented for all Messages which implement ContractCallMsg
//! - Wallets and generation of temp wallets available for contract via [InsturctionContext]
//! - InstructionContext allows to create and wait for subinstructions from contract code
//! - Instruction states transition via InstructionContext method, transitioning happens automatically
//! based on contract execution result. See impl [`actix::Handler`] for [TemplateRunner]
//! - Contracts can use tokio::delay_for to wait for external event
//!
//! ### Caveats:
//! - Contract Actors sharing thread pool with actix_web
//! - There is no subscriptions on external events for contract code, like wallet balance change or
//! transaction status change, hence contracts should use delay_for and check to wait for event to occur
//! - Contract code does not implement restart and continuation on failure,
//! does not support rollbacks on failures

// TODO: Potentially via unsafe code Template still might acquire access to the database connection
// we shall provide some custom build script which disallows installing templates using unsafe on a node

use crate::types::TemplateID;
use actix_web::web;

pub mod errors;
pub use errors::TemplateError;

pub mod actix_web_impl;
pub mod actors;
pub use actors::{ContractCallMsg, TemplateRunner};

pub mod single_use_tokens;

mod context;
pub use context::{
    AssetInstructionContext,
    ContextEvent,
    InstructionContext,
    TemplateContext,
    TokenInstructionContext,
};

const LOG_TARGET: &'static str = "tari_validator_node::template";

pub trait Contracts {
    fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig);
}
impl Contracts for () {
    fn setup_actix_routes(_: TemplateID, _: &mut web::ServiceConfig) {}
}

pub trait Template: Clone {
    type AssetContracts: Contracts;
    type TokenContracts: Contracts;

    fn id() -> TemplateID;
}
