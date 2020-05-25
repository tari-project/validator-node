//! InstructionContext provides access to contextual request data
//!
//! InstructionContext is always supplied as first parameter to Smart Contract implementation

use super::errors::TemplateError;
use crate::{
    api::errors::{ApiError, ApplicationError},
    db::{
        models::{
            consensus::instructions::*,
            tokens::{NewToken, Token, UpdateToken},
            AssetState,
        },
        utils::errors::DBError,
    },
    processing_err,
    types::*,
    wallet::{NodeWallet, WalletStore},
};
use deadpool_postgres::{Client, Pool};
use multiaddr::Multiaddr;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::Mutex;

/// [Template] context, is factory for [Instuction] and [InstructionContext]
///
/// Fields:
/// - TemplateID
/// - (private) DB connection
#[derive(Clone)]
pub struct TemplateContext {
    pub(crate) template_id: TemplateID,
    // TODO: this is not secure, we provide access to context to template,
    // To make it safe our templates should be completely sandboxed,
    // having only access to the context methods...
    pub(crate) pool: Arc<Pool>,
    pub(crate) wallets: Arc<Mutex<WalletStore>>,
    pub(crate) address: Multiaddr,
}

impl TemplateContext {
    /// [TemplateID] of current TemplateContext
    #[inline]
    pub fn template_id(&self) -> TemplateID {
        self.template_id
    }

    /// Creates [Instruction]
    pub async fn create_instruction(&self, data: NewInstruction) -> Result<Instruction, TemplateError> {
        if data.status != InstructionStatus::Scheduled {
            return processing_err!(
                "Failed to create Instruction in status {}, initial status should be Scheduled",
                data.status
            );
        }
        let client = self.pool.get().await.map_err(DBError::from)?;
        Ok(Instruction::insert(data, &client).await?)
    }

    /// Creates [InstructionContext] which can be used by [InstructionRunner] to process [Instruction]
    pub async fn instruction_context(&self, id: InstructionID) -> Result<InstructionContext, TemplateError> {
        let client = self.pool.get().await.map_err(DBError::from)?;
        let instruction = Instruction::load(id, &client).await?;
        Ok(InstructionContext {
            client,
            instruction,
            template_context: self.clone(),
        })
    }
}

pub struct InstructionContext {
    template_context: TemplateContext,
    client: Client,
    instruction: Instruction,
}

impl InstructionContext {
    #[inline]
    pub fn template_id(&self) -> TemplateID {
        self.template_context.template_id
    }

    pub async fn create_token(&self, data: NewToken) -> Result<Token, TemplateError> {
        let id = Token::insert(data, &self.client).await?;
        Ok(Token::load(id, &self.client).await?)
    }

    pub async fn update_token(&self, token: Token, data: UpdateToken) -> Result<Token, TemplateError> {
        Ok(token.update(data, &self.instruction, &self.client).await?)
    }

    pub async fn load_token(&self, id: TokenID) -> Result<Option<Token>, TemplateError> {
        Ok(Token::find_by_token_id(&id, &self.client).await?)
    }

    pub async fn load_asset(&self, id: AssetID) -> Result<Option<AssetState>, TemplateError> {
        Ok(AssetState::find_by_asset_id(&id, &self.client).await?)
    }

    /// Updates result and status of [Instruction]
    pub async fn update_instruction_status(&mut self, status: InstructionStatus) -> Result<(), TemplateError> {
        if status == InstructionStatus::Scheduled ||
            status == InstructionStatus::Processing ||
            status == InstructionStatus::Invalid
        {
            Instruction::update_instructions_status(&[self.instruction.id], None, status, &self.client).await?;
            self.instruction.status = status;
        } else {
            return processing_err!(
                "Failed to update Instruction status to {} from within InstructionContext",
                status
            );
        }
        Ok(())
    }

    pub async fn create_temp_wallet(&mut self) -> Result<Pubkey, TemplateError> {
        let transaction = self.client.transaction().await.map_err(DBError::from)?;
        let wallet_name = self.instruction.id.to_string();
        let wallet = NodeWallet::new(self.template_context.address.clone(), wallet_name)?;
        let wallet = self
            .template_context
            .wallets
            .lock()
            .await
            .add(wallet, &transaction)
            .await?;
        transaction.commit().await.map_err(DBError::from)?;
        Ok(wallet.public_key_hex())
    }
}

/// Extract [Instruction] from InstructionContext
impl From<InstructionContext> for Instruction {
    #[inline]
    fn from(ctx: InstructionContext) -> Instruction {
        ctx.instruction
    }
}

/// Smart contract request context for asset contracts
///
/// Fields:
/// - InstructionContext
/// - AssetState
pub struct AssetInstructionContext {
    context: InstructionContext,
    pub asset: AssetState,
}

impl Deref for AssetInstructionContext {
    type Target = InstructionContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
impl DerefMut for AssetInstructionContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl AssetInstructionContext {
    pub fn new(context: InstructionContext, asset: AssetState) -> Self {
        Self { context, asset }
    }
}

impl From<AssetInstructionContext> for Instruction {
    #[inline]
    fn from(ctx: AssetInstructionContext) -> Instruction {
        ctx.context.into()
    }
}

/// Smart contract request context for asset contracts
///
/// Fields:
/// - InstructionContext
/// - AssetState
/// - Token
pub struct TokenInstructionContext {
    context: InstructionContext,
    pub asset: AssetState,
    pub token: Token,
}

impl Deref for TokenInstructionContext {
    type Target = InstructionContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
impl DerefMut for TokenInstructionContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl TokenInstructionContext {
    pub fn new(context: InstructionContext, asset: AssetState, token: Token) -> Self {
        Self { context, asset, token }
    }
}

impl From<TokenInstructionContext> for Instruction {
    #[inline]
    fn from(ctx: TokenInstructionContext) -> Instruction {
        ctx.context.into()
    }
}
