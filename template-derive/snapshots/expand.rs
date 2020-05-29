pub mod sell_token_actix {
    use super::*;
    use crate::{
        api::errors::{ApiError, ApplicationError},
        db::models::consensus::instructions::*,
        template::{actors::*, context::*},
    };
    use actix_web::web;
    impl From<SellTokenParams> for TokenContracts {
        fn from(params: SellTokenParams) -> Self {
            TokenContracts::SellToken(params)
        }
    }
    pub async fn web_handler(
        params: web::Path<TokenCallParams>,
        data: web::Json<SellTokenParams>,
        context: web::Data<TemplateContext<SingleUseTokenTemplate>>,
    ) -> Result<web::Json<Instruction>, ApiError>
    {
        let asset_id = params.asset_id(context.template_id())?;
        let token_id = params.token_id(context.template_id())?;
        let data = data.into_inner();
        let instruction = NewInstruction {
            asset_id: asset_id.clone(),
            token_id: Some(token_id.clone()),
            template_id: context.template_id(),
            params: serde_json::to_value(&data)
                .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
            contract_name: "sell_token".into(),
            status: InstructionStatus::Scheduled,
            ..NewInstruction::default()
        };
        let instruction = context.create_instruction(instruction).await?;
        let contract: TokenContracts = data.clone().into();
        let message = contract.into_message(instruction.clone());
        context
            .addr()
            .try_send(message)
            .map_err(|err| TemplateError::ActorSend {
                source: err.into(),
                params: serde_json::to_string(&data).unwrap(),
                name: "sell_token".into(),
            })?;
        return Ok(web::Json(instruction));
    }
}
pub mod sell_token_lock_actix {
    use super::*;
    use crate::{
        api::errors::{ApiError, ApplicationError},
        db::models::consensus::instructions::*,
        template::{actors::*, context::*},
    };
    use actix_web::web;
    impl From<SellTokenLockParams> for TokenContracts {
        fn from(params: SellTokenLockParams) -> Self {
            TokenContracts::SellTokenLock(params)
        }
    }
    pub async fn web_handler(
        params: web::Path<TokenCallParams>,
        data: web::Json<SellTokenLockParams>,
        context: web::Data<TemplateContext<SingleUseTokenTemplate>>,
    ) -> Result<web::Json<Instruction>, ApiError>
    {
        let asset_id = params.asset_id(context.template_id())?;
        let token_id = params.token_id(context.template_id())?;
        let data = data.into_inner();
        let instruction = NewInstruction {
            asset_id: asset_id.clone(),
            token_id: Some(token_id.clone()),
            template_id: context.template_id(),
            params: serde_json::to_value(&data)
                .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
            contract_name: "sell_token_lock".into(),
            status: InstructionStatus::Scheduled,
            ..NewInstruction::default()
        };
        let instruction = context.create_instruction(instruction).await?;
        let contract: TokenContracts = data.clone().into();
        let message = contract.into_message(instruction.clone());
        context
            .addr()
            .try_send(message)
            .map_err(|err| TemplateError::ActorSend {
                source: err.into(),
                params: serde_json::to_string(&data).unwrap(),
                name: "sell_token_lock".into(),
            })?;
        return Ok(web::Json(instruction));
    }
}
pub mod transfer_token_actix {
    use super::*;
    use crate::{
        api::errors::{ApiError, ApplicationError},
        db::models::consensus::instructions::*,
        template::{actors::*, context::*},
    };
    use actix_web::web;
    impl From<TransferTokenParams> for TokenContracts {
        fn from(params: TransferTokenParams) -> Self {
            TokenContracts::TransferToken(params)
        }
    }
    pub async fn web_handler(
        params: web::Path<TokenCallParams>,
        data: web::Json<TransferTokenParams>,
        context: web::Data<TemplateContext<SingleUseTokenTemplate>>,
    ) -> Result<web::Json<Instruction>, ApiError>
    {
        let asset_id = params.asset_id(context.template_id())?;
        let token_id = params.token_id(context.template_id())?;
        let data = data.into_inner();
        let instruction = NewInstruction {
            asset_id: asset_id.clone(),
            token_id: Some(token_id.clone()),
            template_id: context.template_id(),
            params: serde_json::to_value(&data)
                .map_err(|err| ApplicationError::bad_request(format!("Contract params error: {}", err).as_str()))?,
            contract_name: "transfer_token".into(),
            status: InstructionStatus::Scheduled,
            ..NewInstruction::default()
        };
        let instruction = context.create_instruction(instruction).await?;
        let contract: TokenContracts = data.clone().into();
        let message = contract.into_message(instruction.clone());
        context
            .addr()
            .try_send(message)
            .map_err(|err| TemplateError::ActorSend {
                source: err.into(),
                params: serde_json::to_string(&data).unwrap(),
                name: "transfer_token".into(),
            })?;
        return Ok(web::Json(instruction));
    }
}
pub mod tokencontracts_impl {
    use super::*;
    use crate::{
        api::errors::ApiError,
        db::models::consensus::instructions::*,
        template::{actors::*, context::*},
        types::{TemplateID, TokenID},
    };
    use actix::prelude::*;
    use actix_web::web;
    impl Contracts for TokenContracts {
        fn setup_actix_routes(tpl: TemplateID, scope: &mut web::ServiceConfig) {
            log::info!("template={}, installing {} APIs", "token", tpl);
            scope.service(web::resource("/sell_token").route(web::post().to(sell_token_actix::web_handler)));
            scope.service(web::resource("/sell_token_lock").route(web::post().to(sell_token_lock_actix::web_handler)));
            scope.service(web::resource("/transfer_token").route(web::post().to(transfer_token_actix::web_handler)));
        }
    }
    impl TokenContracts {
        pub async fn call(
            self,
            mut context: TokenInstructionContext<SingleUseTokenTemplate>,
        ) -> TokenCallResult<SingleUseTokenTemplate>
        {
            let value = match self {
                TokenContracts::SellToken(params) => {
                    let result = Self::sell_token(&mut context, params).await?;
                    serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?
                },
                TokenContracts::SellTokenLock(params) => {
                    let result = Self::sell_token_lock(&mut context, params).await?;
                    serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?
                },
                TokenContracts::TransferToken(params) => {
                    let result = Self::transfer_token(&mut context, params).await?;
                    serde_json::to_value(result).map_err(|err| TemplateError::Processing(err.to_string()))?
                },
            };
            Ok((value, context))
        }

        pub fn into_message(self, instruction: Instruction) -> Msg {
            Msg {
                params: self,
                id: instruction.token_id.clone().unwrap(),
                instruction,
            }
        }
    }
    #[doc = r" Actor's message is input parameters combined with Instruction"]
    #[derive(Message, Clone)]
    #[rtype(result = "Result<(),TemplateError>")]
    pub struct Msg {
        id: TokenID,
        params: TokenContracts,
        instruction: Instruction,
    }
    impl ContractCallMsg for Msg {
        type Context = TokenInstructionContext<Self::Template>;
        type Params = TokenContracts;
        type Template = SingleUseTokenTemplate;

        type CallResult = impl Future<Output = TokenCallResult<SingleUseTokenTemplate>>;
        type ContextFuture = impl Future<Output = Result<Self::Context, TemplateError>>;

        fn instruction(&self) -> Instruction {
            self.instruction.clone()
        }

        fn call(self, context: Self::Context) -> Self::CallResult {
            self.params.clone().call(context)
        }

        fn init_context(self, ctx: TemplateContext<Self::Template>) -> Self::ContextFuture {
            TokenInstructionContext::init(ctx, self.instruction, self.id)
        }
    }
}
