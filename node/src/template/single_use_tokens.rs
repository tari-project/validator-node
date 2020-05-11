#[contract(asset)]
async fn issue_tokens(asset: AssetState, amount: u64, price: u64, context: TemplateContext) -> Result<Vec<TokenID>, TemplateError> {
    let mut tokens = Vec::with_capacity(amount);
    for mut token in (0..amount).map(|_| NewToken::from(&asset)) {
        token.update_data(json!({price}))?;
        tokens.push(context.create_token(token).await?.token_id());
    };
    Ok(tokens)
}

#[contract(asset)]
async fn buy_token(asset: AssetState, timeout_ms: u64, user_wallet_key: WalletID) -> Result<TokenID, TemplateError> {
    Ok(TokenID)
}

#[derive(Contracts)]
enum AssetContracts {
    IssueTokens(issue_tokens),
    BuyToken(buy_token),
}

// derive would extend:
impl AssetContracts {
    fn actix_routes(scope: actix_web::web::Scope) -> actix_web::web::Scope {
        scope
        .service(
            web::resource("/issue_tokens")
                .route(web::post().to(asset_contract_wrapper(issue_tokens)))
        )
        .service(
            web::resource("/buy_token")
                .route(web::post().to(asset_contract_wrapper(buy_token)))
        )
    }
}

struct SingleUseTokenTemplate;
impl Template for SingleUseTokenTemplate {
    type Network = CommitteeNetwork;
    type AssetContracts = AssetContracts;
    type TokenContracts = ();
    fn id() -> TemplateID {
        1.into()
    }
}
