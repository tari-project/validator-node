use darling::{ast::Data, Error, FromDeriveInput, FromField, FromVariant};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(contracts), supports(enum_any), forward_attrs(allow, doc, cfg))]
struct ContractsOpt {
    ident: syn::Ident,
    data: darling::ast::Data<ContractsVariant, darling::util::Ignored>,
    template: String,
    #[darling(default)]
    token: bool,
    #[darling(default)]
    asset: bool,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(contract))]
struct ContractsVariant {
    ident: syn::Ident,
    fields: darling::ast::Fields<ContractsVariantFields>,
    #[darling(default)]
    method: Option<String>,
}

#[derive(Debug, FromField)]
struct ContractsVariantFields {
    ty: syn::Type,
}

#[proc_macro_derive(Contracts, attributes(contracts, contract))]
pub fn derive_contracts(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item);
    derive_contracts_impl(input).into()
}

fn derive_contracts_impl(input: DeriveInput) -> proc_macro2::TokenStream {
    let opts: ContractsOpt = match ContractsOpt::from_derive_input(&input) {
        Ok(attrs) => attrs,
        Err(e) => return e.write_errors().into(),
    };
    if !(opts.asset ^ opts.token) {
        let msg = format!(
            "#[derive(Contracts)]: contract type #[contracts(..)] attribute: one of token or asset should be specified"
        );
        return Error::custom(msg.as_str()).with_span(&opts.ident).write_errors().into();
    } else if opts.asset {
        let msg = format!("#[derive(Contracts)]: contract type `asset` is not supprted yet");
        return Error::custom(msg.as_str()).with_span(&opts.ident).write_errors().into();
    }
    let mut web_handlers = vec![];
    if let Data::Enum(variants) = &opts.data {
        for contract in variants {
            if contract.method.is_none() {
                return Error::custom(
                    "#[derive(Contracts)]: variant requires attribute #[contract(method=..)] to be defined",
                )
                .with_span(&contract.ident)
                .write_errors()
                .into();
            } else if !contract.fields.is_tuple() || contract.fields.len() > 1 {
                return Error::custom("#[derive(Contracts)]: variant can be defined only on single element tuple")
                    .with_span(&contract.ident)
                    .write_errors()
                    .into();
            }
            web_handlers.push(ContractImpl::generate(contract, &opts));
        }
    } else {
        return Error::unexpected_type("#[derive(Contracts)] can only be applied to enum")
            .with_span(&opts.ident)
            .write_errors()
            .into();
    };

    let contracts_impl = contracts::generate(&web_handlers, &opts);

    let contracts = web_handlers.into_iter().map(|c| c.tokens);

    quote! {
        #( #contracts )*

        #contracts_impl
    }
}

mod contract;
pub(crate) use contract::ContractImpl;
mod contracts;

#[cfg(test)]
mod test {
    use super::*;

    const BAD: &[&str] = &[
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token)]
struct NotSupported {}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token)]
struct NotSupported {
    #[contract(method="field1")]
    field1: String
}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token)]
enum NotSupported {
    OptionOne(String),
}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token)]
enum NotSupported {
    #[contract(method="option_one")]
    OptionOne,
}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token,asset)]
enum NotSupported {
    #[contract(method="option_one")]
    OptionOne(String),
}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(template="Template")]
enum NotSupported {
    #[contract(method="option_one")]
    OptionOne(String),
}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(token)]
enum NotSupported {
    #[contract(method="option_one")]
    OptionOne(String),
}
        "###,
    ];

    #[test]
    fn bad_templates() {
        for input in BAD {
            let parsed: syn::DeriveInput = syn::parse_str(*input).expect(format!("Failed to parse {}", input).as_str());
            assert!(ContractsOpt::from_derive_input(&parsed).is_err(), "{}", input);
        }
    }

    const GOOD: &[&str] = &[
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token)]
enum Supported {}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token)]
enum Supported {}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token)]
enum Supported {
    #[contract(method="option_one")]
    OptionOne(String),
}
        "###,
        r###"
#[derive(Contracts)]
#[contracts(template="Template",token)]
enum Supported {
    #[contract(method="option_one")]
    OptionOne(String),
    #[contract(method="option_two")]
    OptionTwo(String),
}
        "###,
    ];

    #[test]
    fn good_templates() {
        for input in GOOD {
            let parsed: syn::DeriveInput = syn::parse_str(*input).expect(format!("Failed to parse {}", input).as_str());
            let result = ContractsOpt::from_derive_input(&parsed);
            assert!(result.is_ok(), "{} -> {:?}", input, result);
        }
    }

    #[test]
    fn snapshot() {
        let input = r#"
#[derive(Contracts, Serialize, Deserialize, Clone)]
#[contracts(template="SingleUseTokenTemplate",token)]
pub enum TokenContracts {
    #[contract(method="sell_token")]
    SellToken(SellTokenParams),
    #[contract(method="sell_token_lock")]
    SellTokenLock(SellTokenLockParams),
    #[contract(method="transfer_token")]
    TransferToken(TransferTokenParams),
}
        "#;

        let parsed: syn::DeriveInput = syn::parse_str(input).expect(format!("Failed to parse {}", input).as_str());
        let result = ContractsOpt::from_derive_input(&parsed);
        assert!(result.is_ok(), "{} -> {:?}", input, result);

        let output = derive_contracts_impl(parsed);
        println!("{}", output);
        //            .expect(format!("Failed to parse output{}", input).as_str());
    }
}
