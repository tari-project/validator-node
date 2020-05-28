//! InstructionContext provides access to contextual request data
//!
//! InstructionContext is always supplied as first parameter to Smart Contract implementation

use super::{Template, TemplateError, TemplateRunner, LOG_TARGET};
use crate::{
    db::{
        models::{
            consensus::instructions::*,
            tokens::{NewToken, Token, UpdateToken},
            wallet::Wallet,
            AssetState,
        },
        utils::errors::DBError,
    },
    processing_err,
    types::*,
    validation_err,
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
/// [`TemplateContext::addr()`] allows to send [`actix::Message`] to [TemplateContextRunner]
/// via [actix::Actor] trait
///
/// [actix_web::Handler], [actix::Message], [actix::Handler] traits usually implemented by attribute macro
/// [validator_template_macros::contract]
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
    pub async fn create_instruction(&self, mut data: NewInstruction) -> Result<Instruction, TemplateError> {
        if data.id == InstructionID::default() {
            // TODO: NodeID should be provided in context
            // TODO: There should be better way
            data.id = InstructionID::new(NodeID::stub()).map_err(anyhow::Error::from)?;
        }
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
        let instruction = Instruction::load(instruction.id, &client).await?;
        Ok(InstructionContext {
            client,
            instruction,
            template_context: self.clone(),
        })
    }

    /// Utility handler for actors when Instruction has failed
    pub async fn instruction_failed(self, instruction: Instruction, error: String) -> Result<(), TemplateError> {
        log::error!(
            target: LOG_TARGET,
            "template={}, instruction={}, Instruction processing failed {}",
            instruction.template_id,
            instruction.id,
            error
        );
        let context = self.instruction_context(instruction.clone()).await;
        let error = match context {
            Ok(mut context) => context
                .transition(ContextEvent::ProcessingFailed {
                    result: serde_json::json!({ "error": error }),
                })
                .await
                .err(),
            Err(err) => Some(err),
        };
        if let Some(error) = error {
            log::error!(
                target: LOG_TARGET,
                "template={}, instruction={}, Non recoverable processing error {}",
                instruction.template_id,
                instruction.id,
                error
            );
            return Err(error);
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

use super::actors::{ContractCallMsg, MessageResult};

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

    /// Create and return token
    pub async fn create_token(&self, data: NewToken) -> Result<Token, TemplateError> {
        let id = Token::insert(data, &self.client).await?;
        Ok(Token::load(id, &self.client).await?)
    }

    /// Create token_append_only_state associated with current [Instruction],
    /// returns updated token
    pub async fn update_token(&self, token: Token, data: UpdateToken) -> Result<Token, TemplateError> {
        Ok(token.update(data, &self.instruction, &self.client).await?)
    }

    /// Load token by [TokenID]
    pub async fn load_token(&self, id: TokenID) -> Result<Option<Token>, TemplateError> {
        Ok(Token::find_by_token_id(&id, &self.client).await?)
    }

    /// Load asset by [AssetID]
    pub async fn load_asset(&self, id: AssetID) -> Result<Option<AssetState>, TemplateError> {
        Ok(AssetState::find_by_asset_id(&id, &self.client).await?)
    }

    /// Move current context's [Instruction] to a new state applying [ContextEvent]
    // TODO: consider using https://github.com/fitzgen/state_machine_future for state machine impl
    pub async fn transition(&mut self, event: ContextEvent) -> Result<(), TemplateError> {
        log::trace!(
            target: LOG_TARGET,
            "template={}, instruction={}, transition event {:?}",
            T::id(),
            self.instruction.id,
            event
        );
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

    /// Creates [Instruction] as a child to current instruction
    pub async fn create_subinstruction<D: serde::Serialize>(
        &self,
        contract_name: String,
        data: D,
    ) -> Result<Instruction, TemplateError>
    {
        let initiating_node_id = self.instruction.initiating_node_id;
        let id = InstructionID::new(initiating_node_id).map_err(anyhow::Error::from)?;
        let params = serde_json::to_value(data).map_err(anyhow::Error::from)?;
        let new = NewInstruction {
            id,
            parent_id: Some(self.instruction.id),
            initiating_node_id,
            asset_id: self.instruction.asset_id.clone(),
            token_id: self.instruction.token_id.clone(),
            template_id: self.instruction.template_id,
            contract_name,
            status: InstructionStatus::Scheduled,
            params,
            ..Default::default()
        };
        Ok(self.template_context.create_instruction(new).await?)
    }

    /// Send message [ContractCallMsg] to subcontract and wait for subcontract to finish
    /// ContractCallMsg is usually autoimplemented by #[derive(Contracts)] on enum `E`
    /// (provided by contract developer), see [`crate::template::actors`] for details.
    /// Message can be created from a contract enum, which when derived has
    /// E::into_message([Instruction]) method
    pub async fn defer<M>(&self, msg: M) -> Result<(), TemplateError>
    where M: ContractCallMsg<Template = T, Result = MessageResult> + 'static {
        let _ = self.template_context.addr().send(msg).await?;
        Ok(())
    }

    /// Create temporary wallet for accepting payment in transaction
    /// Method will return temp_wallet [Pubkey]
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

    /// Check balance on a wallet identified by wallet_key
    pub async fn check_balance(&self, pubkey: &Pubkey) -> Result<i64, TemplateError> {
        let wallet = Wallet::select_by_key(pubkey, &self.client).await?;
        Ok(wallet.balance)
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
    pub async fn init(
        ctx: TemplateContext<T>,
        instruction: Instruction,
        asset_id: AssetID,
    ) -> Result<Self, TemplateError>
    {
        let context = ctx.instruction_context(instruction).await?;
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
    pub async fn init(
        ctx: TemplateContext<T>,
        instruction: Instruction,
        token_id: TokenID,
    ) -> Result<Self, TemplateError>
    {
        let context = ctx.instruction_context(instruction).await?;
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

    /// Create token_append_only_state associated with current [Instruction] and token,
    /// returns updated token
    pub async fn update_token(&mut self, data: UpdateToken) -> Result<(), TemplateError> {
        let token = self.token.clone();
        self.token = token
            .update(data, &self.context.instruction, &self.context.client)
            .await?;
        Ok(())
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::test::utils::{TestTemplate, test_db_client, builders::TokenContextBuilder};

    #[actix_rt::test]
    async fn instruction_failed() {
        let log_level = log::max_level();
        // diable logging as we expect some log errors here
        log::set_max_level(log::LevelFilter::Off);
        let (client, _lock) = test_db_client().await;
        let mut token_ctx: TokenInstructionContext<TestTemplate> = TokenContextBuilder::default().build().await.unwrap();
        let instruction = token_ctx.context.instruction.clone();
        let instruction_id = instruction.id.clone();
        let context = token_ctx.context.template_context.clone();
        assert!(context.clone().instruction_failed(instruction, "This should fail".into()).await.is_err());
        let instruction = Instruction::load(instruction_id, &client).await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Scheduled);
        assert!(token_ctx.context.transition(ContextEvent::StartProcessing).await.is_ok());
        assert!(context.instruction_failed(instruction, "This should pass".into()).await.is_ok());
        log::set_max_level(log_level);
    }
}