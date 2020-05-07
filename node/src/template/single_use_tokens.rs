use super::{actix::*, AssetTemplateContext, Contracts, Template, TemplateContext, TokenTemplateContext};
use crate::{
    db::models::{NewToken, Token, TokenStatus},
    types::{Pubkey, TemplateID, TokenID},
};
use anyhow::{bail, Result};

/// ***************** Asset contracts *******************

//#[derive(Contracts)]
pub enum AssetContracts {
    //#[contract(issue_tokens)]
    IssueTokens,
}

//#[contract(asset)]
async fn issue_tokens<'a>(
    context: AssetTemplateContext<'a>,
    owner_pub_key: Pubkey,
    token_ids: Vec<TokenID>,
) -> Result<Vec<Token>>
{
    let mut tokens = Vec::with_capacity(token_ids.len());
    let asset = &context.asset;
    let new_token = move |token_id| NewToken {
        token_id,
        owner_pub_key: owner_pub_key.clone(),
        asset_state_id: asset.id.clone(),
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
async fn transfer_token<'a>(context: TokenTemplateContext<'a>, user_pubkey: Pubkey) -> Result<Token> {
    let mut token = context.token.clone();
    if token.status == TokenStatus::Retired {
        bail!("Tried to transfer already used token");
    }
    token
        .additional_data_json
        .as_object_mut()
        .map(|obj| obj.insert("user_pubkey".into(), user_pubkey.into()))
        .ok_or(anyhow::anyhow!("Corrupt token: {}", token.id))?;
    context.update_token(&token).await?;
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
    use crate::{api::utils::errors::ApiError, db::models::transaction::*};
    use serde::{Deserialize, Serialize};

    ////// impl #[contract(asset)] for issue_tokens()

    #[derive(Serialize, Deserialize)]
    struct IssueTokensPayload {
        token_ids: Vec<TokenID>,
        owner_pub_key: Pubkey,
    }

    // wrapper will convert from actix types into Rust,
    // create transactions writing RPC params,
    // returning transaction
    async fn issue_tokens_actix<'a>(
        params: web::Path<AssetCallParams>,
        data: web::Json<IssueTokensPayload>,
        context: TemplateContext<'a>,
    ) -> Result<web::Json<ContractTransaction>, ApiError>
    {
        // extract and transform parameters
        let asset_id = params.asset_id(&context.template_id)?;
        let asset = match context.load_asset(asset_id).await? {
            None => return Err(ApiError::bad_request("Asset ID not found")),
            Some(asset) => asset,
        };
        // create context
        let context = AssetTemplateContext::new(context, asset.clone());
        let params = data.into_inner();
        let transaction = NewContractTransaction {
            asset_state_id: asset.id,
            template_id: context.template_id.clone(),
            params: serde_json::to_value(&params)
                .map_err(|err| ApiError::bad_request(format!("Contract params error: {}", err).as_str()))?,
            contract_name: "issue_tokens".to_string(),
            ..NewContractTransaction::default()
        };
        // create transaction
        let mut transaction = context.create_transaction(transaction).await?;

        // TODO: move following outside of actix request lifecycle
        // run contract
        let result = issue_tokens(context, params.owner_pub_key, params.token_ids).await?;
        // update transaction after contract executed
        transaction.result = serde_json::to_value(result)
            .map_err(|err| ApiError::bad_request(format!("Contract params error: {}", err).as_str()))?;
        transaction.status = TransactionStatus::Commit;
        Ok(web::Json(transaction))
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
        context: TemplateContext<'a>,
    ) -> Result<web::Json<ContractTransaction>, ApiError>
    {
        // extract and transform parameters
        let asset_id = params.asset_id(&context.template_id)?;
        let asset = match context.load_asset(asset_id).await? {
            None => return Err(ApiError::bad_request("Asset ID not found")),
            Some(asset) => asset,
        };
        let token_id = params.token_id(&context.template_id)?;
        let token = match context.load_token(token_id).await? {
            None => return Err(ApiError::bad_request("Token ID not found")),
            Some(token) => token,
        };
        let params = data.into_inner();
        // create context
        let context = TokenTemplateContext::new(context, asset.clone(), token.clone());
        // create transaction
        let transaction = NewContractTransaction {
            asset_state_id: asset.id,
            token_id: Some(token.id),
            template_id: context.template_id.clone(),
            params: serde_json::to_value(&params)
                .map_err(|err| ApiError::bad_request(format!("Contract params error: {}", err).as_str()))?,
            contract_name: "transfer_token".to_string(),
            ..NewContractTransaction::default()
        };
        let mut transaction = context.create_transaction(transaction).await?;

        // TODO: move following outside of actix request lifecycle
        // run contract
        let result = transfer_token(context, params.user_pubkey).await?;
        // update transaction
        transaction.result = serde_json::to_value(result)
            .map_err(|err| ApiError::bad_request(format!("Contract params error: {}", err).as_str()))?;
        transaction.status = TransactionStatus::Commit;
        Ok(web::Json(transaction))
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
