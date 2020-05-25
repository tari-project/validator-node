use super::*;
use crate::{
    db::models::{consensus::instructions::*, *},
    template::*,
    types::*,
    wallet::WalletStore,
};
use deadpool_postgres::Pool;
use multiaddr::Multiaddr;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AssetContextBuilder {
    pub template_id: TemplateID,
    pub asset: Option<AssetState>,
    pub wallets: Arc<Mutex<WalletStore>>,
    pub address: Multiaddr,
    pub params: Value,
    pub contract_name: String,
}

impl Default for AssetContextBuilder {
    fn default() -> Self {
        Self {
            template_id: 65536.into(),
            asset: None,
            wallets: WalletStoreBuilder::default().build().unwrap(),
            address: Multiaddr::empty(),
            params: json!({}),
            contract_name: "test_contract".into(),
        }
    }
}

impl AssetContextBuilder {
    pub async fn build<'a>(self, pool: Arc<Pool>) -> anyhow::Result<AssetInstructionContext> {
        let asset = match self.asset {
            Some(asset) => asset,
            None => {
                let asset_id = AssetID::test_from_template(self.template_id);
                let client = pool.get().await?;
                AssetStateBuilder {
                    asset_id,
                    ..Default::default()
                }
                .build(&client)
                .await?
            },
        };

        let context = TemplateContext {
            pool,
            template_id: asset.asset_id.template_id(),
            wallets: self.wallets,
            address: self.address,
        };
        let instruction = NewInstruction {
            asset_id: asset.asset_id.clone(),
            template_id: context.template_id.clone(),
            params: self.params,
            contract_name: self.contract_name,
            status: InstructionStatus::Scheduled,
            ..NewInstruction::default()
        };
        let instruction = context.create_instruction(instruction).await?;
        let context = context.instruction_context(instruction.id).await?;

        Ok(AssetInstructionContext::new(context, asset))
    }
}

pub struct TokenContextBuilder {
    pub template_id: TemplateID,
    pub token: Option<Token>,
    pub wallets: Arc<Mutex<WalletStore>>,
    pub address: Multiaddr,
    pub params: Value,
    pub contract_name: String,
}

impl Default for TokenContextBuilder {
    fn default() -> Self {
        Self {
            template_id: 65536.into(),
            token: None,
            wallets: WalletStoreBuilder::default().build().unwrap(),
            address: Multiaddr::empty(),
            params: json!({}),
            contract_name: "test_contract".into(),
        }
    }
}

impl TokenContextBuilder {
    pub async fn build<'a>(self, pool: Arc<Pool>) -> anyhow::Result<TokenInstructionContext> {
        let client = pool.get().await?;
        let token = match self.token {
            Some(token) => token,
            None => {
                let asset_id = AssetID::test_from_template(self.template_id);
                let token_id = TokenID::test_from_asset(&asset_id);
                TokenBuilder {
                    token_id,
                    ..Default::default()
                }
                .build(&client)
                .await?
            },
        };
        let asset = AssetState::load(token.asset_state_id, &client).await?;

        let context = TemplateContext {
            pool,
            template_id: asset.asset_id.template_id(),
            wallets: self.wallets,
            address: self.address,
        };
        let instruction = NewInstruction {
            asset_id: token.token_id.asset_id(),
            template_id: context.template_id.clone(),
            params: self.params,
            contract_name: self.contract_name,
            status: InstructionStatus::Scheduled,
            ..NewInstruction::default()
        };
        let instruction = context.create_instruction(instruction).await?;
        let context = context.instruction_context(instruction.id).await?;

        Ok(TokenInstructionContext::new(context, asset, token))
    }
}
