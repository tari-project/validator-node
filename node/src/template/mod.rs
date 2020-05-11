//! Template module provides a set of traits, which make implementation of smart contracts for Tari
//! ergonomic, safe as to bindings validated during compile time yet flexible.
//!
//! Template is a factory and store for Contracts of AssetContract and TokenContract types.
//! Contracts provides access to Contract by a name.
//! Contract has incoming and outgoing serializable paramters (e.g. RPC call) with validators
//! and provides asynchronous procedures.
//! Context should take care of all the network communication.
//!
//! ```ignore
//! async fn issue_tokens(asset: AssetState, amount: u64, price: u64, context: TemplateContext) -> Result<Vec<TokenID>, TemplateError> {
//!     let mut tokens = Vec::with_capacity(amount);
//!     for mut token in (0..amount).map(|_| NewToken::from(&asset)) {
//!         token.update_data(json!({price})?;
//!         tokens.push(context.create_token(token).await?.token_id());
//!     };
//!     Ok(tokens)
//! }
//! async fn buy_token(asset: AssetState, timeout_ms: u64, user_wallet_key: WalletID) -> Result<TokenID, TemplateError> {
//!     ...
//! }
//! #[derive(Contracts)]
//! enum AssetContracts {
//!     IssueTokens(issue_token),
//!     BuyToken(buy_token),
//! }
//!
//! struct SingleUseTokenTemplate;
//! impl Template for SingleUseTokenTemplate {
//!     type Network = CommitteeNetwork;
//!     type AssetContracts = AssetContracts;
//!     type TokenContracts = ();
//!     fn id() -> TemplateID {
//!         1.into()
//!     }
//! }
//! ```

use crate::types::TemplateID;
use std::str::FromStr;
use serde::{Serialize, Deserialize};

mod errors;
pub use errors::TemplateError;

pub trait Contracts {
    fn locate(name: &str) -> Result<Box<dyn Contract>, TemplateError>;
}

pub trait ContractHanlder {
    type Input: Serialize + Deserialize;
    type Output: Serialize + Deserialize;
    type State: ToString + FromStr;
    fn name() -> &'static str;
    fn call(params: Self::Input) -> Result<Self::Output, TemplateError>;
}

pub trait Contract {
    type Input: Serialize + Deserialize;
    type Output: Serialize + Deserialize;
    type State: ToString + FromStr;
    fn name() -> &'static str;
    fn call(params: Self::Input) -> Result<Self::Output, TemplateError>;
}

pub trait AssetContract: Contract {}

pub trait TokenContract: Contract {}

#[async_trait::async_trait]
pub trait Template {
    type AssetContracts: Contracts<Item=AssetContract>;
    type TokenContracts: Contracts<Item=TokenContract>;

    fn id() -> TemplateID;
}
