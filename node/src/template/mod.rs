//! Template module provides a set of traits, which make implementation of smart contracts for Tari
//! ergonomic, safe as to bindings validated during compile time yet flexible.
//!
//! Template is a factory and store for Contracts of AssetContract and TokenContract types.
//! Contracts provides access to Contract by a name.
//! Contract has incoming and outgoing serializable paramters (e.g. RPC call) with validators
//! and provides asynchronous procedures.
//! Context should take care of all the network communication.



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
