use crate::{
    db::models::{AssetState, DigitalAsset},
    types::{AssetID, TemplateID, TokenID},
};
use serde_json::Value;
use std::str::FromStr;

pub trait Functions: FromStr {}

#[async_trait::async_trait]
pub trait Template {
    type Error;
    type AssetFunction: Functions;
    type TokenFunction: Functions;

    fn id() -> TemplateID;
    async fn create_asset(asset: DigitalAsset, state: AssetState) -> Result<AssetState, Self::Error>;
    async fn asset_call(function: Self::AssetFunction, asset_id: AssetID, params: Value);
    async fn token_call(function: Self::TokenFunction, token_id: TokenID, params: Value);
}
