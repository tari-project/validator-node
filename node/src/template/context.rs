//! InstructionContext provides access to contextual request data
//!
//! InstructionContext is always supplied as first parameter to Smart Contract implementation

use super::{errors::TemplateError, runner::TemplateRunner, Template, LOG_TARGET};
use crate::{
    db::{
        models::{
            consensus::instructions::*,
            tokens::{NewToken, Token, UpdateToken},
            AssetState,
        },
        utils::errors::DBError,
    },
    {processing_err, validation_err},
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

/// TemplateContext, is factory for [Instruction] and [InstructionContext]
/// It also holding address of [TemplateRunner] actor, which executes
/// [Template] Instructions
///
/// ## Instruction processing:
/// [`TemplateContext::addr()`] allows to send [`actix::Message`] to [TemplateContextRunner],
/// which implements [`actix::Actor`]
/// [actix::Message]  should is implemented
#[derive(Clone)]
pub struct TemplateContext<T: Template + Clone + 'static> {
    // TODO: this is not secure, we provide access to context to template,
    // To make it safe our templates should be completely sandboxed,
    // having only access to the context methods...
    pub(super) pool: Arc<Pool>,
    pub(super) wallets: Arc<Mutex<WalletStore>>,
    pub(super) node_address: Multiaddr,
    pub(super) actor_address: Option<actix::Addr<TemplateRunner<T>>>,
}

impl<T: Template + Clone + 'static> TemplateContext<T> {
    /// [TemplateID] of current TemplateContext
    #[inline]
    pub fn template_id(&self) -> TemplateID {
        T::id()
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
    pub async fn instruction_context(&self, instruction: Instruction) -> Result<InstructionContext<T>, TemplateError> {
        let client = self.pool.get().await.map_err(DBError::from)?;
        Ok(InstructionContext {
            client,
            instruction,
            template_context: self.clone(),
        })
    }

    /// Utility handler for actors when Instruction has failed
    pub async fn instruction_failed(self, instruction: Instruction, err: TemplateError) -> Result<(), TemplateError> {
        log::error!(
            target: LOG_TARGET,
            "template={}, instruction={}, Instruction processing failed {}",
            instruction.template_id,
            instruction.id,
            err
        );
        let context = self.instruction_context(instruction.clone()).await;
        let err = match context {
            Ok(mut context) => context
                .transition(ContextEvent::ProcessingFailed { result: serde_json::json!({"error": err.to_string()}) })
                .await
                .err(),
            Err(err) => Some(err),
        };
        if let Some(err) = err {
            log::error!(
                target: LOG_TARGET,
                "template={}, instruction={}, Non recoverable processing error {}",
                instruction.template_id,
                instruction.id,
                err
            );
            return Err(err);
        };
        Ok(())
    }

    /// [TemplateRunner] Actor's address, which is responsible for processing [Instruction]s
    #[inline]
    pub fn addr(&self) -> &actix::Addr<TemplateRunner<T>> {
        self.actor_address.as_ref().expect("TemplateRunner")
    }
}

/// Provides environment and methods for Instruction's code to execute
pub struct InstructionContext<T: Template + Clone + 'static> {
    template_context: TemplateContext<T>,
    client: Client,
    instruction: Instruction,
}

#[derive(Debug)]
/// Event for transitioning [Instruction]
/// Instruction's updates triggered via calling
/// InstructionContext::transition(event)
pub enum ContextEvent {
    StartProcessing,
    ProcessingResult { result: serde_json::Value },
    ProcessingFailed { result: serde_json::Value },
    Commit,
}

impl<T: Template + Clone> InstructionContext<T> {
    #[inline]
    pub fn template_id(&self) -> TemplateID {
        T::id()
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

    pub async fn transition(&mut self, event: ContextEvent) -> Result<(), TemplateError> {
        log::trace!(target: LOG_TARGET, "template={}, instruction={}, transition event {:?}", T::id(), self.instruction.id, event);
        let update = match (self.instruction.status, event) {
            (InstructionStatus::Scheduled, ContextEvent::StartProcessing) => UpdateInstruction {
                status: Some(InstructionStatus::Processing),
                ..UpdateInstruction::default()
            },
            (InstructionStatus::Processing, ContextEvent::ProcessingResult { result }) => UpdateInstruction {
                result: Some(result),
                status: Some(InstructionStatus::Pending),
                ..UpdateInstruction::default()
            },
            (InstructionStatus::Processing, ContextEvent::ProcessingFailed { result }) => UpdateInstruction {
                result: Some(result),
                status: Some(InstructionStatus::Invalid),
                ..UpdateInstruction::default()
            },
            (InstructionStatus::Pending, ContextEvent::Commit) => UpdateInstruction {
                status: Some(InstructionStatus::Commit),
                ..UpdateInstruction::default()
            },
            (a, b) => {
                return processing_err!(
                    "Invalid Instruction {} status {} transition {:?}",
                    self.instruction.id,
                    a,
                    b
                );
            },
        };
        self.instruction = self.instruction.clone().update(update, &self.client).await?;
        Ok(())
    }

    pub async fn create_temp_wallet(&mut self) -> Result<Pubkey, TemplateError> {
        let transaction = self.client.transaction().await.map_err(DBError::from)?;
        let wallet_name = self.instruction.id.to_string();
        let wallet = NodeWallet::new(self.template_context.node_address.clone(), wallet_name)?;
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

/// Provides environment and methods for Instruction's code on asset to execute
pub struct AssetInstructionContext<T: Template + Clone + 'static> {
    context: InstructionContext<T>,
    pub asset: AssetState,
}

impl<T: Template + Clone + 'static> Deref for AssetInstructionContext<T> {
    type Target = InstructionContext<T>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
impl<T: Template + Clone + 'static> DerefMut for AssetInstructionContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<T: Template + Clone> AssetInstructionContext<T> {
    pub fn new(context: InstructionContext<T>, asset: AssetState) -> Self {
        Self { context, asset }
    }
    /// Initialize from TemplateContext, instruction and asset_id
    pub async fn init(ctx: TemplateContext<T>, instruction: Instruction, asset_id: AssetID) -> Result<Self, TemplateError> {
        let context = ctx
            .instruction_context(instruction)
            .await?;
        // create asset context
        let asset = match context.load_asset(asset_id).await? {
            None => return validation_err!("Asset ID not found"),
            Some(asset) => asset,
        };
        Ok(Self::new(context, asset))
    }
}

/// Provides environment and methods for Instruction's code on token to execute
pub struct TokenInstructionContext<T: Template + Clone + 'static> {
    context: InstructionContext<T>,
    pub asset: AssetState,
    pub token: Token,
}

impl<T: Template + Clone + 'static> Deref for TokenInstructionContext<T> {
    type Target = InstructionContext<T>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
impl<T: Template + Clone + 'static> DerefMut for TokenInstructionContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<T: Template + Clone> TokenInstructionContext<T> {
    pub fn new(context: InstructionContext<T>, asset: AssetState, token: Token) -> Self {
        Self { context, asset, token }
    }
    /// Initialize from TemplateContext, instruction and token_id
    pub async fn init(ctx: TemplateContext<T>, instruction: Instruction, token_id: TokenID) -> Result<Self, TemplateError> {
        let context = ctx
            .instruction_context(instruction)
            .await?;
        // create asset context
        let asset = match context.load_asset(token_id.asset_id()).await? {
            None => return validation_err!("Asset ID not found"),
            Some(asset) => asset,
        };
        let token = match context.load_token(token_id).await? {
            None => return validation_err!("Token ID not found"),
            Some(asset) => asset,
        };
        Ok(Self::new(context, asset, token))
    }
}
