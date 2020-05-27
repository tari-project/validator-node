use crate::{
    db::models::{NewToken, Token, TokenStatus, UpdateToken},
    template::{actix_web_impl::*, *},
    types::{Pubkey, TemplateID, TokenID},
    validation_err,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tari_template_derive::Contracts;

const LOG_TARGET: &'static str = "validator_node::template::single_use_tokens";

#[derive(Serialize, Deserialize)]
struct TokenData {
    pub owner_pubkey: Pubkey,
}

/// ***************** Asset contracts *******************

//#[derive(Contracts)]
#[derive(Serialize, Deserialize, Clone)]
pub enum AssetContracts {
    //#[contract(issue_tokens)]
    IssueTokens(IssueTokensParams),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IssueTokensParams {
    token_ids: Vec<TokenID>,
}

// TODO: return type is converted to ContextEvent with Value parameter,
// constrain return type
// TODO: probably we can automate boilerplate via higher level traits
// instead of macros? Or would that require GAT?
//#[contract(asset)]
impl AssetContracts {
    pub async fn issue_tokens(
        context: &mut AssetInstructionContext<SingleUseTokenTemplate>,
        IssueTokensParams { token_ids }: IssueTokensParams,
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
}

/// ***************** Token contracts *******************

#[derive(Contracts, Serialize, Deserialize, Clone)]
#[contracts(template = "SingleUseTokenTemplate", token)]
pub enum TokenContracts {
    #[contract(method = "sell_token")]
    SellToken(SellTokenParams),
    #[contract(method = "sell_token_lock")]
    SellTokenLock(SellTokenLockParams),
    #[contract(method = "transfer_token")]
    TransferToken(TransferTokenParams),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SellTokenParams {
    price: u64,
    user_pubkey: Pubkey,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SellTokenLockParams {
    wallet_key: Pubkey,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TransferTokenParams {
    user_pubkey: Pubkey,
}

impl TokenContracts {
    /// Sell token for a `price` amount of tari to user with `user_pubkey`
    ///
    /// ### Input Parameters:
    /// - price - amount of tari
    /// - user_pubkey - new owner of a token
    ///
    /// # Caveats:
    /// - Instruction is creating subinstruction with a wallet key,
    /// - Client need to retrieve wallet key from subinstruction and transfer amount
    async fn sell_token(
        context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
        SellTokenParams { price, user_pubkey }: SellTokenParams,
    ) -> Result<Pubkey, TemplateError>
    {
        let token = context.token.clone();
        if token.status == TokenStatus::Retired {
            return validation_err!("Tried to transfer already used token");
        }
        let wallet = context.create_temp_wallet().await?;
        // context.subinstruction().await?;
        Ok(wallet)
    }

    /// Subcontract for sell_token
    async fn sell_token_lock(
        context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
        SellTokenLockParams { wallet_key }: SellTokenLockParams,
    ) -> Result<(), TemplateError>
    {
        Ok(())
    }

    // With token contract TokenInstructionContext is always passed as first argument
    async fn transfer_token(
        context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
        TransferTokenParams { user_pubkey }: TransferTokenParams,
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

pub mod asset_contracts_actix {
    use super::*;
    use crate::{
        api::errors::ApiError,
        db::models::consensus::instructions::*,
        template::{actors::*, context::*},
        types::AssetID,
    };
    use actix::prelude::*;
    use actix_web::web;

    ////// impl #[derive(Contracts)] for AssetContracts

    impl Contracts for AssetContracts {
        fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig) {
            log::info!("template={}, installing assets API issue_tokens", tpl);
            scope.service(web::resource("/issue_tokens").route(web::post().to(asset_contracts_actix::web_handler)));
        }
    }

    impl From<IssueTokensParams> for AssetContracts {
        fn from(params: IssueTokensParams) -> Self {
            Self::IssueTokens(params)
        }
    }

    impl AssetContracts {
        pub async fn call(
            self,
            mut context: AssetInstructionContext<SingleUseTokenTemplate>,
        ) -> AssetCallResult<SingleUseTokenTemplate>
        {
            let result = match self {
                Self::IssueTokens(params) => Self::issue_tokens(&mut context, params).await?,
            };
            let value = serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?;
            Ok((value, context))
        }

        pub fn into_message(self, instruction: Instruction) -> Msg {
            Msg {
                params: self,
                asset_id: instruction.asset_id.clone(),
                instruction,
            }
        }
    }

    /// Actor's message is input parameters combined with Instruction
    #[derive(Message, Clone)]
    #[rtype(result = "Result<(),TemplateError>")]
    pub struct Msg {
        asset_id: AssetID,
        params: AssetContracts,
        instruction: Instruction,
    }

    impl ContractCallMsg for Msg {
        type Context = AssetInstructionContext<Self::Template>;
        type Params = AssetContracts;
        type Template = SingleUseTokenTemplate;

        type CallResult = impl Future<Output = AssetCallResult<Self::Template>>;
        type ContextFuture = impl Future<Output = Result<Self::Context, TemplateError>>;

        fn instruction(&self) -> Instruction {
            self.instruction.clone()
        }

        fn call(self, context: AssetInstructionContext<Self::Template>) -> Self::CallResult {
            self.params.clone().call(context)
        }

        fn init_context(self, ctx: TemplateContext<Self::Template>) -> Self::ContextFuture {
            AssetInstructionContext::init(ctx, self.instruction, self.asset_id)
        }
    }

    ////// end of #[derive(Contracts)]

    ////// impl #[contract(asset)] for issue_tokens()

    // Wrapper will convert from actix types into Rust,
    // create instructions writing RPC params,
    // returning instruction
    // Instruction is created here to return it immediately to the client
    // so client can keep polling for result.
    pub async fn web_handler(
        params: web::Path<AssetCallParams>,
        data: web::Json<IssueTokensParams>,
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
        let contract: AssetContracts = data.clone().into();
        let message = contract.into_message(instruction.clone());
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
        let contract = AssetContracts::IssueTokens(IssueTokensParams {
            token_ids: token_ids.clone(),
        });

        let (tokens, _) = contract.call(context).await.unwrap();
        let tokens: Vec<Token> = serde_json::from_value(tokens).unwrap();
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
        let contract = AssetContracts::IssueTokens(IssueTokensParams { token_ids });
        assert!(contract.call(context).await.is_err());
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
