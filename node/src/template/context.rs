//! TemplateContext provides access to contextual request data
//!
//! TemplateContext is always supplied as first parameter to Smart Contract implementation

use super::errors::TemplateError;
use crate::{
    api::errors::{ApiError, ApplicationError},
    db::{
        models::{
            consensus::instructions::{Instruction, NewInstruction, UpdateInstruction},
            tokens::{NewToken, Token, UpdateToken},
            AssetState,
        },
        utils::errors::DBError,
    },
    processing_err,
    types::{AssetID, Pubkey, TemplateID, TokenID},
    wallet::{NodeWallet, WalletStore},
};
use deadpool_postgres::{Client, Transaction};
use multiaddr::Multiaddr;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::Mutex;

/// Smart contract request context
///
/// Fields:
/// - TemplateID
/// - (private) DB connection
pub struct TemplateContext<'a> {
    pub template_id: TemplateID,
    // TODO: this is not secure, we provide access to context to template,
    // To make it safe our templates should be completely sandboxed,
    // having only access to the context methods...
    pub(crate) client: Client,
    pub(crate) wallets: Arc<Mutex<WalletStore>>,
    pub(crate) address: Multiaddr,
    pub(crate) db_transaction: Option<Transaction<'a>>,
    pub(crate) instruction: Option<Instruction>,
}

impl<'a> TemplateContext<'a> {
    pub async fn create_token(&self, data: NewToken) -> Result<Token, TemplateError> {
        let id = Token::insert(data, &self.client).await?;
        Ok(Token::load(id, &self.client).await?)
    }

    pub async fn update_token(&self, token: Token, data: UpdateToken) -> Result<Token, TemplateError> {
        if let Some(instruction) = self.instruction.as_ref() {
            Ok(token.update(data, instruction, &self.client).await?)
        } else {
            return processing_err!("Failed to update token {} without Instruction", token.token_id);
        }
    }

    pub async fn load_token(&self, id: TokenID) -> Result<Option<Token>, TemplateError> {
        Ok(Token::find_by_token_id(&id, &self.client).await?)
    }

    pub async fn load_asset(&self, id: AssetID) -> Result<Option<AssetState>, TemplateError> {
        Ok(AssetState::find_by_asset_id(&id, &self.client).await?)
    }

    /// Creates [Instruction]
    // TODO: move this somewhere outside of reach of contract code...
    pub async fn create_instruction(&mut self, data: NewInstruction) -> Result<(), TemplateError> {
        self.instruction = Some(Instruction::insert(data, &self.client).await?);
        // TODO: broadcast instruction to network
        Ok(())
    }

    /// Updates result and status of [Instruction]
    // TODO: move this somewhere outside of reach of contract code...
    pub async fn update_instruction(&mut self, data: UpdateInstruction) -> Result<(), TemplateError> {
        if let Some(instruction) = self.instruction.take() {
            self.instruction = Some(instruction.update(data, &self.client).await?);
            Ok(())
        } else {
            return processing_err!("Failed to update Instruction {:?}: instruction not found", data);
        }
    }

    pub async fn commit(&self) -> Result<(), DBError> {
        // TODO: implement database transactino through the whole Context
        Ok(())
    }

    pub async fn create_temp_wallet(&mut self) -> Result<Pubkey, TemplateError> {
        if self.instruction.is_none() {
            return processing_err!("Failed to create temporary wallet for Contract: ContractTransaction not found");
        }
        let transaction = self.client.transaction().await.map_err(DBError::from)?;
        let wallet_name = self.instruction.as_ref().unwrap().id.to_string();
        let wallet = NodeWallet::new(self.address.clone(), wallet_name)?;
        let wallet = self.wallets.lock().await.add(wallet, &transaction).await?;
        transaction.commit().await.map_err(DBError::from)?;
        Ok(wallet.public_key_hex())
    }
}

/// Extract [Instruction] from TemplateContext
impl<'a> From<TemplateContext<'a>> for Option<Instruction> {
    #[inline]
    fn from(ctx: TemplateContext<'a>) -> Option<Instruction> {
        ctx.instruction
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

impl<'a> From<AssetTemplateContext<'a>> for Option<Instruction> {
    #[inline]
    fn from(ctx: AssetTemplateContext<'a>) -> Option<Instruction> {
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

impl<'a> From<TokenTemplateContext<'a>> for Option<Instruction> {
    #[inline]
    fn from(ctx: TokenTemplateContext<'a>) -> Option<Instruction> {
        ctx.context.into()
    }
}
