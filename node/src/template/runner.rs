

// // create context
// let mut context = TokenTemplateContext::new(context, asset.clone(), token.clone());

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
