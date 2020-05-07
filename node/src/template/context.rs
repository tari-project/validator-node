//! TemplateContext provides access to contextual request data
//!
//! TemplateContext is always supplied as first parameter to Smart Contract implementation

use crate::{
    db::{
        models::{
            transaction::{ContractTransaction, NewContractTransaction},
            AssetState,
            NewToken,
            Token,
        },
        utils::errors::DBError,
    },
    types::{AssetID, TemplateID, TokenID},
};
use deadpool_postgres::{Client, Transaction};
use std::ops::Deref;

/// Smart contract request context
///
/// Fields:
/// - TemplateID
/// - (private) DB connection
pub struct TemplateContext<'a> {
    pub template_id: TemplateID,
    pub(crate) client: Client,
    pub(crate) transaction: Option<Transaction<'a>>,
}

impl<'a> TemplateContext<'a> {
    pub async fn create_token(&self, data: NewToken) -> Result<Token, DBError> {
        let id = Token::insert(data, &self.client).await?;
        Token::load(id, &self.client).await
    }

    pub async fn update_token(&self, token: &Token) -> Result<u64, DBError> {
        token.update(&self.client).await
    }

    pub async fn load_token(&self, id: TokenID) -> Result<Option<Token>, DBError> {
        Token::find_by_token_id(id, &self.client).await
    }

    pub async fn load_asset(&self, id: AssetID) -> Result<Option<AssetState>, DBError> {
        AssetState::find_by_asset_id(id, &self.client).await
    }

    // TODO: move this somewhere outside of reach of contract code...
    pub async fn create_transaction(&self, data: NewContractTransaction) -> Result<ContractTransaction, DBError> {
        Ok(ContractTransaction::insert(data, &self.client).await?)
    }

    pub async fn commit(&self) -> Result<(), DBError> {
        // TODO: implement database transactino through the whole Context
        Ok(())
    }
}

/// Smart contract request context for asset contracts
///
/// Fields:
/// - TemplateContext
/// - AssetState
pub struct AssetTemplateContext<'a> {
    context: TemplateContext<'a>,
    pub asset: AssetState,
}

impl<'a> Deref for AssetTemplateContext<'a> {
    type Target = TemplateContext<'a>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<'a> AssetTemplateContext<'a> {
    pub fn new(context: TemplateContext<'a>, asset: AssetState) -> Self {
        Self { context, asset }
    }
}

/// Smart contract request context for asset contracts
///
/// Fields:
/// - TemplateContext
/// - AssetState
/// - Token
pub struct TokenTemplateContext<'a> {
    context: TemplateContext<'a>,
    pub asset: AssetState,
    pub token: Token,
}

impl<'a> Deref for TokenTemplateContext<'a> {
    type Target = TemplateContext<'a>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<'a> TokenTemplateContext<'a> {
    pub fn new(context: TemplateContext<'a>, asset: AssetState, token: Token) -> Self {
        Self { context, asset, token }
    }
}
