use tari_template_macro::contract;
use tari_validator_node::{
    template::{errors::TemplateError, *},
    test::utils::{actix_test_pool, builders::*, load_env},
};

#[contract(token)]
async fn simple_contract(_: &mut TokenInstructionContext, input: u32) -> Result<u32, TemplateError> {
    Ok(input)
}

// just check that it compiles
#[actix_rt::test]
async fn test_contract() {
    load_env();
    let mut context = TokenContextBuilder::default().build(actix_test_pool()).await.unwrap();
    let res = simple_contract(&mut context, 1).await.unwrap();
    assert_eq!(res, 1);
}
