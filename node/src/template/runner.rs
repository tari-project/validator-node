use crate::template::{Template, TemplateContext, InstructionContext};
use crate::template::errors::TemplateError;
use crate::db::models::consensus::Instruction;
use actix::prelude::*;

pub trait TemplateRunner {
    type Template: Template;
}

/// Implements [Actor] for given Template
/// Provides instruction code with [TemplateContext]
pub struct TemplateRunnerContext<T: Template + 'static> {
    template: T,
    context: TemplateContext<T>,
}

impl<T: Template> TemplateRunnerContext<T> {
    pub fn new(template: T, context: TemplateContext<T>) -> Self {
        Self { template, context }
    }

    #[inline]
    pub fn context(&self) -> TemplateContext<T> {
        self.context.clone()
    }
}

impl<T: Template + 'static> Unpin for TemplateRunnerContext<T> {}

impl<T: Template + 'static> TemplateRunner for TemplateRunnerContext<T> {
    type Template = T;
}

impl<T: Template + 'static> Actor for TemplateRunnerContext<T> {
    type Context = Context<Self>;
}


// // create context
// let mut context = TokenInstructionContext::new(context, asset.clone(), token.clone());

// // TODO: move following outside of actix request lifecycle
// // run contract
// let result = #fn_name (&mut context, #( params.#fn_args ),*).await?;
// // update transaction
// let result = serde_json::to_value(result).map_err(|err| {
//     ApplicationError::bad_request(format!("Failed to serialize contract result: {}", err).as_str())
// })?;
// let data = UpdateInstruction {
//     result: Some(result),
//     status: Some(InstructionStatus::Commit),
// };
// context.update_instruction(data).await?;
