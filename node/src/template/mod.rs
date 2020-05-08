use crate::{
    db::models::{AssetState, DigitalAsset},
    types::{AssetID, TemplateID},
};
use serde_json::Value;
use std::str::FromStr;

#[async_trait::async_trait]
pub trait Template {
    type Error: std::error::Error;
    type AssetFunctions: FromStr<Err = Self::Error>;
    type TokenFunctions: FromStr<Err = Self::Error>;

    fn id() -> TemplateID;
    async fn create_asset(asset: DigitalAsset, state: AssetState) -> Result<AssetState, Self::Error>;
    async fn asset_call(function: Self::AssetFunctions, asset_id: AssetID, params: Value);
    async fn token_call(function: Self::TokenFunctions, token_id: TokenID, params: Value);
}
