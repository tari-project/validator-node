use super::{Contracts, Template, TemplateContext};
use crate::db::models::{AssetState, NewToken, Token, TokenStatus, UpdateToken};
use crate::types::{TokenID, Pubkey, TemplateID};
use anyhow::{bail, Result};
use super::actix::*;
use serde_json::json;
use serde::Deserialize;

//#[contract]
async fn issue_tokens(asset: AssetState, token_ids: Vec<TokenID>, context: TemplateContext) -> Result<Vec<Token>> {
    let tokens_data: Vec<_> = token_ids.into_iter()
        .map(|id| NewToken::from((&asset, id)));
    let mut tokens = Vec::with_capacity(token_ids.len());
    for data in tokens_data {
        if data.token_id.asset_id() != asset.asset_id {
            bail!("Token ID {} does not match asset {}", data.token_id, asset.asset_id);
        }
        let token = context.create_token(data).await?;
        tokens.push(token);
    }
    Ok(tokens)
}

//////  expanded impl #[contract]

#[derive(Deserialize)]
struct IssueTokensPayload { token_ids: Vec<TokenID> }

async fn issue_tokens_actix(params: web::Path<AssetCallParams>, data: web::Json<IssueTokensPayload>, context: TemplateContext) -> Result<Vec<Token>> {
    let asset = params.asset(&context);

    issue_tokens(asset, data.token_ids, context).await
}

/////// end of impl #[contract]


//#[contract]

async fn transfer_token(token: Token, user_pubkey: Pubkey, context: TemplateContext) -> Result<Token> {
    if token.status == TokenStatus::Retired {
        bail!("Tried to transfer already used token");
    }
    let data = UpdateToken {
        additional_data_json: Some(json!({user_pubkey: user_pubkey})),
        ..UpdateToken::default()
    };
    let token = context.update_token(token.id, data).await?;
    Ok(token)
}

//////  expanded impl #[contract]

#[derive(Deserialize)]
struct TransferTokenPayload { user_pubkey: Pubkey }

async fn transfer_token_actix(params: web::Path<TokenCallParams>, data: web::Json<TransferTokenPayload>, context: TemplateContext) -> Result<Token> {
    let token = params.token(&context);

    transfer_token(token, data.user_pubkey, context).await
}

/////// end of impl #[contract]

//#[derive(Contracts)]
enum AssetContracts {
    IssueTokens(issue_tokens),
}

////// impl #[derive(Contracts)]:

impl Contracts for AssetContracts {
    fn setup_actix_routes(scope: &mut web::ServiceConfig) {
        scope
        .service(
            web::resource("/issue_tokens")
                .route(web::post().to(issue_tokens_actix))
        )
    }
}


////// end of #[derive(Contracts)]

//#[derive(Contracts)]
enum TokenContracts {
    TransferToken(transfer_token),
}

////// impl #[derive(Contracts)]:

use actix_web::web;

impl Contracts for TokenContracts {
    fn setup_actix_routes(scope: &mut web::ServiceConfig) {
        scope
        .service(
            web::resource("/transfer_token")
                .route(web::post().to(transfer_token_actix))
        )
    }
}

////// end of #[derive(Contracts)]

struct SingleUseTokenTemplate;
impl Template for SingleUseTokenTemplate {
    type AssetContracts = AssetContracts;
    type TokenContracts = TokenContracts;
    fn id() -> TemplateID {
        1.into()
    }
}
