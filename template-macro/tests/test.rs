use tari_template_macro::contract;
use tari_validator_node::{
    template::*,
    test_utils::{builders::*, test_db_client},
};
use anyhow::Result;

#[contract(token)]
async fn simple_contract<'a>(_: &TokenTemplateContext<'a>, input: u32) -> Result<u32> {
    Ok(input)
}

// just check that it compiles
#[actix_rt::test]
async fn test_contract() {
    let (client, _lock) = test_db_client().await;
    let context = TokenContextBuilder::default().build(client).await.unwrap();
    let res = simple_contract(&context, 1).await.unwrap();
    assert_eq!(res, 1);
}
