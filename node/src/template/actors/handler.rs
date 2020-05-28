use crate::{
    db::models::consensus::instructions::Instruction,
    template::{context::*, Template, TemplateError, TemplateRunner, LOG_TARGET},
};
use actix::prelude::*;
use futures::future::FutureExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Try;

pub type ContractCallResult<C> = Result<(Value, C), TemplateError>;
pub type MessageResult = Result<(), TemplateError>;
pub type AssetCallResult<T> = Result<(Value, AssetInstructionContext<T>), TemplateError>;
pub type TokenCallResult<T> = Result<(Value, TokenInstructionContext<T>), TemplateError>;

/// TokenCallMsg should be implemented by Contract, this would grant
/// auto-implementation of [actix::Handler] for contract messages
///
/// TokenCallMsg implementation is usually derived with proc-macro Contracts on enum:
/// ```ignore
/// #[derive(Contracts)]
/// enum MyContracts { ... }
/// ```
pub trait ContractCallMsg: Clone + Message + Send {
    type Params: Serialize + for<'de> Deserialize<'de> + Clone;
    type Template: Template + 'static;
    type CallResult: Future<Output = ContractCallResult<Self::Context>>;
    type Context: std::ops::DerefMut<Target = InstructionContext<Self::Template>>;
    type ContextFuture: Future<Output = Result<Self::Context, TemplateError>>;

    fn instruction(&self) -> Instruction;
    fn call(self, context: Self::Context) -> Self::CallResult;
    fn init_context(self, ctx: TemplateContext<Self::Template>) -> Self::ContextFuture;
}

/// Actor is accepting TokenCallMsg and tries to perform activity
impl<M, T> Handler<M> for TemplateRunner<T>
where
    T: Template + 'static,
    M: ContractCallMsg<Template = T, Result = MessageResult> + 'static,
    M::Params: Serialize + for<'de> Deserialize<'de> + Clone,
{
    type Result = ResponseActFuture<Self, M::Result>;

    fn handle(&mut self, msg: M, _ctx: &mut Context<Self>) -> Self::Result {
        let context = self.context();
        let instruction = msg.instruction();
        let token_context_fut = msg.clone().init_context(self.context());
        log::trace!(
            target: LOG_TARGET,
            "template={}, instruction={}, Actor received issue_tokens instruction",
            Self::template_id(),
            msg.instruction().id
        );

        // TODO: make whole execution in a single DB transaction
        let fut = actix::fut::wrap_future::<_, Self>(
            async move {
                let mut context = token_context_fut.await?;
                context.transition(ContextEvent::StartProcessing).await?;
                // TODO: instruction needs to be able to run in an encapsulated way and return
                // NewTokenStateAppendOnly and NewAssetStateAppendOnly vecs       as the
                // consensus workers need to be able to run an instruction set and confirm the
                // resulting state matches run contract
                let (result, mut context) = msg.call(context).await?;
                context.transition(ContextEvent::ProcessingResult { result }).await?;
                // TODO: commit DB transaction
                Ok(())
            }
            .then(move |res: Result<(), TemplateError>| async move {
                match res {
                    // update instruction after contract executed
                    Ok(()) => M::Result::from_ok(()),
                    Err(err) => {
                        context.instruction_failed(instruction, err.to_string()).await;
                        // TODO: commit DB transaction
                        M::Result::from_error(err)
                    },
                }
            }),
        );
        Box::pin(fut)
    }
}
