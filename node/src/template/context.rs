//! TemplateContext provides access to contextual request data
//!
//! TemplateContext is always supplied as first parameter to Smart Contract implementation

use crate::{
    api::errors::{ApiError, ApplicationError},
    db::{
        models::{
            transactions::{ContractTransaction, NewContractTransaction, UpdateContractTransaction},
            AssetState,
            tokens::{NewToken, Token, UpdateToken },
        },
    },
    types::{AssetID, TemplateID, TokenID},
};
use deadpool_postgres::{Client, Transaction};
use std::ops::{Deref, DerefMut};

/// Smart contract request context
///
/// Fields:
/// - TemplateID
/// - (private) DB connection
pub struct TemplateContext<'a> {
    pub template_id: TemplateID,
    pub(crate) client: Client,
    pub(crate) db_transaction: Option<Transaction<'a>>,
    pub(crate) contract_transaction: Option<ContractTransaction>,
}

impl<'a> TemplateContext<'a> {
    pub async fn create_token(&self, data: NewToken) -> Result<Token, ApiError> {
        let id = Token::insert(data, &self.client).await?;
        Ok(Token::load(id, &self.client).await?)
    }

    pub async fn update_token(&self, token: Token, data: UpdateToken) -> Result<Token, ApiError> {
        if let Some(transaction) = self.contract_transaction.as_ref() {
            Ok(token.update(data, transaction, &self.client).await?)
        } else {
            Err(ApplicationError::new(format!("Failed to update token {} without ContractTransaction", token.token_id)).into())
        }
    }

    pub async fn load_token(&self, id: TokenID) -> Result<Option<Token>, ApiError> {
        Ok(Token::find_by_token_id(id, &self.client).await?)
    }

    pub async fn load_asset(&self, id: AssetID) -> Result<Option<AssetState>, ApiError> {
        Ok(AssetState::find_by_asset_id(id, &self.client).await?)
    }

    /// Creates [ContractTransaction]
    // TODO: move this somewhere outside of reach of contract code...
    pub async fn create_transaction(&mut self, data: NewContractTransaction) -> Result<(), ApiError> {
        self.contract_transaction = Some(ContractTransaction::insert(data, &self.client).await?);
        Ok(())
    }

    /// Updates result and status of [ContractTransaction]
    // TODO: move this somewhere outside of reach of contract code...
    pub async fn update_transaction(&mut self, data: UpdateContractTransaction) -> Result<(), ApiError> {
        if let Some(transaction) = self.contract_transaction.take() {
            self.contract_transaction = Some(transaction.update(data, &self.client).await?);
            Ok(())
        } else {
            Err(ApplicationError::new(format!("Failed to update ContractTransaction {:?}: transaction not found", data)).into())
        }
    }

    pub async fn commit(&self) -> Result<(), ApiError> {
        // TODO: implement database transactino through the whole Context
        Ok(())
    }
}

/// Extract [ContractTransaction] from TemplateContext
impl<'a> From<TemplateContext<'a>> for Option<ContractTransaction> {
    #[inline]
    fn from(ctx: TemplateContext<'a>) -> Option<ContractTransaction> {
        ctx.contract_transaction
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
impl<'a> DerefMut for AssetTemplateContext<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<'a> AssetTemplateContext<'a> {
    pub fn new(context: TemplateContext<'a>, asset: AssetState) -> Self {
        Self { context, asset }
    }
}

impl<'a> From<AssetTemplateContext<'a>> for Option<ContractTransaction> {
    #[inline]
    fn from(ctx: AssetTemplateContext<'a>) -> Option<ContractTransaction> {
        ctx.context.into()
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
impl<'a> DerefMut for TokenTemplateContext<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<'a> TokenTemplateContext<'a> {
    pub fn new(context: TemplateContext<'a>, asset: AssetState, token: Token) -> Self {
        Self { context, asset, token }
    }
}

impl<'a> From<TokenTemplateContext<'a>> for Option<ContractTransaction> {
    #[inline]
    fn from(ctx: TokenTemplateContext<'a>) -> Option<ContractTransaction> {
        ctx.context.into()
    }
}
