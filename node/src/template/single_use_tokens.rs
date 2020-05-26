use super::{
    actix_web_impl::*,
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

const LOG_TARGET: &'static str = "validator_node::template::single_use_tokens";

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
async fn issue_tokens(
    context: &AssetInstructionContext<SingleUseTokenTemplate>,
    token_ids: Vec<TokenID>,
) -> Result<Vec<Token>, TemplateError>
{
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

#[tari_template_macro::contract(token, template = "SingleUseTokenTemplate", internal)]
/// Initiate sell token instruction
///
/// ### Input Parameters:
/// - price - amount of tari
/// - user_pubkey - new owner of a token
///
/// # Returns:
/// - Temporary wallet pubkey, where user need to transfer price amount of tari's
async fn sell_token(
    context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
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

#[tari_template_macro::contract(token, template = "SingleUseTokenTemplate", internal)]
// With token contract TokenInstructionContext is always passed as first argument
async fn transfer_token(
    context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
    user_pubkey: Pubkey,
) -> Result<Token, TemplateError>
{
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
#[derive(Clone)]
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
        api::errors::ApiError,
        db::models::consensus::instructions::*,
        template::{context::*, runner::*},
        types::AssetID,
    };
    use actix::prelude::*;
    use actix_web::web;
    use futures::future::TryFutureExt;
    use log::info;
    use serde::{Deserialize, Serialize};

    ////// impl #[contract(asset)] for issue_tokens()

    type ThisActor = TemplateRunner<SingleUseTokenTemplate>;

    /// Input parameters via RPC
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Params {
        token_ids: Vec<TokenID>,
    }

    /// Actor's message is input parameters combined with Instruction
    #[derive(Message)]
    #[rtype(result = "Result<(),TemplateError>")]
    pub struct MessageIssueTokens {
        asset_id: AssetID,
        params: Params,
        instruction: Instruction,
    }

    /// Actor is accepting Scheduled / Processing Instruction and tries to perform activity
    impl Handler<MessageIssueTokens> for ThisActor {
        type Result = ResponseActFuture<Self, Result<(), TemplateError>>;

        fn handle(&mut self, msg: MessageIssueTokens, _ctx: &mut Context<Self>) -> Self::Result {
            let context = self.context();
            let instruction = msg.instruction.clone();
            let asset_context_fut =
                AssetInstructionContext::init(self.context(), msg.instruction.clone(), msg.asset_id.clone());
            log::trace!(
                target: LOG_TARGET,
                "template={}, instruction={}, Actor received issue_tokens instruction",
                Self::template_id(),
                msg.instruction.id
            );

            let fut = actix::fut::wrap_future::<_, Self>(
                async move {
                    let mut context = asset_context_fut.await?;
                    context.transition(ContextEvent::StartProcessing).await?;
                    // TODO: instruction needs to be able to run in an encapsulated way and return
                    // NewTokenStateAppendOnly and NewAssetStateAppendOnly vecs       as the
                    // consensus workers need to be able to run an instruction set and confirm the
                    // resulting state matches run contract
                    let result = issue_tokens(&context, msg.params.token_ids).await?;
                    // update instruction after contract executed
                    let result =
                        serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?;
                    context.transition(ContextEvent::ProcessingResult { result }).await?;
                    Ok(())
                }
                .or_else(move |err: TemplateError| context.instruction_failed(instruction, err)),
            );
            Box::pin(fut)
        }
    }

    // Wrapper will convert from actix types into Rust,
    // create instructions writing RPC params,
    // returning instruction
    // Instruction is created here to return it immediately to the client
    // so client can keep polling for result.
    async fn issue_tokens_actix(
        params: web::Path<AssetCallParams>,
        data: web::Json<Params>,
        context: web::Data<TemplateContext<SingleUseTokenTemplate>>,
    ) -> Result<web::Json<Instruction>, ApiError>
    {
        // extract and transform parameters
        let asset_id = params.asset_id(context.template_id())?;
        let data = data.into_inner();
        // start instruction
        let instruction = NewInstruction {
            asset_id: asset_id.clone(),
            template_id: context.template_id(),
            // TODO: proper handling of unlikely error
            params: serde_json::to_value(&data).unwrap(),
            contract_name: "issue_tokens".to_string(),
            status: InstructionStatus::Scheduled,
            ..NewInstruction::default()
        };
        let instruction = context.create_instruction(instruction).await?;
        let message = MessageIssueTokens {
            asset_id,
            instruction: instruction.clone(),
            params: data.clone(),
        };
        context
            .addr()
            .try_send(message)
            .map_err(|err| TemplateError::ActorSend {
                source: err.into(),
                // TODO: proper handling of unlikely error
                params: serde_json::to_string(&data).unwrap(),
                name: "issue_tokens".into(),
            })?;
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

    // impl Handler<sell_token_actix::Msg> for ThisActor {
    //     type Result = ResponseActFuture<Self, Result<(), TemplateError>>;

    //     fn handle(&mut self, msg: sell_token_actix::Msg, _ctx: &mut Context<Self>) -> Self::Result {
    //         let context = self.context();
    //         let instruction = msg.instruction.clone();
    //         let token_context_fut =
    //             TokenInstructionContext::init(self.context(), msg.instruction.clone(), msg.token_id.clone());
    //         log::trace!(target: LOG_TARGET, "template={}, instruction={}, Actor received issue_tokens instruction",
    // Self::template_id(), msg.instruction.id);

    //         let fut = actix::fut::wrap_future::<_, Self>(
    //             async move {
    //                 let mut context = token_context_fut.await?;
    //                 context.transition(ContextEvent::StartProcessing).await?;
    //                 // TODO: instruction needs to be able to run in an encapsulated way and return
    //                 // NewTokenStateAppendOnly and NewAssetStateAppendOnly vecs       as the
    //                 // consensus workers need to be able to run an instruction set and confirm the
    //                 // resulting state matches run contract
    //                 let result = sell_token(&mut context, msg.params.price, msg.params.user_pubkey).await?;
    //                 // update instruction after contract executed
    //                 let result =
    //                     serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?;
    //                 context.transition(ContextEvent::ProcessingResult { result }).await?;
    //                 Ok(())
    //             }
    //             .or_else(move |err: TemplateError| {
    //                 context.instruction_failed(instruction, err)
    //             }),
    //         );
    //         Box::pin(fut)
    //     }
    // }

    ////// impl #[derive(Contracts)] for TokenContracts

    impl Contracts for TokenContracts {
        fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig) {
            info!("template={}, installing token API transfer_token", tpl);
            scope.service(web::resource("/transfer_token").route(web::post().to(transfer_token_actix::web_handler)));
            scope.service(web::resource("/sell_token").route(web::post().to(sell_token_actix::web_handler)));
        }
    }

    ////// end of #[derive(Contracts)]
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::models::consensus::instructions::*,
        test::utils::{actix::TestAPIServer, builders::*, test_db_client, Test},
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
        .build()
        .await
        .unwrap();
        let asset_id = context.asset.asset_id.clone();
        let token_ids: Vec<_> = (0..10).map(|_| Test::<TokenID>::from_asset(&asset_id)).collect();

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
        .build()
        .await
        .unwrap();
        let asset_id = AssetID::default();
        let token_ids: Vec<_> = (0..10).map(|_| Test::<TokenID>::from_asset(&asset_id)).collect();
        assert!(issue_tokens(&context, token_ids).await.is_err());
    }

    #[actix_rt::test]
    async fn issue_tokens_full_stack() {
        let srv = TestAPIServer::<SingleUseTokenTemplate>::new();
        let (client, _lock) = test_db_client().await;

        let tpl = SingleUseTokenTemplate::id();
        let asset_id = Test::<AssetID>::from_template(tpl);
        let token_ids: Vec<_> = (0..10).map(|_| Test::<TokenID>::from_asset(&asset_id)).collect();
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
        let instruction: Instruction = resp.json().await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Scheduled);
        assert!(srv.context().addr().connected());
        let id = instruction.id;
        // TODO: need better solution for async Actor tests, some Test wrapper for actor
        for _ in 0..10 {
            tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
            let instruction = Instruction::load(id, &client).await.unwrap();
            assert_ne!(instruction.status, InstructionStatus::Invalid);
            if instruction.status == InstructionStatus::Pending {
                return;
            }
        }
        let instruction = Instruction::load(id, &client).await.unwrap();
        panic!(
            "Waiting for Actor to process Instruction longer than 1s {:?}",
            instruction
        );
    }
}
