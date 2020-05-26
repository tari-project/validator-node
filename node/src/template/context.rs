//! InstructionContext provides access to contextual request data
//!
//! InstructionContext is always supplied as first parameter to Smart Contract implementation

use super::{errors::TemplateError, runner::TemplateRunner, Template};
use crate::{
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
}
