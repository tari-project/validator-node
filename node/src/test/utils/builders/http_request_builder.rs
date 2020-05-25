use super::WalletStoreBuilder;
use crate::{
    test::utils::{actix_test_pool, build_test_config},
    types::{AssetID, TokenID},
};
use actix_web::test::TestRequest;

#[allow(dead_code)]
pub struct HttpRequestBuilder {
    test_request: TestRequest,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl Default for HttpRequestBuilder {
    fn default() -> Self {
        let pool = actix_test_pool();
        let wallets = WalletStoreBuilder::default().build().unwrap();
        let config = build_test_config().unwrap();
        let test_request = TestRequest::default()
            .app_data(pool)
            .app_data(config)
            .app_data(wallets);
        Self {
            test_request,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl HttpRequestBuilder {
    pub fn asset_call(mut self, id: &AssetID, contract: &str) -> Self {
        let uri = format!(
            "/asset_call/{}/{:04X}/{}/{}/{}",
            id.template_id(),
            id.features(),
            id.raid_id().to_base58(),
            id.hash(),
            contract
        );
        self.test_request = self.test_request.uri(uri.as_str()).data(id.template_id());
        self
    }

    pub fn token_call(mut self, id: &TokenID, contract: &str) -> Self {
        let asset_id = id.asset_id();
        let uri = format!(
            "/token_call/{}/{:04X}/{}/{}/{}/{}",
            asset_id.template_id(),
            asset_id.features(),
            asset_id.raid_id().to_base58(),
            asset_id.hash(),
            id.uid().to_simple(),
            contract
        );
        self.test_request = self.test_request.uri(uri.as_str()).data(asset_id.template_id());
        self
    }

    pub fn build(self) -> TestRequest {
        self.test_request
    }
}
