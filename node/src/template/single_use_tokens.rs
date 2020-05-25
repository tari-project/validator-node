use super::{
    actix::*,
    errors::TemplateError,
    AssetInstructionContext,
    Contracts,
    Template,
    TemplateContext,
    TokenInstructionContext,
};
use crate::{
    db::models::{NewToken, Token, TokenStatus, UpdateToken},
    types::{Pubkey, TemplateID, TokenID},
    validation_err,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct TokenData {
    pub owner_pubkey: Pubkey,
}

/// ***************** Asset contracts *******************

//#[derive(Contracts)]
pub enum AssetContracts {
    //#[contract(issue_tokens)]
    IssueTokens,
}

//#[contract(asset)]
async fn issue_tokens(context: &AssetInstructionContext, token_ids: Vec<TokenID>) -> Result<Vec<Token>, TemplateError> {
    let mut tokens = Vec::with_capacity(token_ids.len());
    let asset = &context.asset;
    let data = TokenData {
        owner_pubkey: asset.asset_issuer_pub_key.clone(),
    };
    let data = serde_json::to_value(data).map_err(anyhow::Error::from)?;
    let new_token = move |token_id| NewToken {
        token_id,
        asset_state_id: asset.id.clone(),
        initial_data_json: data.clone(),
        ..NewToken::default()
    };
    for data in token_ids.into_iter().map(new_token) {
        if data.token_id.asset_id() != asset.asset_id {
            return validation_err!("Token ID {} does not match asset {}", data.token_id, asset.asset_id);
        }
        let token = context.create_token(data).await?;
        tokens.push(token);
    }
    Ok(tokens)
}

/// ***************** Token contracts *******************

//#[derive(Contracts)]
pub enum TokenContracts {
    //#[contract(sell_token)]
    SellToken,
    //#[contract(transfer_token)]
    TransferToken,
}

#[tari_template_macro::contract(token, local_use)]
/// Initiate sell token instruction
///
/// ### Input Parameters:
/// - price - amount of tari
/// - user_pubkey - new owner of a token
///
/// # Returns:
/// - Temporary wallet pubkey, where user need to transfer price amount of tari's
async fn sell_token(
    context: &mut TokenInstructionContext,
    price: u64,
    user_pubkey: Pubkey,
) -> Result<Pubkey, TemplateError>
{
    let token = context.token.clone();
    if token.status == TokenStatus::Retired {
        return validation_err!("Tried to transfer already used token");
    }
    let wallet = context.create_temp_wallet().await?;
    Ok(wallet)
}

#[tari_template_macro::contract(token, local_use)]
// With token contract TokenInstructionContext is always passed as first argument
async fn transfer_token(context: &mut TokenInstructionContext, user_pubkey: Pubkey) -> Result<Token, TemplateError> {
    let token = context.token.clone();
    if token.status == TokenStatus::Retired {
        return validation_err!("Tried to transfer already used token");
    }
    let append_state_data_json = Some(json!({ "user_pubkey": user_pubkey }));
    let data = UpdateToken {
        append_state_data_json,
        ..Default::default()
    };
    let token = context.update_token(token, data).await?;
    Ok(token)
}

/// **************** TEMPLATE ************

pub struct SingleUseTokenTemplate;
impl Template for SingleUseTokenTemplate {
    type AssetContracts = AssetContracts;
    type TokenContracts = TokenContracts;

    fn id() -> TemplateID {
        1.into()
    }
}

mod expanded_macros {
    use super::*;
    use crate::{
        api::errors::{ApiError, ApplicationError},
        db::models::consensus::instructions::*,
    };
    use actix_web::web;
    use log::info;
    use serde::{Deserialize, Serialize};

    ////// impl #[contract(asset)] for issue_tokens()

    #[derive(Serialize, Deserialize)]
    pub struct IssueTokensPayload {
        token_ids: Vec<TokenID>,
    }

    // wrapper will convert from actix types into Rust,
    // create instructions writing RPC params,
    // returning instruction
    async fn issue_tokens_actix(
        params: web::Path<AssetCallParams>,
        data: web::Json<IssueTokensPayload>,
        context: TemplateContext,
    ) -> Result<web::Json<Instruction>, ApiError>
    {
        // extract and transform parameters
        let asset_id = params.asset_id(context.template_id())?;
        let data = data.into_inner();
        // start instruction
        let instruction = NewInstruction {
            asset_id,
            template_id: context.template_id(),
            params: serde_json::to_value(&data)
                .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
            contract_name: "issue_tokens".to_string(),
            status: InstructionStatus::Scheduled,
            ..NewInstruction::default()
        };
        let instruction = context.create_instruction(instruction).await?;
        // There must be instruction - otherwise we would fail on previous call
        Ok(web::Json(instruction))
    }
    /////// end of impl #[contract]

    ////// impl #[derive(Contracts)] for AssetContracts

    impl Contracts for AssetContracts {
        fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig) {
            info!("template={}, installing assets API issue_tokens", tpl);
            scope.service(web::resource("/issue_tokens").route(web::post().to(issue_tokens_actix)));
        }
    }
    ////// end of #[derive(Contracts)]

    ////// impl #[derive(Contracts)] for TokenContracts

    impl Contracts for TokenContracts {
        fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig) {
            info!("template={}, installing token API transfer_token", tpl);
            scope.service(
                web::resource("/transfer_token").route(web::post().to(transfer_token_actix::transfer_token_actix)),
            );
            scope.service(web::resource("/sell_token").route(web::post().to(sell_token_actix::sell_token_actix)));
        }
    }

    ////// end of #[derive(Contracts)]
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::consensus::instructions::*,
        test::utils::{actix::TestAPIServer, actix_test_pool, builders::*, test_db_client},
        types::AssetID,
    };
    use serde_json::json;

    #[actix_rt::test]
    async fn issue_tokens_positive() {
        let (_client, _lock) = test_db_client().await;
        let template_id = SingleUseTokenTemplate::id();
        let context = AssetContextBuilder {
            template_id,
            ..Default::default()
        }
        .build(actix_test_pool())
        .await
        .unwrap();
        let asset_id = context.asset.asset_id.clone();
        let token_ids: Vec<_> = (0..10).map(|_| TokenID::test_from_asset(&asset_id)).collect();

        let tokens = issue_tokens(&context, token_ids.clone()).await.unwrap();
        for (token, token_id) in tokens.iter().zip(token_ids.iter()) {
            assert_eq!(token.token_id, *token_id);
        }
    }

    #[actix_rt::test]
    async fn issue_tokens_negative() {
        let (_client, _lock) = test_db_client().await;
        let template_id = SingleUseTokenTemplate::id();
        let context = AssetContextBuilder {
            template_id,
            ..Default::default()
        }
        .build(actix_test_pool())
        .await
        .unwrap();
        let asset_id = AssetID::default();
        let token_ids: Vec<_> = (0..10).map(|_| TokenID::test_from_asset(&asset_id)).collect();
        assert!(issue_tokens(&context, token_ids).await.is_err());
    }

    #[actix_rt::test]
    async fn issue_tokens_full_stack() {
        let srv = TestAPIServer::new(SingleUseTokenTemplate::actix_scopes);
        let (client, _lock) = test_db_client().await;

        let tpl = SingleUseTokenTemplate::id();
        let asset_id = AssetID::test_from_template(tpl);
        let token_ids: Vec<_> = (0..10).map(|_| TokenID::test_from_asset(&asset_id)).collect();
        AssetStateBuilder {
            asset_id: asset_id.clone(),
            ..Default::default()
        }
        .build(&client)
        .await
        .unwrap();

        let mut resp = srv
            .asset_call(&asset_id, "issue_tokens")
            .send_json(&json!({ "token_ids": token_ids }))
            .await
            .unwrap();

        assert!(resp.status().is_success());
        let trans: Instruction = resp.json().await.unwrap();
        assert_eq!(trans.status, InstructionStatus::Scheduled);
    }
}
