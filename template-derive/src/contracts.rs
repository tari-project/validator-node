use super::*;
use quote::{format_ident, quote};
use syn::Type;

pub(crate) fn generate(contracts: &Vec<ContractImpl>, opts: &ContractsOpt) -> proc_macro2::TokenStream {
    let mod_name = format_ident!("{}_impl", opts.ident.to_string().to_lowercase());
    let actix_routes = generate_actix_routes(contracts, opts);
    let contracts_impls = generate_contracts_impls(contracts, opts);
    let actor = generate_actor_msg(opts);

    quote! {
        pub mod #mod_name {
            use super::*;
            use crate::{
                api::errors::ApiError,
                db::models::consensus::instructions::*,
                template::{context::*, actors::*},
                types::{TokenID, TemplateID},
            };
            use actix::prelude::*;

            #actix_routes

            #contracts_impls

            #actor
        }
    }
}

fn generate_actix_routes(contracts: &Vec<ContractImpl>, opts: &ContractsOpt) -> proc_macro2::TokenStream {
    let entity = if opts.token { "token" } else { "asset" };
    let ident = &opts.ident;
    let urls = contracts.iter().map(|c| format!("/{}", c.method));
    let handlers = contracts.iter().map(|c| c.web_handler.clone());
    quote! {
        use actix_web::web;
        impl Contracts for #ident {
            fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig) {
                log::info!("template={}, installing {} APIs", #entity, tpl);
                #( scope.service(web::resource(#urls).route(web::post().to(#handlers))) );* ;
            }
        }
    }
}

fn generate_contracts_impls(contracts: &Vec<ContractImpl>, opts: &ContractsOpt) -> proc_macro2::TokenStream {
    let template: Type = syn::parse_str(opts.template.as_str()).unwrap();
    let variants = contracts.iter().map(|c| c.variant_ident.clone());
    let methods = contracts.iter().map(|c| c.method.clone());
    let instruction_context = instruction_context(opts);
    let call_result = call_result(opts);
    let id_gen: syn::Expr = if opts.token {
        syn::parse_str("instruction.token_id.clone().unwrap()").unwrap()
    } else {
        syn::parse_str("instruction.asset_id.clone()").unwrap()
    };
    quote! {
        impl TokenContracts {
            pub async fn call(self, mut context: #instruction_context<#template>) -> #call_result {
                let value = match self {
                    #(
                        #variants ( params ) => {
                            let result = Self::#methods(&mut context, params).await?;
                            serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?
                        }
                    ),*
                };
                Ok((value, context))
            }
            pub fn into_message(self, instruction: Instruction) -> Msg {
                Msg {
                    params: self,
                    id: #id_gen,
                    instruction
                }
            }
        }
    }
}

fn generate_actor_msg(opts: &ContractsOpt) -> proc_macro2::TokenStream {
    let template: Type = syn::parse_str(opts.template.as_str()).unwrap();
    let ident = &opts.ident;
    let id_type: Type = if opts.token {
        syn::parse_str("TokenID").unwrap()
    } else {
        syn::parse_str("AssetID").unwrap()
    };
    let call_result = call_result(opts);
    let instruction_context = instruction_context(opts);
    quote! {
        /// Actor's message is input parameters combined with Instruction
        #[derive(Message, Clone)]
        #[rtype(result = "Result<(),TemplateError>")]
        pub struct Msg {
            id: #id_type,
            params: #ident,
            instruction: Instruction,
        }

        impl ContractCallMsg for Msg {
            type Params = #ident;
            type Template = #template;
            type CallResult = impl Future<Output=#call_result>;
            type Context = #instruction_context<Self::Template>;
            type ContextFuture = impl Future<Output=Result<Self::Context, TemplateError>>;

            fn instruction(&self) -> Instruction {
                self.instruction.clone()
            }
            fn call(self, context: Self::Context) -> Self::CallResult {
                self.params.clone().call(context)
            }
            fn init_context(self, ctx: TemplateContext<Self::Template>) -> Self::ContextFuture {
                #instruction_context::init(ctx, self.instruction, self.id)
            }
        }
    }
}

fn instruction_context(opts: &ContractsOpt) -> Type {
    if opts.token {
        syn::parse_str("TokenInstructionContext").unwrap()
    } else if opts.asset {
        syn::parse_str("AssetInstructionContext").unwrap()
    } else {
        unimplemented!("Only token OR asset options supported");
    }
}

fn call_result(opts: &ContractsOpt) -> Type {
    if opts.token {
        syn::parse_str(format!("TokenCallResult<{}>", opts.template).as_str()).unwrap()
    } else {
        syn::parse_str(format!("AssetCallResult<{}>", opts.template).as_str()).unwrap()
    }
}
