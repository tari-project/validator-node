use super::{actix::*, Contracts, Template, TemplateContext, TokenTemplateContext, AssetTemplateContext};
use crate::{
    db::models::{AssetState, NewToken, Token, TokenStatus},
    types::{Pubkey, TemplateID, TokenID},
};
use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::json;

/// ***************** Asset contracts *******************

//#[derive(Contracts)]
enum AssetContracts {
    //#[contract(issue_tokens)]
    IssueTokens,
}

//#[contract(asset)]
async fn issue_tokens(context: AssetTemplateContext, token_ids: Vec<TokenID>) -> Result<Vec<Token>> {
    let mut tokens = Vec::with_capacity(token_ids.len());
    let asset = &context.asset;
    let tokens_data = token_ids.into_iter().map(|id| NewToken::from((asset, id)));
    for data in tokens_data {
        if data.token_id.asset_id()? != asset.asset_id {
            bail!("Token ID {} does not match asset {}", data.token_id, asset.asset_id);
        }
        let token = context.create_token(data).await?;
        tokens.push(token);
    }
    Ok(tokens)
}

/// ***************** Token contracts *******************

//#[derive(Contracts)]
enum TokenContracts {
    //#[contract(transfer_token)]
    TransferToken,
}

//#[contract(token)]
//With token contract TokenTemplateContext is always passed as first argument
async fn transfer_token(context: TokenTemplateContext, user_pubkey: Pubkey) -> Result<Token> {
    let mut token = context.token.clone();
    if token.status == TokenStatus::Retired {
        bail!("Tried to transfer already used token");
    }
    token
        .additional_data_json
        .as_object_mut()
        .map(|obj| obj.insert("user_pubkey".into(), user_pubkey.into()))
        .ok_or(anyhow::anyhow!("Corrupt token: {}", token.id))?;
    let token = context.update_token(token).await?;
    Ok(token)
}

struct SingleUseTokenTemplate;
impl Template for SingleUseTokenTemplate {
    type AssetContracts = AssetContracts;
    type TokenContracts = TokenContracts;

    fn id() -> TemplateID {
        1.into()
    }
}

mod expanded_macros {
    use super::*;

    ////// impl #[contract(asset)] for issue_tokens()

    #[derive(Deserialize)]
    struct IssueTokensPayload {
        token_ids: Vec<TokenID>,
    }

    async fn issue_tokens_actix(
        params: web::Path<AssetCallParams>,
        data: web::Json<IssueTokensPayload>,
        context: TemplateContext,
    ) -> Result<Vec<Token>>
    {
        let asset = context.load_asset(params.asset_id(&context.template_id)?).await?;
        let context = AssetTemplateContext::new(context, asset);

        issue_tokens(context, data.into_inner().token_ids).await
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

    #[derive(Deserialize)]
    struct TransferTokenPayload {
        user_pubkey: Pubkey,
    }

    async fn transfer_token_actix(
        params: web::Path<TokenCallParams>,
        data: web::Json<TransferTokenPayload>,
        context: TemplateContext,
    ) -> Result<Token>
    {
        let asset_id = params.asset_id(&context.template_id)?;
        let asset = context.load_asset(asset_id).await?;
        let token_id = params.token_id(&context.template_id)?;
        let token = context.load_token(token_id).await?;
        let context = TokenTemplateContext::new(context, asset, token);

        transfer_token(context, data.into_inner().user_pubkey).await
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
