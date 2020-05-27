use super::*;
use quote::{format_ident, quote};
use syn::Type;

pub(crate) struct ContractImpl {
    pub method: syn::Ident,
    pub variant_ident: Type,
    pub params: Type,
    pub tokens: proc_macro2::TokenStream,
    pub web_handler: Type,
}

impl ContractImpl {
    pub(crate) fn generate(variant: &ContractsVariant, opts: &ContractsOpt) -> Self {
        let method = format_ident!("{}", variant.method.as_ref().unwrap());
        let template: Type = syn::parse_str(opts.template.as_str()).unwrap();
        let mod_name = format_ident!("{}_actix", method);
        let web_handler: Type = syn::parse_str(format!("{}::web_handler", mod_name).as_str()).unwrap();
        let params = variant.fields.fields.get(0).unwrap().ty.clone();
        let variant_ident = syn::parse_str(format!("{}::{}", opts.ident, variant.ident).as_str()).unwrap();

        let web = generate_web_body(&method, &template, &params, &opts.ident);
        let from_impl = generate_from_params(&params, &variant_ident, &opts.ident);

        let tokens = quote! {
            pub mod #mod_name {
                use super::*;
                // TODO: fix this to let using in outer crates
                use crate::{
                    api::errors::{ApiError, ApplicationError},
                    db::models::consensus::instructions::*,
                    template::{context::*, actors::*},
                };
                use actix_web::web;

                #from_impl

                #web
            }
        };

        Self {
            web_handler,
            tokens,
            method,
            params,
            variant_ident,
        }
    }
}

fn generate_web_body(
    fn_name: &syn::Ident,
    template: &Type,
    params: &Type,
    contracts: &syn::Ident,
) -> proc_macro2::TokenStream
{
    let fn_name_string = format!("{}", fn_name);
    quote! {
        pub async fn web_handler (
            params: web::Path<TokenCallParams>,
            data: web::Json<#params>,
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
            let contract: #contracts = data.clone().into();
            let message = contract.into_message(instruction.clone());
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

fn generate_from_params(params: &Type, variant_ident: &Type, contracts: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        impl From<#params> for #contracts {
            fn from(params: #params) -> Self {
                #variant_ident(params)
            }
        }
    }
}
