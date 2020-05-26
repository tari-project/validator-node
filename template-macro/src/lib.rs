use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, punctuated::Punctuated, AttributeArgs, FnArg, ItemFn, Pat, Type};

#[derive(Debug, FromMeta)]
struct ContractMacroArgs {
    #[darling(default)]
    token: bool,
    #[darling(default)]
    asset: bool,
    #[darling(default)]
    internal: bool,
    template: String,
}

#[proc_macro_attribute]
pub fn contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as ItemFn);
    let attrs = parse_macro_input!(attr as AttributeArgs);
    let args = match ContractMacroArgs::from_list(&attrs) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        },
    };
    if args.asset {
        unimplemented!("#contract(asset) is not implemented yet")
    }
    generate_token_contract(parsed, args).into()
}

fn generate_token_contract(parsed: ItemFn, args: ContractMacroArgs) -> proc_macro2::TokenStream {
    let orig_fn = parsed.clone();
    let sig = parsed.sig; // function signature
    let fn_name = sig.ident; // function name/identifier
    let fn_args = sig.inputs; // comma separated args
    let fn_return_type = sig.output; // return type
    let template = format_ident!("{}", args.template);
    let log_target = format!("{}::{}", args.template, fn_name);

    let return_str = format!("{}", quote! { #fn_return_type });
    if return_str.find("Result").is_none() {
        panic!(
            "contract function should return anyhow::Result<impl Serialize> type, returning {} instead",
            return_str
        )
    }

    let arg_idents = extract_arg_idents(fn_args.clone());
    let arg_types = extract_arg_types(fn_args.clone());
    let first_type = arg_types.first().unwrap();

    let first_arg_required_type = format!("&mut TokenInstructionContext<{}>", template);
    if **first_type != syn::parse_str::<Type>(first_arg_required_type.as_str()).unwrap() {
        panic!("first argument to token contract should be of type &mut TokenInstructionContext");
    }

    let params = generate_type_params_struct(&arg_idents[1..], &arg_types[1..]);

    let msg = generate_message_struct();

    let web_impl = generate_token_web_body(&fn_name, &template);

    let mod_name = format_ident!("{}_actix", fn_name);

    let actor_impl = generate_token_actor_body(&arg_idents[1..], &fn_name, &log_target);

    let tari_validator_node = if args.internal {
        format_ident!("crate")
    } else {
        format_ident!("tari_validator_node")
    };

    return quote! {
        #orig_fn

        pub mod #mod_name {
            use super::*;
            use #tari_validator_node::{
                api::errors::{ApiError, ApplicationError},
                db::models::consensus::instructions::*,
                template::actix_web_impl::*,
                template::{TemplateRunner, ContextEvent},
                types::{AssetID, TokenID},
            };
            use log::info;
            use actix_web::web;
            use actix::prelude::*;
            use futures::future::TryFutureExt;

            type ThisActor = TemplateRunner<#template>;

            #params

            #msg

            #web_impl

            #actor_impl
        }
    };
}

fn extract_arg_idents(fn_args: Punctuated<FnArg, syn::token::Comma>) -> Vec<Box<Pat>> {
    return fn_args.into_iter().map(extract_arg_pat).collect::<Vec<_>>();
}

fn extract_arg_pat(a: FnArg) -> Box<Pat> {
    match a {
        FnArg::Typed(p) => p.pat,
        _ => panic!("Not supported on types with `self`!"),
    }
}

fn extract_arg_types(fn_args: Punctuated<FnArg, syn::token::Comma>) -> Vec<Box<Type>> {
    return fn_args.into_iter().map(extract_type).collect::<Vec<_>>();
}

fn extract_type(a: FnArg) -> Box<Type> {
    match a {
        FnArg::Typed(p) => p.ty, // notice `ty` instead of `pat`
        _ => panic!("Not supported on types with `self`!"),
    }
}

// Output:
// pub async fn web_handler (
// params: web::Path<TokenCallParams>,
// data: web::Json<Params>,
// context: web::Data<TemplateContext<#template>>,
// ) -> Result<web::Json<Instruction>, ApiError> {
// extract and transform parameters
// let asset_id = params.asset_id(&context.template_id)?;
// let token_id = params.token_id(&context.template_id)?;
// let asset = match context.load_asset(asset_id).await? {
// None => return Err(ApplicationError::bad_request("Asset ID not found").into()),
// Some(asset) => asset,
// };
// let token = match context.load_token(token_id).await? {
// None => return Err(ApplicationError::bad_request("Token ID not found").into()),
// Some(token) => token,
// };
// let params = data.into_inner();
// create transaction
// let transaction = NewInstruction {
// asset_id: asset.id,
// token_id: Some(token.token_id),
// template_id: context.template_id.clone(),
// params: serde_json::to_value(&params)
// .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
// contract_name: "transfer_token".to_string(),
// ..NewInstruction::default()
// };
// context.create_transaction(transaction).await?;
// create context
// let mut context = TokenInstructionContext::new(context, asset.clone(), token.clone());
// let message = Msg {
// asset_id,
// token_id,
// instruction: instruction.clone(),
// params: data.clone(),
// };
// context
// .addr()
// .try_send(message)
// .map_err(|err| TemplateError::ActorSend {
// source: err.into(),
// TODO: proper handling of unlikely error
// params: serde_json::to_string(&data).unwrap(),
// name: "transfer_token".into(),
// })?;
// There must be transaction - otherwise we would fail on previous call
// return Ok(web::Json(context.into()))
// }
fn generate_token_web_body(fn_name: &syn::Ident, template: &syn::Ident) -> proc_macro2::TokenStream {
    let fn_name_string = format!("{}", fn_name);
    quote! {
        pub async fn web_handler (
            params: web::Path<TokenCallParams>,
            data: web::Json<Params>,
            context: web::Data<TemplateContext<#template>>,
        ) -> Result<web::Json<Instruction>, ApiError> {
            // extract and transform parameters
            let asset_id = params.asset_id(context.template_id())?;
            let token_id = params.token_id(context.template_id())?;
            let data = data.into_inner();
            // create transaction
            let instruction = NewInstruction {
                asset_id: asset_id.clone(),
                token_id: Some(token_id.clone()),
                template_id: context.template_id(),
                params: serde_json::to_value(&data)
                    .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
                contract_name: #fn_name_string .into(),
                status: InstructionStatus::Scheduled,
                ..NewInstruction::default()
            };
            let instruction = context.create_instruction(instruction).await?;
            let message = Msg {
                asset_id,
                token_id,
                instruction: instruction.clone(),
                params: data.clone(),
            };
            context
                .addr()
                .try_send(message)
                .map_err(|err| TemplateError::ActorSend {
                    source: err.into(),
                    // TODO: proper handling of unlikely error
                    params: serde_json::to_string(&data).unwrap(),
                    name: #fn_name_string .into(),
                })?;
            // There must be transaction - otherwise we would fail on previous call
            return Ok(web::Json(instruction));
        }
    }
}

// Output:
// #[derive(Serialize, Deserialize)]
// pub struct Params {
//     owner_pubkey: Pubkey,
// }
fn generate_type_params_struct(fn_arg_idents: &[Box<Pat>], fn_arg_types: &[Box<Type>]) -> proc_macro2::TokenStream {
    let mut types = vec![];

    for (i, t) in fn_arg_idents.into_iter().zip(fn_arg_types.into_iter()) {
        types.push(quote! {
            pub(super) #i: #t
        });
    }

    quote! {
        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize, Clone)]
        pub struct Params {
            #(#types),*
        }
    }
}

// Output:
// /// Actor's message is input parameters combined with Instruction
// #[derive(Message)]
// #[rtype(result = "Result<(),TemplateError>")]
// pub struct Msg {
//     pub(super) asset_id: AssetID,
//     pub(super) token_id: TokenID,
//     pub(super) params: Params,
//     pub(super) instruction: Instruction,
// }
fn generate_message_struct() -> proc_macro2::TokenStream {
    quote! {
        use actix::Message;

        #[derive(Message)]
        #[rtype(result = "Result<(),TemplateError>")]
        pub struct Msg {
            pub(super) asset_id: AssetID,
            pub(super) token_id: TokenID,
            pub(super) params: Params,
            pub(super) instruction: Instruction,
        }
    }
}

// impl Handler<sell_token_actix::Msg> for ThisActor {
//     type Result = ResponseActFuture<Self, Result<(), TemplateError>>;

//     fn handle(&mut self, msg: sell_token_actix::Msg, _ctx: &mut Context<Self>) -> Self::Result {
//         let context = self.context();
//         let instruction = msg.instruction.clone();
//         let token_context_fut =
//             TokenInstructionContext::init(self.context(), msg.instruction.clone(), msg.token_id.clone());
//         log::trace!(target: LOG_TARGET, "template={}, instruction={}, Actor received issue_tokens instruction",
// Self::template_id(), msg.instruction.id);

//         let fut = actix::fut::wrap_future::<_, Self>(
//             async move {
//                 let mut context = token_context_fut.await?;
//                 context.transition(ContextEvent::StartProcessing).await?;
//                 // TODO: instruction needs to be able to run in an encapsulated way and return
//                 // NewTokenStateAppendOnly and NewAssetStateAppendOnly vecs       as the
//                 // consensus workers need to be able to run an instruction set and confirm the
//                 // resulting state matches run contract
//                 let result = sell_token(&mut context, msg.params.price, msg.params.user_pubkey).await?;
//                 // update instruction after contract executed
//                 let result =
//                     serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?;
//                 context.transition(ContextEvent::ProcessingResult { result }).await?;
//                 Ok(())
//             }
//             .or_else(move |err: TemplateError| {
//                 context.instruction_failed(instruction, err)
//             }),
//         );
//         Box::pin(fut)
//     }
// }
fn generate_token_actor_body(
    fn_arg_idents: &[Box<Pat>],
    fn_name: &syn::Ident,
    log_target: &String,
) -> proc_macro2::TokenStream
{
    let fn_name_string = format!("{}", fn_name);
    quote! {
        impl Handler<Msg> for ThisActor {
            type Result = ResponseActFuture<Self, Result<(), TemplateError>>;

            fn handle(&mut self, msg: Msg, _ctx: &mut Context<Self>) -> Self::Result {
                let context = self.context();
                let instruction = msg.instruction.clone();
                let token_context_fut =
                    TokenInstructionContext::init(self.context(), msg.instruction.clone(), msg.token_id.clone());
                log::trace!(target: #log_target, "template={}, instruction={}, Actor received {} instruction", Self::template_id(), msg.instruction.id, #fn_name_string);

                let fut = actix::fut::wrap_future::<_, Self>(
                    async move {
                        let mut context = token_context_fut.await?;
                        context.transition(ContextEvent::StartProcessing).await?;
                        // TODO: instruction needs to be able to run in an encapsulated way and return
                        // NewTokenStateAppendOnly and NewAssetStateAppendOnly vecs       as the
                        // consensus workers need to be able to run an instruction set and confirm the
                        // resulting state matches run contract
                        let result = #fn_name(&mut context, #( msg.params.#fn_arg_idents),* ).await?;
                        // update instruction after contract executed
                        let result =
                            serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?;
                        context.transition(ContextEvent::ProcessingResult { result }).await?;
                        Ok(())
                    }
                    .or_else(move |err: TemplateError| {
                        context.instruction_failed(instruction, err)
                    }),
                );
                Box::pin(fut)
            }
        }
    }
}
