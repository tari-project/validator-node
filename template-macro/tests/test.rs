use tari_template_macro::contract;
use tari_validator_node::{
    template::*,
    template::errors::TemplateError,
    test_utils::{builders::*, test_db_client},
};

#[contract(token)]
async fn simple_contract<'a>(_: &mut TokenTemplateContext<'a>, input: u32) -> Result<u32, TemplateError> {
    Ok(input)
}

// just check that it compiles
#[actix_rt::test]
async fn test_contract() {
    let (client, _lock) = test_db_client().await;
    let mut context = TokenContextBuilder::default().build(client).await.unwrap();
    let res = simple_contract(&mut context, 1).await.unwrap();
    assert_eq!(res, 1);
}
