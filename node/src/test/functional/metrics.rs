use crate::{
    metrics::GetMetrics,
    template::{single_use_tokens::SingleUseTokenTemplate, Template},
    test::utils::{actix::TestAPIServer, builders::AssetStateBuilder, test_db_client, Test},
    types::{AssetID, TokenID},
};
use serde_json::json;
use std::time::Duration;
use tokio::time::delay_for;

#[actix_rt::test]
async fn fullstack_metrics() {
    let srv = TestAPIServer::<SingleUseTokenTemplate>::new();
    let (client, _lock) = test_db_client().await;

    let tpl = SingleUseTokenTemplate::id();
    let asset_id = Test::<AssetID>::from_template(tpl);
    let token_id = Test::<TokenID>::from_asset(&asset_id);
    let asset_builder = AssetStateBuilder {
        asset_id: asset_id.clone(),
        ..Default::default()
    };
    asset_builder.build(&client).await.unwrap();

    let resp = srv
        .asset_call(&asset_id, "issue_tokens")
        .send_json(&json!({ "token_ids": vec![token_id.clone()] }))
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let metrics = srv.metrics.send(GetMetrics).await.unwrap();
    // TODO: add metrics for API calls
    // assert_eq!(metrics.total_calls["issue_tokens"], 1);
    assert_eq!(metrics.total_unique_instructions, 1);
    assert_eq!(metrics.instructions_scheduled_spark.into_iter().sum::<u64>(), 1);
    delay_for(Duration::from_secs(1)).await;
    let metrics = srv.metrics.send(GetMetrics).await.unwrap();
    assert_eq!(metrics.instructions_pending_spark.into_iter().sum::<u64>(), 1);
    assert_eq!(metrics.instructions_processing_spark.into_iter().sum::<u64>(), 1);
    assert_eq!(metrics.instructions_invalid_spark.into_iter().sum::<u64>(), 0);

    let resp2 = srv
        .asset_call(&asset_id, "issue_tokens")
        .send_json(&json!({ "token_ids": vec![token_id] }))
        .await
        .unwrap();

    assert!(resp2.status().is_success());
    let metrics = srv.metrics.send(GetMetrics).await.unwrap();
    // assert_eq!(metrics.total_calls["issue_tokens"], 2);
    assert_eq!(metrics.total_unique_instructions, 2);
    delay_for(Duration::from_secs(1)).await;
    let metrics = srv.metrics.send(GetMetrics).await.unwrap();
    assert_eq!(metrics.instructions_scheduled_spark.into_iter().sum::<u64>(), 2);
    assert_eq!(metrics.instructions_pending_spark.into_iter().sum::<u64>(), 1);
    assert_eq!(metrics.instructions_processing_spark.into_iter().sum::<u64>(), 2);
    assert_eq!(metrics.instructions_invalid_spark.into_iter().sum::<u64>(), 1);
}
