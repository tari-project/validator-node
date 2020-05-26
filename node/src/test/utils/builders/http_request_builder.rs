use crate::{
    types::{AssetID, TokenID},
    template::{TemplateRunner, Template},
};
use crate::test::utils::{actix_test_pool, build_test_config};
use actix_web::test::TestRequest;

#[allow(dead_code)]
pub struct HttpRequestBuilder<T: Template> {
    test_request: TestRequest,
    phantom: std::marker::PhantomData<T>,
    #[doc(hidden)]
    pub __non_exhaustive: (),
}

impl<T: Template + 'static> Default for HttpRequestBuilder<T> {
    fn default() -> Self {
        let pool = actix_test_pool();
        let config = build_test_config().unwrap();
        let runner = TemplateRunner::<T>::create(pool, config);
        let context = runner.start();
        let test_request = TestRequest::default().data(context).data(T::id());
        Self {
            test_request,
            phantom: std::marker::PhantomData,
            __non_exhaustive: (),
        }
    }
}

#[allow(dead_code)]
impl<T: Template> HttpRequestBuilder<T> {
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
