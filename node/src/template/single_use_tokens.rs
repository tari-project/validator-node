use super::{actix::*, AssetTemplateContext, Contracts, Template, TemplateContext, TokenTemplateContext};
use crate::{
    db::models::{NewToken, Token, TokenStatus, UpdateToken},
    types::{Pubkey, TemplateID, TokenID},
};
use anyhow::{bail, Result};
use serde_json::json;

/// ***************** Asset contracts *******************

//#[derive(Contracts)]
pub enum AssetContracts {
    //#[contract(issue_tokens)]
    IssueTokens,
}

//#[contract(asset)]
async fn issue_tokens<'a>(
    context: &AssetTemplateContext<'a>,
    user_pubkey: Pubkey,
    token_ids: Vec<TokenID>,
) -> Result<Vec<Token>>
{
    let mut tokens = Vec::with_capacity(token_ids.len());
    let asset = &context.asset;
    let new_token = move |token_id| NewToken {
        token_id,
        asset_state_id: asset.id.clone(),
        initial_data_json: json!({"user_pubkey": user_pubkey}),
        ..NewToken::default()
    };
    for data in token_ids.into_iter().map(new_token) {
        if data.token_id.asset_id() != asset.asset_id {
            bail!("Token ID {} does not match asset {}", data.token_id, asset.asset_id);
        }
        let token = context.create_token(data).await?;
        tokens.push(token);
    }
    Ok(tokens)
}

/// ***************** Token contracts *******************

//#[derive(Contracts)]
pub enum TokenContracts {
    //#[contract(transfer_token)]
    TransferToken,
}

//#[contract(token)]
// With token contract TokenTemplateContext is always passed as first argument
async fn transfer_token<'a>(context: &TokenTemplateContext<'a>, user_pubkey: Pubkey) -> Result<Token> {
    let token = context.token.clone();
    if token.status == TokenStatus::Retired {
        bail!("Tried to transfer already used token");
    }
    let append_state_data_json = Some(json!({"user_pubkey": user_pubkey}));
    let data = UpdateToken { append_state_data_json, ..Default::default() };
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
    use crate::{api::errors::{ApiError, ApplicationError}, db::models::transactions::*};
    use serde::{Deserialize, Serialize};

    ////// impl #[contract(asset)] for issue_tokens()

    #[derive(Serialize, Deserialize)]
    struct IssueTokensPayload {
        token_ids: Vec<TokenID>,
        user_pubkey: Pubkey,
    }

    // wrapper will convert from actix types into Rust,
    // create transactions writing RPC params,
    // returning transaction
    async fn issue_tokens_actix<'a>(
        params: web::Path<AssetCallParams>,
        data: web::Json<IssueTokensPayload>,
        mut context: TemplateContext<'a>,
    ) -> Result<web::Json<Option<ContractTransaction>>, ApiError>
    {
        // extract and transform parameters
        let asset_id = params.asset_id(&context.template_id)?;
        let asset = match context.load_asset(asset_id).await? {
            None => return Err(ApplicationError::bad_request("Asset ID not found").into()),
            Some(asset) => asset,
        };
        let params = data.into_inner();
        // start transaction
        let transaction = NewContractTransaction {
            asset_state_id: asset.id,
            template_id: context.template_id.clone(),
            params: serde_json::to_value(&params)
                .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
            contract_name: "issue_tokens".to_string(),
            ..NewContractTransaction::default()
        };
        context.create_transaction(transaction).await?;
        // create asset context
        let mut context = AssetTemplateContext::new(context, asset.clone());

        // TODO: move following outside of actix request lifecycle
        // run contract
        let result = issue_tokens(&context, params.user_pubkey, params.token_ids).await?;
        // update transaction after contract executed
        let result = serde_json::to_value(result)
            .map_err(|err| ApplicationError::bad_request(format!("Failed to serialize contract result: {}", err).as_str()))?;
        let data = UpdateContractTransaction { result: Some(result), status: Some(TransactionStatus::Commit) };
        context.update_transaction(data).await?;
        // There must be transaction - otherwise we would fail on previous call
        Ok(web::Json(context.into()))
    }
    /////// end of impl #[contract]

    ////// impl #[derive(Contracts)] for AssetContracts

    impl Contracts for AssetContracts {
        fn setup_actix_routes(scope: &mut web::ServiceConfig) {
            scope.service(web::resource("/issue_tokens").route(web::post().to(issue_tokens_actix)));
        }
    }
    ////// end of #[derive(Contracts)]

    //////  impl #[contract(token)] for transfer_token()

    #[derive(Serialize, Deserialize)]
    struct TransferTokenPayload {
        user_pubkey: Pubkey,
    }

    async fn transfer_token_actix<'a>(
        params: web::Path<TokenCallParams>,
        data: web::Json<TransferTokenPayload>,
        mut context: TemplateContext<'a>,
    ) -> Result<web::Json<Option<ContractTransaction>>, ApiError>
    {
        // extract and transform parameters
        let asset_id = params.asset_id(&context.template_id)?;
        let asset = match context.load_asset(asset_id).await? {
            None => return Err(ApplicationError::bad_request("Asset ID not found").into()),
            Some(asset) => asset,
        };
        let token_id = params.token_id(&context.template_id)?;
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
        let result = transfer_token(&context, params.user_pubkey).await?;
        // update transaction
        let result = serde_json::to_value(result)
            .map_err(|err| ApplicationError::bad_request(format!("Failed to serialize contract result: {}", err).as_str()))?;
        let data = UpdateContractTransaction { result: Some(result), status: Some(TransactionStatus::Commit) };
        context.update_transaction(data).await?;
        // There must be transaction - otherwise we would fail on previous call
        Ok(web::Json(context.into()))
    }
    /////// end of impl #[contract]

    ////// impl #[derive(Contracts)] for TokenContracts

    use actix_web::web;

    impl Contracts for TokenContracts {
        fn setup_actix_routes(scope: &mut web::ServiceConfig) {
            scope.service(web::resource("/transfer_token").route(web::post().to(transfer_token_actix)));
        }
    }

    ////// end of #[derive(Contracts)]
}
