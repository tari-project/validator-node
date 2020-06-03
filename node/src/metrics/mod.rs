pub mod actor;
pub mod events;
pub mod metrics;

pub use events::{ContractCallEvent, InstructionEvent, MetricEvent};
pub use metrics::{GetMetrics, Metrics, MetricsConfig, MetricsSnapshot};

pub const LOG_TARGET: &'static str = "tari_validator_node::metrics";

#[cfg(test)]
mod test {
    use super::*;
    use crate::{db::models::InstructionStatus, test::utils::Test, types::InstructionID};
    use actix::Actor;
    use std::time::Duration;
    use tokio::time::delay_for;

    #[actix_rt::test]
    async fn contract_call_actor_counters() {
        let addr = Metrics::default().start();

        let event: MetricEvent = ContractCallEvent {
            contract_name: "contract1".into(),
        }
        .into();
        addr.send(event.clone()).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.total_calls["contract1"], 1);
        addr.send(event).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.total_calls["contract1"], 2);

        let event2: MetricEvent = ContractCallEvent {
            contract_name: "contract2".into(),
        }
        .into();
        addr.send(event2).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.total_calls["contract1"], 2);
        assert_eq!(metrics.total_calls["contract2"], 1);
    }

    #[actix_rt::test]
    async fn unique_instruction_actor_counter() {
        let addr = Metrics::default().start();

        let id = Test::<InstructionID>::new();
        let event: MetricEvent = InstructionEvent {
            id,
            status: InstructionStatus::Pending,
        }
        .into();
        addr.send(event.clone()).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.total_unique_instructions, 1);
        addr.send(event).await.unwrap();
        let event2: MetricEvent = InstructionEvent {
            id,
            status: InstructionStatus::Processing,
        }
        .into();
        addr.send(event2).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.total_unique_instructions, 1);

        let id2 = Test::<InstructionID>::new();
        let event3: MetricEvent = InstructionEvent {
            id: id2,
            status: InstructionStatus::Pending,
        }
        .into();
        addr.send(event3).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.total_unique_instructions, 2);
    }

    #[actix_rt::test]
    async fn instruction_spark_actor_counters_timed() {
        let _ = pretty_env_logger::try_init();
        let addr = Metrics::default().start();
        let _ = addr
            .send(MetricsConfig {
                instructions_spark_sizes: 3,
            })
            .await
            .unwrap();

        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.instructions_scheduled_spark, vec![0, 0, 0]);

        let id = Test::<InstructionID>::new();
        let event: MetricEvent = InstructionEvent {
            id,
            status: InstructionStatus::Scheduled,
        }
        .into();
        addr.send(event.clone()).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.instructions_scheduled_spark, vec![0, 0, 1]);
        delay_for(Duration::from_millis(1500)).await;

        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.instructions_scheduled_spark, vec![0, 1, 0]);
        assert_eq!(metrics.instructions_processing_spark, vec![0, 0, 0]);

        let event: MetricEvent = InstructionEvent {
            id,
            status: InstructionStatus::Scheduled,
        }
        .into();
        addr.send(event).await.unwrap();
        let event2: MetricEvent = InstructionEvent {
            id,
            status: InstructionStatus::Processing,
        }
        .into();
        addr.send(event2).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.instructions_scheduled_spark, vec![0, 1, 1]);
        assert_eq!(metrics.instructions_processing_spark, vec![0, 0, 1]);
        assert_eq!(metrics.instructions_pending_spark, vec![0, 0, 0]);

        let id2 = Test::<InstructionID>::new();
        let event3: MetricEvent = InstructionEvent {
            id,
            status: InstructionStatus::Pending,
        }
        .into();
        addr.send(event3).await.unwrap();
        let event4: MetricEvent = InstructionEvent {
            id: id2,
            status: InstructionStatus::Scheduled,
        }
        .into();
        addr.send(event4.clone()).await.unwrap();
        addr.send(event4).await.unwrap();
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.instructions_pending_spark, vec![0, 0, 1]);
        assert_eq!(metrics.instructions_processing_spark, vec![0, 0, 1]);
        assert_eq!(metrics.instructions_scheduled_spark, vec![0, 1, 3]);

        delay_for(Duration::from_millis(1000)).await;
        let metrics = addr.send(GetMetrics).await.unwrap();
        assert_eq!(metrics.instructions_pending_spark, vec![0, 1, 0]);
        assert_eq!(metrics.instructions_processing_spark, vec![0, 1, 0]);
        assert_eq!(metrics.instructions_scheduled_spark, vec![1, 3, 0]);
    }
}
