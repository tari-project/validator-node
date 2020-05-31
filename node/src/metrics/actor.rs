use super::*;
use actix::{prelude::*, utils::IntervalFunc};
use std::time::Duration;

/// Metrics Actor is updating charts every second as well as updating data on every incoming MetricEvent
impl Actor for Metrics {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        IntervalFunc::new(Duration::from_millis(1000), Self::tick)
            .finish()
            .spawn(ctx);
    }
}

/// Actor is processing [MetricEvent] amending self [Metrics] data
impl Handler<MetricEvent> for Metrics {
    type Result = ();

    fn handle(&mut self, msg: MetricEvent, _ctx: &mut Context<Self>) -> Self::Result {
        self.process_event(msg);
    }
}

/// [MetricsConfig] allows to configure shape of [Metrics] data, e.g. sparklines size
impl Handler<MetricsConfig> for Metrics {
    type Result = ();

    fn handle(&mut self, msg: MetricsConfig, _ctx: &mut Context<Self>) -> Self::Result {
        self.configure(msg);
    }
}

/// Provides current snapshot of metrics data
impl Handler<GetMetrics> for Metrics {
    type Result = MetricsSnapshot;

    fn handle(&mut self, _: GetMetrics, _ctx: &mut Context<Self>) -> Self::Result {
        MetricsSnapshot::from(&*self)
    }
}
