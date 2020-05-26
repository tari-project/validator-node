//! InstructionContext provides access to contextual request data
//!
//! InstructionContext is always supplied as first parameter to Smart Contract implementation

use super::{errors::TemplateError, runner::TemplateRunnerContext, Template};
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
/// It also holding address of TemplateRunnerContext actor, which executes
/// [Template] Instructions
///
/// ## Instruction processing:
/// [`TemplateContext::addr()`] allows to send [`actix::Message`] to [TemplateContextRunner],
/// which implements [`actix::Actor`]
/// actix::Message is implemented 
#[derive(Clone)]
pub struct TemplateContext<T: Template + Clone + 'static> {
    // TODO: this is not secure, we provide access to context to template,
    // To make it safe our templates should be completely sandboxed,
    // having only access to the context methods...
    pub(crate) pool: Arc<Pool>,
    pub(crate) wallets: Arc<Mutex<WalletStore>>,
    pub(crate) address: Multiaddr,
    pub(crate) actor_addr: actix::Addr<TemplateRunnerContext<T>>,
}

impl<T: Template + 'static> TemplateContext<T> {
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
    pub async fn instruction_context(&self, instruction: Instruction) -> Result<InstructionContext, TemplateError> {
        let client = self.pool.get().await.map_err(DBError::from)?;
        Ok(InstructionContext {
            client,
            instruction,
            template_context: self.clone(),
        })
    }

    /// [TemplateRunnerContext] Actor's address, which is responsible for processing [Instruction]s
    #[inline]
    pub fn addr(&self) -> &actix::Addr<TemplateRunnerContext<T>> {
        &self.actor_addr
    }
}


/// Provides environment and methods for Instruction's code to execute
pub struct InstructionContext<T: Template + 'static> {
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

impl<T: Template> InstructionContext<T> {
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
            (InstructionStatus::Scheduled, ContextEvent::StartProcessing) => {
                UpdateInstruction {
                    status: Some(InstructionStatus::Processing),
                    ..UpdateInstruction::default()
                }
            },
            (InstructionStatus::Processing, ContextEvent::ProcessingResult { result }) => {
                UpdateInstruction {
                    result: Some(result),
                    status: Some(InstructionStatus::Pending),
                    ..UpdateInstruction::default()
                }
            },
            (InstructionStatus::Processing, ContextEvent::ProcessingFailed { result }) => {
                UpdateInstruction {
                    result: Some(result),
                    status: Some(InstructionStatus::Invalid),
                    ..UpdateInstruction::default()
                }
            },
            (InstructionStatus::Pending, ContextEvent::Commit) => {
                UpdateInstruction {
                    status: Some(InstructionStatus::Commit),
                    ..UpdateInstruction::default()
                }
            },
            (a, b) => {
                return processing_err!("Invalid Instruction {} status {} transition {:?}", self.instruction.id, a, b);
            }
        };
        self.instruction = self.instruction.clone().update(update, &self.client).await?;
        Ok(())
    }

    /// Updates result and status of [Instruction]
    async fn update_instruction_status(&mut self, status: InstructionStatus) -> Result<(), TemplateError> {
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

/// Provides environment and methods for Instruction's code on asset to execute
pub struct AssetInstructionContext<T: Template + 'static>  {
    context: InstructionContext<T>,
    pub asset: AssetState,
}

impl<T: Template + 'static>  Deref for AssetInstructionContext<T> {
    type Target = InstructionContext<T>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
impl<T: Template + 'static> DerefMut for AssetInstructionContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<T: Template> AssetInstructionContext<T> {
    pub fn new(context: InstructionContext<T>, asset: AssetState) -> Self {
        Self { context, asset }
    }
}

/// Provides environment and methods for Instruction's code on token to execute
pub struct TokenInstructionContext<T: Template + 'static>  {
    context: InstructionContext<T>,
    pub asset: AssetState,
    pub token: Token,
}

impl<T: Template + 'static> Deref for TokenInstructionContext<T> {
    type Target = InstructionContext<T>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
impl<T: Template + 'static> DerefMut for TokenInstructionContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<T: Template> TokenInstructionContext<T> {
    pub fn new(context: InstructionContext<T>, asset: AssetState, token: Token) -> Self {
        Self { context, asset, token }
    }
}
