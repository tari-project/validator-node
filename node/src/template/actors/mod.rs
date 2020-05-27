//! [Template] [Instruction]s are executed in [TemplateRunner] via [`actix::Actor`].
//!
//! [TemplateRunner] when starting provides [TemplateContext] which
//! can be passed to any entity which need to send messages to Actor.
//! For instance, [TemplateContext] is available via [`actix_web::web::Data`]
//! and all the relevant Template web routes.
//!
//! [TemplateRunner] provides [TemplateContext] for running actor via Self.
//!
//! Usually one does not need to implement Handler and Actor, it's autoimplemented
//! via implementing [ContractCallMsg].
//!
//! Implementations for [AssetCallMsg] and [TokenCallMsg] might be derived
//! using [derive(Contracts)] macro on Contracts enum:
//! ```
//! #[derive(Contracts, Serialize, Deserialize, Clone)]
//! #[contracts(template="SingleUseTokenTemplate",token)]
//! pub enum TokenContracts {
//!     #[contract(method="sell_token")]
//!     SellToken(SellTokenParams),
//!     #[contract(method="sell_token_lock")]
//!     SellTokenLock(SellTokenLockParams),
//!     #[contract(method="transfer_token")]
//!     TransferToken(TransferTokenParams),
//! }
//! ```


pub use handler::*;
pub use runner::*;

mod handler;
mod runner;
