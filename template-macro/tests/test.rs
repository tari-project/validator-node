use tari_template_macro::contract;
use tari_validator_node::{
    types::TemplateID,
    template::{errors::TemplateError, *},
    test::utils::{builders::*, load_env},
};

#[derive(Clone)]
pub struct Test;
impl Template for Test {
    type AssetContracts = ();
    type TokenContracts = ();

    fn id() -> TemplateID {
        1.into()
    }
}

#[contract(token,template="Test")]
async fn simple_contract(_: &mut TokenInstructionContext<Test>, input: u32) -> Result<u32, TemplateError> {
    Ok(input)
}

// just check that it compiles
#[actix_rt::test]
async fn test_contract() {
    load_env();
    let mut context = TokenContextBuilder::default().build().await.unwrap();
    let res = simple_contract(&mut context, 1).await.unwrap();
    assert_eq!(res, 1);
}
