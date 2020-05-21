use super::*;
use crate::{db::models::*, template::*, types::*};
use deadpool_postgres::Client;
use serde_json::Value;

#[derive(Default)]
pub struct AssetContextBuilder {
    pub template_id: TemplateID,
    pub asset: Option<AssetState>,
    pub params: Value,
    pub contract_name: String,
}

impl AssetContextBuilder {
    pub async fn build<'a>(self, client: Client) -> anyhow::Result<AssetTemplateContext<'a>> {
        let asset = match self.asset {
            Some(asset) => asset,
            None => {
                let asset_id = AssetID::test_from_template(self.template_id);
                AssetStateBuilder {
                    asset_id,
                    ..Default::default()
                }
                .build(&client)
                .await?
            },
        };

        let mut context = TemplateContext {
            client,
            template_id: asset.asset_id.template_id(),
            contract_transaction: None,
            db_transaction: None,
        };
        let transaction = NewContractTransaction {
            asset_state_id: asset.id,
            template_id: context.template_id.clone(),
            params: self.params,
            contract_name: self.contract_name,
            ..NewContractTransaction::default()
        };
        context.create_transaction(transaction).await?;

        Ok(AssetTemplateContext::new(context, asset))
    }
}

#[derive(Default)]
pub struct TokenContextBuilder {
    pub template_id: TemplateID,
    pub token: Option<Token>,
    pub params: Value,
    pub contract_name: String,
}

impl TokenContextBuilder {
    pub async fn build<'a>(self, client: Client) -> anyhow::Result<TokenTemplateContext<'a>> {
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

        let mut context = TemplateContext {
            client,
            template_id: asset.asset_id.template_id(),
            contract_transaction: None,
            db_transaction: None,
        };
        let transaction = NewContractTransaction {
            asset_state_id: token.asset_state_id,
            template_id: context.template_id.clone(),
            params: self.params,
            contract_name: self.contract_name,
            ..NewContractTransaction::default()
        };
        context.create_transaction(transaction).await?;

        Ok(TokenTemplateContext::new(context, asset, token))
    }
}
