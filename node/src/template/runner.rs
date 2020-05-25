use super::Template;
use actix::prelude::*;

trait TemplateRunner {
    type Template: Template;
}

pub struct TemplateRunnerContext<T: Template> {
    template: T,
}

impl<T: Template> TemplateRunnerContext<T> {
    pub fn new(template: T) -> Self {
        Self { template }
    }
}

impl<T: Template + 'static> Unpin for TemplateRunnerContext<T> {}

impl<T: Template + 'static> TemplateRunner for TemplateRunnerContext<T> {
    type Template = T;
}

impl<T: Template + 'static> Actor for TemplateRunnerContext<T> {
    type Context = Context<Self>;
}

// issue_tokens

// // create asset context
// let asset = match context.load_asset(asset_id.clone()).await? {
//     None => return Err(ApplicationError::bad_request("Asset ID not found").into()),
//     Some(asset) => asset,
// };
// let mut context = AssetInstructionContext::new(context, asset.clone());

// // TODO: move following outside of actix request lifecycle
// // TODO: instruction needs to be able to run in an encapsulated way and return NewTokenStateAppendOnly and
// // NewAssetStateAppendOnly vecs       as the consensus workers need to be able to run an instruction set
// // and confirm the resulting state matches run contract
// let result = issue_tokens(&context, data.token_ids).await?;
// // update instruction after contract executed
// let result = serde_json::to_value(result).map_err(|err| {
//     ApplicationError::bad_request(format!("Failed to serialize contract result: {}", err).as_str())
// })?;
// let data = UpdateInstruction {
//     // TODO: Instruction should not be run at this point in consensus
//     // result: Some(result),
//     status: Some(InstructionStatus::Commit),
//     ..UpdateInstruction::default()
// };
// context.update_instruction(data).await?;

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
