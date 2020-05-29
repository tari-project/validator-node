use crate::{
    db::models::{NewToken, Token, TokenStatus, UpdateToken},
    template::{actix_web_impl::*, *},
    types::{Pubkey, TemplateID, TokenID},
    validation_err,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tari_template_derive::Contracts;

#[derive(Serialize, Deserialize)]
struct TokenData {
    pub owner_pubkey: Pubkey,
    pub used: bool,
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

/// ***************** Asset contracts *******************

//#[derive(Contracts)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AssetContracts {
    //#[contract(issue_tokens)]
    IssueTokens(IssueTokensParams),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
            used: false,
        };
        let new_token = move |token_id| NewToken {
            token_id,
            asset_state_id: asset.id.clone(),
            initial_data_json: json!(data),
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

#[derive(Contracts, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[contracts(template = "SingleUseTokenTemplate", token)]
/// Token contracts for SingleUseTokenTemplate
pub enum TokenContracts {
    /// sell_token accepting `price` and `user_pubkey` as input params
    /// 1. creates subinstruction with `wallet_key`
    /// 2. waiting for `price` to appear in the `wallet_key`, or `timeout_secs`
    /// 3. reassings token to `user_pubkey`
    /// NOTICE: ontract methods should implemented on this enum,
    /// also *Params struct should be distict for every method
    /// and passed as 2nd parameter
    #[contract(method = "sell_token")]
    SellToken(SellTokenParams),
    /// sell_token_lock transitions token to Locked state
    /// for while sell_token did not complete
    #[contract(method = "sell_token_lock")]
    SellTokenLock(SellTokenLockParams),
    /// transfer_token is moving token to new owner
    #[contract(method = "transfer_token")]
    TransferToken(TransferTokenParams),
    /// redeem_token returns token back to asset owner
    /// also marking it as used
    #[contract(method = "redeem_token")]
    RedeemToken(RedeemTokenParams),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SellTokenParams {
    price: i64,
    timeout_secs: u64,
    user_pubkey: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SellTokenLockParams {
    wallet_key: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TransferTokenParams {
    user_pubkey: Pubkey,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RedeemTokenParams;

impl TokenContracts {
    /// Sell token for a `price` amount of tari to user with `user_pubkey`
    ///
    /// ### Input Parameters:
    /// - price - amount of tari
    /// - user_pubkey - new owner of a token
    /// - timeout_secs - timeout before Instruction is cancelled as expired
    ///
    /// # Caveats:
    /// - Instruction is creating subinstruction with a wallet key,
    /// - Client need to retrieve wallet key from subinstruction and transfer amount
    async fn sell_token(
        context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
        SellTokenParams {
            price,
            timeout_secs,
            user_pubkey,
        }: SellTokenParams,
    ) -> Result<Token, TemplateError>
    {
        if let Err(err) = Self::validate_token(context, TokenStatus::Available) {
            return validation_err!("Can't sell: {}", err);
        };
        let wallet_key = context.create_temp_wallet().await?;
        let subcontract: Self = SellTokenLockParams {
            wallet_key: wallet_key.clone(),
        }
        .into();
        let suninstruction = context
            .create_subinstruction("sell_token".into(), subcontract.clone())
            .await?;
        let message = subcontract.into_message(suninstruction);
        let _ = context.defer(message).await?;
        // TODO: should start timeout timer once subinstruction moves to Commit
        let timeout = std::time::Instant::now();
        let timeout_secs = std::time::Duration::from_secs(timeout_secs);
        // TODO: implement better strategies for waiting for temporal events like subscriptions
        while context.check_balance(&wallet_key).await? < price {
            tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
            if timeout.elapsed() > timeout_secs {
                // TODO: any failure in instrustion should also fail all subinstructions
                return validation_err!("Timeout expired for sell_token");
            }
        }
        let token_data = TokenData {
            owner_pubkey: user_pubkey,
            used: false,
        };
        let data = UpdateToken {
            status: Some(TokenStatus::Active),
            append_state_data_json: Some(json!(token_data)),
            ..Default::default()
        };
        context.update_token(data).await?;
        Ok(context.token.clone())
    }

    /// Subcontract for sell_token
    async fn sell_token_lock(
        context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
        _: SellTokenLockParams,
    ) -> Result<(), TemplateError>
    {
        if let Err(err) = Self::validate_token(context, TokenStatus::Available) {
            return validation_err!("Can't lock: {}", err);
        };
        let data = UpdateToken {
            status: Some(TokenStatus::Locked),
            ..Default::default()
        };
        context.update_token(data).await?;
        Ok(())
    }

    // With token contract TokenInstructionContext is always passed as first argument
    async fn transfer_token(
        context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
        TransferTokenParams { user_pubkey }: TransferTokenParams,
    ) -> Result<Token, TemplateError>
    {
        if let Err(err) = Self::validate_token(context, TokenStatus::Active) {
            return validation_err!("Can't transfer: {}", err);
        };
        let token_data = TokenData {
            owner_pubkey: user_pubkey,
            used: false,
        };
        let data = UpdateToken {
            append_state_data_json: Some(json!(token_data)),
            ..Default::default()
        };
        context.update_token(data).await?;
        Ok(context.token.clone())
    }

    // With token contract TokenInstructionContext is always passed as first argument
    async fn redeem_token(
        context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
        _: RedeemTokenParams,
    ) -> Result<Token, TemplateError>
    {
        if let Err(err) = Self::validate_token(context, TokenStatus::Active) {
            return validation_err!("Can't redeem: {}", err);
        };
        let token_data = TokenData {
            owner_pubkey: context.asset.asset_issuer_pub_key.clone(),
            used: true,
        };
        let data = UpdateToken {
            append_state_data_json: Some(json!(token_data)),
            ..Default::default()
        };
        context.update_token(data).await?;
        Ok(context.token.clone())
    }

    fn validate_token(
        context: &mut TokenInstructionContext<SingleUseTokenTemplate>,
        status: TokenStatus,
    ) -> Result<(), String>
    {
        if context.token.status != status {
            return Err(format!(
                "expected token status {}, got {}",
                status, context.token.status
            ));
        }
        match serde_json::from_value::<TokenData>(context.token.additional_data_json.clone()) {
            Ok(data) => {
                if data.used {
                    return Err("already used token".into());
                }
            },
            _ => {},
        };
        Ok(())
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

        fn params(&self) -> Self::Params {
            self.params.clone()
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
        let data: AssetContracts = data.into_inner().into();
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
        let message = data.clone().into_message(instruction.clone());
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
        db::models::{asset_states::*, consensus::instructions::*, wallet::*},
        test::utils::{actix::TestAPIServer, builders::*, test_db_client, Test},
        types::AssetID,
    };
    use deadpool_postgres::Client;
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
        let asset_builder = AssetStateBuilder {
            asset_id: asset_id.clone(),
            ..Default::default()
        };
        asset_builder.build(&client).await.unwrap();

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

    async fn test_token(client: &Client) -> TokenID {
        let tpl = SingleUseTokenTemplate::id();
        let asset_id: AssetID = Test::from_template(tpl);
        let token_id: TokenID = Test::from_asset(&asset_id);
        let token_builder = TokenBuilder {
            token_id: token_id.clone(),
            ..Default::default()
        };
        token_builder.build(&client).await.unwrap();
        token_id
    }

    #[actix_rt::test]
    async fn instruction_params() {
        let srv = TestAPIServer::<SingleUseTokenTemplate>::new();
        let (client, _lock) = test_db_client().await;
        let token_id = test_token(&client).await;
        let user_pubkey = Test::<Pubkey>::new();
        let params = SellTokenParams {
            user_pubkey,
            timeout_secs: 1,
            price: 1,
        };
        let mut resp = srv
            .token_call(&token_id, "sell_token")
            .send_json(&params)
            .await
            .unwrap();

        let instruction: Instruction = resp.json().await.unwrap();
        let params2: TokenContracts = serde_json::from_value(instruction.params).unwrap();
        assert_eq!(params2, params.into());
    }

    #[actix_rt::test]
    async fn sell_token_full_stack() {
        let srv = TestAPIServer::<SingleUseTokenTemplate>::new();
        let (client, _lock) = test_db_client().await;
        let token_id = test_token(&client).await;
        let user_pubkey = Test::<Pubkey>::new();
        let mut resp = srv
            .token_call(&token_id, "sell_token")
            .send_json(&SellTokenParams {
                user_pubkey,
                timeout_secs: 10,
                price: 1,
            })
            .await
            .unwrap();

        assert!(resp.status().is_success());
        let instruction: Instruction = resp.json().await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Scheduled);

        let id = instruction.id;
        let wallet: Option<Wallet> = None;
        // TODO: need better solution for async Actor tests, some Test wrapper for actor
        for _ in 0u8..100 {
            tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
            let instruction = Instruction::load(id, &client).await.unwrap();
            assert_ne!(
                instruction.status,
                InstructionStatus::Invalid,
                "Instruction: {:?}",
                instruction
            );
            if instruction.status == InstructionStatus::Processing && wallet.is_none() {
                let subinstructions = instruction.load_subinstructions(&client).await.unwrap();
                if subinstructions.len() == 0 {
                    continue;
                }
                assert_eq!(subinstructions.len(), 1);
                let sub = subinstructions.first().unwrap();
                let params: TokenContracts = serde_json::from_value(sub.params.clone()).unwrap();
                if let TokenContracts::SellTokenLock(SellTokenLockParams { wallet_key }) = &params {
                    let wallet = Some(Wallet::select_by_key(wallet_key, &client).await.unwrap());
                    // top up money in wallet
                    wallet.as_ref().unwrap().set_balance(1, &client).await.unwrap();
                } else {
                    panic!("Incorrect params in subcontract {:?}", params)
                }
            } else if instruction.status == InstructionStatus::Pending {
                return;
            }
        }
        let instruction = Instruction::load(id, &client).await.unwrap();
        panic!(
            "Waiting for Actor to process Instruction longer than 10s {:?}",
            instruction
        );
    }

    async fn update_token(token_id: &TokenID, update: UpdateToken, client: &Client) {
        let token = Token::find_by_token_id(token_id, &client).await.unwrap().unwrap();
        let instruction = consensus::InstructionBuilder {
            token_id: Some(token_id.clone()),
            status: InstructionStatus::Commit,
            ..Default::default()
        }
        .build(&client)
        .await
        .unwrap();
        let _ = token.update(update, &instruction, &client).await.unwrap();
    }

    #[actix_rt::test]
    async fn sell_token_negative() {
        let srv = TestAPIServer::<SingleUseTokenTemplate>::new();
        let (client, _lock) = test_db_client().await;
        let token_id = test_token(&client).await;
        update_token(
            &token_id,
            UpdateToken {
                status: Some(TokenStatus::Active),
                ..Default::default()
            },
            &client,
        )
        .await;
        let user_pubkey = Test::<Pubkey>::new();
        let mut resp = srv
            .token_call(&token_id, "sell_token")
            .send_json(&SellTokenParams {
                user_pubkey,
                timeout_secs: 1,
                price: 1,
            })
            .await
            .unwrap();
        let instruction: Instruction = resp.json().await.unwrap();
        let id = instruction.id;
        for _ in 0u8..10 {
            tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
            let instruction = Instruction::load(id, &client).await.unwrap();
            if instruction.status != InstructionStatus::Scheduled {
                assert_eq!(instruction.status, InstructionStatus::Invalid);
                return;
            }
        }
        let instruction = Instruction::load(id, &client).await.unwrap();
        panic!(
            "Waiting for Actor to process Instruction longer than 1s {:?}",
            instruction
        );
    }

    #[actix_rt::test]
    async fn transfer_token() {
        let srv = TestAPIServer::<SingleUseTokenTemplate>::new();
        let (client, _lock) = test_db_client().await;
        let token_id = test_token(&client).await;
        update_token(
            &token_id,
            UpdateToken {
                status: Some(TokenStatus::Active),
                ..Default::default()
            },
            &client,
        )
        .await;
        let params = TransferTokenParams {
            user_pubkey: Test::<Pubkey>::new(),
        };
        let mut resp = srv
            .token_call(&token_id, "transfer_token")
            .send_json(&params)
            .await
            .unwrap();

        assert!(resp.status().is_success());
        let instruction: Instruction = resp.json().await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Scheduled);
        let _: TokenContracts = serde_json::from_value(instruction.params).unwrap();

        let id = instruction.id;
        // TODO: need better solution for async Actor tests, some Test wrapper for actor
        for _ in 0u8..10 {
            tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
            let instruction = Instruction::load(id, &client).await.unwrap();
            assert_ne!(
                instruction.status,
                InstructionStatus::Invalid,
                "Instruction: {:?}",
                instruction
            );
            if instruction.status == InstructionStatus::Pending {
                let token = Token::find_by_token_id(&token_id, &client).await.unwrap().unwrap();
                let data: TokenData = serde_json::from_value(token.additional_data_json).unwrap();
                assert_eq!(data.owner_pubkey, params.user_pubkey);
                return;
            }
        }
        let instruction = Instruction::load(id, &client).await.unwrap();
        panic!(
            "Waiting for Actor to process Instruction longer than 1s {:?}",
            instruction
        );
    }

    #[actix_rt::test]
    async fn redeem_token() {
        let srv = TestAPIServer::<SingleUseTokenTemplate>::new();
        let (client, _lock) = test_db_client().await;
        let token_id = test_token(&client).await;
        let update = UpdateToken {
            status: Some(TokenStatus::Active),
            append_state_data_json: Some(json!(TokenData {
                owner_pubkey: Test::<Pubkey>::new(),
                used: false
            })),
        };
        update_token(&token_id, update, &client).await;
        let mut resp = srv
            .token_call(&token_id, "redeem_token")
            .send_json(&RedeemTokenParams)
            .await
            .unwrap();

        assert!(resp.status().is_success());
        let instruction: Instruction = resp.json().await.unwrap();
        assert_eq!(instruction.status, InstructionStatus::Scheduled);
        let _: TokenContracts = serde_json::from_value(instruction.params).unwrap();

        let id = instruction.id;
        // TODO: need better solution for async Actor tests, some Test wrapper for actor
        for _ in 0u8..10 {
            tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
            let instruction = Instruction::load(id, &client).await.unwrap();
            assert_ne!(
                instruction.status,
                InstructionStatus::Invalid,
                "Instruction: {:?}",
                instruction
            );
            if instruction.status == InstructionStatus::Pending {
                let token = Token::find_by_token_id(&token_id, &client).await.unwrap().unwrap();
                let asset = AssetState::find_by_asset_id(&instruction.asset_id, &client)
                    .await
                    .unwrap()
                    .unwrap();
                let data: TokenData = serde_json::from_value(token.additional_data_json).unwrap();
                assert_eq!(data.owner_pubkey, asset.asset_issuer_pub_key);
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
