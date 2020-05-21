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
    local_use: bool,
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
        unimplemented!(
            "#contract(asset) is not implemented yet                                                                  \
                                                                                                        "
        )
    }
    generate_token_contract(parsed, args).into()
}

fn generate_token_contract(parsed: ItemFn, args: ContractMacroArgs) -> proc_macro2::TokenStream {
    let orig_fn = parsed.clone();
    let sig = parsed.sig; // function signature
    let vis = parsed.vis; // visibility, pub or not
    let fn_name = sig.ident; // function name/identifier
    let fn_args = sig.inputs; // comma separated args
    let fn_return_type = sig.output; // return type

    let return_str = format!("{}", quote! { #fn_return_type });
    assert!(
        return_str.find("Result").is_some(),
        "contract function should return anyhow::Result<impl Serialize> type, returning {} instead",
        return_str
    );

    let arg_idents = extract_arg_idents(fn_args.clone());
    let arg_types = extract_arg_types(fn_args.clone());
    let first_type = arg_types.first().unwrap();

    assert_eq!(
        **first_type,
        syn::parse_str::<Type>("&TokenTemplateContext<'a>").unwrap(),
        "first argument to token contract should be of type &TokenTemplateContext<'a>"
    );

    let (params_type, params_def) = generate_type_params_struct(&arg_idents[1..], &arg_types[1..], &fn_name);

    let body = generate_token_contract_body(&fn_name, &arg_idents[1..]);

    let handler_fn_name = format_ident!("{}_actix", fn_name);

    let handler_impl = quote! {
        pub async fn #handler_fn_name<'a> (
            params: web::Path<TokenCallParams>,
            data: web::Json<#params_type>,
            mut context: TemplateContext<'a>,
        ) -> Result<web::Json<Option<ContractTransaction>>, ApiError> {
            #body
        }
    };

    let tari_validator_node = if args.local_use {
        format_ident!("crate")
    } else {
        format_ident!("tari_validator_node")
    };

    return quote! {
        #orig_fn

        pub mod #handler_fn_name {
            use super::*;
            use #tari_validator_node::{
                api::errors::{ApiError, ApplicationError},
                db::models::transactions::*,
                template::actix::*,
            };
            use log::info;
            use actix_web::web;

            #params_def

            #handler_impl
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
// // extract and transform parameters
// let asset_id = params.asset_id(&context.template_id)?;
// let token_id = params.token_id(&context.template_id)?;
// let asset = match context.load_asset(asset_id).await? {
//     None => return Err(ApplicationError::bad_request("Asset ID not found").into()),
//     Some(asset) => asset,
// };
// let token = match context.load_token(token_id).await? {
//     None => return Err(ApplicationError::bad_request("Token ID not found").into()),
//     Some(token) => token,
// };
// let params = data.into_inner();
// // create transaction
// let transaction = NewContractTransaction {
//     asset_state_id: asset.id,
//     token_id: Some(token.id),
//     template_id: context.template_id.clone(),
//     params: serde_json::to_value(&params)
//         .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
//     contract_name: "transfer_token".to_string(),
//     ..NewContractTransaction::default()
// };
// context.create_transaction(transaction).await?;
// // create context
// let mut context = TokenTemplateContext::new(context, asset.clone(), token.clone());

// // TODO: move following outside of actix request lifecycle
// // run contract
// let result = transfer_token(&context, params.owner_pubkey).await?;
// // update transaction
// let result = serde_json::to_value(result).map_err(|err| {
//     ApplicationError::bad_request(format!("Failed to serialize contract result: {}", err).as_str())
// })?;
// let data = UpdateContractTransaction {
//     result: Some(result),
//     status: Some(TransactionStatus::Commit),
// };
// context.update_transaction(data).await?;
// // There must be transaction - otherwise we would fail on previous call
// return Ok(web::Json(context.into()))
fn generate_token_contract_body(fn_name: &syn::Ident, fn_args: &[Box<Pat>]) -> proc_macro2::TokenStream {
    quote! {
        // extract and transform parameters
        let asset_id = params.asset_id(&context.template_id)?;
        let token_id = params.token_id(&context.template_id)?;
        let asset = match context.load_asset(asset_id).await? {
            None => return Err(ApplicationError::bad_request("Asset ID not found").into()),
            Some(asset) => asset,
        };
        let token = match context.load_token(token_id).await? {
            None => return Err(ApplicationError::bad_request("Token ID not found").into()),
            Some(token) => token,
        };
        let params = data.into_inner();
        // create transaction
        let transaction = NewContractTransaction {
            asset_state_id: asset.id,
            token_id: Some(token.id),
            template_id: context.template_id.clone(),
            params: serde_json::to_value(&params)
                .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
            contract_name: "transfer_token".to_string(),
            ..NewContractTransaction::default()
        };
        context.create_transaction(transaction).await?;
        // create context
        let mut context = TokenTemplateContext::new(context, asset.clone(), token.clone());

        // TODO: move following outside of actix request lifecycle
        // run contract
        let result = #fn_name (&context, #( params.#fn_args ),*).await?;
        // update transaction
        let result = serde_json::to_value(result).map_err(|err| {
            ApplicationError::bad_request(format!("Failed to serialize contract result: {}", err).as_str())
        })?;
        let data = UpdateContractTransaction {
            result: Some(result),
            status: Some(TransactionStatus::Commit),
        };
        context.update_transaction(data).await?;
        // There must be transaction - otherwise we would fail on previous call
        return Ok(web::Json(context.into()));
    }
}

// Output:
// #[derive(Serialize, Deserialize)]
// pub struct TransferTokenPayload {
//     owner_pubkey: Pubkey,
// }
fn generate_type_params_struct(
    fn_arg_idents: &[Box<Pat>],
    fn_arg_types: &[Box<Type>],
    fn_name: &syn::Ident,
) -> (Type, proc_macro2::TokenStream)
{
    let mut types = vec![];

    for (i, t) in fn_arg_idents.into_iter().zip(fn_arg_types.into_iter()) {
        types.push(quote! {
            #i: #t
        });
    }
    let name: Type = syn::parse_str(format!("Params_{}", fn_name).as_str()).unwrap();
    let definition = quote! {
        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize)]
        pub struct #name {
            #(#types),*
        }
    };

    (name, definition)
}
