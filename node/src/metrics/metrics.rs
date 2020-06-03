//! Metrics is a centralized collector of tari-validator-node metrics.
//!
//! This is of demo-display only purpose, hence provides oversimplified implementation,
//! it does no guarantee correct timing data under heavy load, also loses all data
//! on actor reset, though this should be fine for displaying realtime stats in CLI UI.

use super::{events::*, LOG_TARGET};
use crate::{db::models::InstructionStatus, types::InstructionID};
use actix::{Context, Message, MessageResponse};
use deadpool_postgres::Pool;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

const SPARKLINE_MAX_SIZE_DEFAULT: usize = 80;

#[derive(Clone, Default)]
/// Metrics collect information from event for display:
/// 1. Turning events into displayable data
/// 2. Handler for events processing
pub struct Metrics {
    pool: Option<Arc<Pool>>,
    instructions_scheduled_spark: Sparkline,
    instructions_processing_spark: Sparkline,
    instructions_pending_spark: Sparkline,
    instructions_invalid_spark: Sparkline,
    instructions_commit_spark: Sparkline,
    current_processing_instructions: u64,
    current_pending_instructions: u64,
    unique_instructions_counter: HashSet<InstructionID>,
    calls_counter: HashMap<String, u64>,
    // TODO: instruction_time_in_status: HashMap<(InstructionStatus,InstructionID),
}

impl Metrics {
    pub fn new(pool: Arc<Pool>) -> Self {
        Self {
            pool: Some(pool),
            ..Default::default()
        }
    }

    pub(super) fn configure(&mut self, config: MetricsConfig) {
        self.instructions_pending_spark
            .set_max_size(config.instructions_spark_sizes);
        self.instructions_processing_spark
            .set_max_size(config.instructions_spark_sizes);
        self.instructions_scheduled_spark
            .set_max_size(config.instructions_spark_sizes);
        self.instructions_invalid_spark
            .set_max_size(config.instructions_spark_sizes);
        self.instructions_commit_spark
            .set_max_size(config.instructions_spark_sizes);
    }

    // Supposed to be called every second and shifting sparkline data
    pub(super) fn tick(&mut self, _: &mut Context<Self>) {
        log::trace!(target: LOG_TARGET, "updating time-bound metrics charts data");
        self.instructions_pending_spark.shift();
        self.instructions_processing_spark.shift();
        self.instructions_scheduled_spark.shift();
        self.instructions_invalid_spark.shift();
        self.instructions_commit_spark.shift();
    }

    pub(super) fn process_event(&mut self, event: MetricEvent) {
        match event {
            MetricEvent::Call(ContractCallEvent { contract_name, .. }) => {
                if let Some(counter) = self.calls_counter.get_mut(&contract_name) {
                    *counter += 1;
                } else {
                    self.calls_counter.insert(contract_name, 1);
                }
            },
            MetricEvent::Instruction(InstructionEvent { id, status, .. }) => {
                match status {
                    InstructionStatus::Pending => {
                        self.instructions_pending_spark.inc();
                        self.current_processing_instructions = self.current_processing_instructions.saturating_sub(1);
                        self.current_pending_instructions += 1;
                    },
                    InstructionStatus::Scheduled => self.instructions_scheduled_spark.inc(),
                    InstructionStatus::Processing => {
                        self.current_processing_instructions += 1;
                        self.instructions_processing_spark.inc()
                    },
                    InstructionStatus::Invalid => {
                        self.current_processing_instructions = self.current_processing_instructions.saturating_sub(1);
                        self.instructions_invalid_spark.inc()
                    },
                    InstructionStatus::Commit => {
                        self.instructions_pending_spark.inc();
                        // TODO: for better precision should be HashSet of instruction_id. or separate status for when
                        // it fails Commit.
                        self.current_pending_instructions = self.current_pending_instructions.saturating_sub(1);
                    },
                };
                self.unique_instructions_counter.insert(id);
            },
        }
    }
}

#[derive(Message)]
#[rtype(result = "MetricsSnapshot")]
/// Get current state of metrics counters,
/// will return MetricsSnapshot back
pub struct GetMetrics;

#[derive(MessageResponse)]
/// Representation of [Metrics] data snapshot suitable for display
pub struct MetricsSnapshot {
    // Note: this should work much faster than HashMap<InstructionStatus..>
    pub instructions_scheduled_spark: Vec<u64>,
    pub instructions_processing_spark: Vec<u64>,
    pub instructions_pending_spark: Vec<u64>,
    pub instructions_invalid_spark: Vec<u64>,
    pub instructions_commit_spark: Vec<u64>,
    pub current_processing_instructions: u64,
    pub current_pending_instructions: u64,
    pub total_unique_instructions: u64,
    pub total_calls: HashMap<String, u64>,
    pub pool_status: Option<deadpool::Status>,
}

impl From<&Metrics> for MetricsSnapshot {
    fn from(metrics: &Metrics) -> Self {
        Self {
            instructions_scheduled_spark: metrics.instructions_scheduled_spark.to_vec(),
            instructions_processing_spark: metrics.instructions_processing_spark.to_vec(),
            instructions_pending_spark: metrics.instructions_pending_spark.to_vec(),
            instructions_invalid_spark: metrics.instructions_invalid_spark.to_vec(),
            instructions_commit_spark: metrics.instructions_commit_spark.to_vec(),
            current_processing_instructions: metrics.current_processing_instructions,
            current_pending_instructions: metrics.current_pending_instructions,
            total_unique_instructions: metrics.unique_instructions_counter.len() as u64,
            total_calls: metrics.calls_counter.clone(),
            pool_status: metrics.pool.as_ref().map(|p| p.status()),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
/// Configures Metrics, setting up dimensions for displayable data
pub struct MetricsConfig {
    pub instructions_spark_sizes: usize,
}

// apart of accepting public events Metrics will receive a beat once a second to shift all sparklines

#[derive(Clone)]
pub struct Sparkline {
    max_size: usize,
    data: VecDeque<u64>,
}

impl Default for Sparkline {
    fn default() -> Self {
        Self {
            max_size: SPARKLINE_MAX_SIZE_DEFAULT,
            data: vec![0; SPARKLINE_MAX_SIZE_DEFAULT].into(),
        }
    }
}

impl Sparkline {
    fn inc(&mut self) {
        // we should be safe as by default it has at least 1 item
        *self.data.back_mut().unwrap() += 1;
    }

    fn shift(&mut self) {
        if self.data.len() >= self.max_size {
            let _ = self.data.pop_front();
        }
        self.data.push_back(0);
    }

    fn set_max_size(&mut self, max_size: usize) {
        self.max_size = max_size;
        if self.data.len() > self.max_size {
            let excess = 0..self.data.len() - self.max_size;
            let _ = self.data.drain(excess).collect::<VecDeque<_>>();
        }
        while self.data.len() < self.max_size {
            self.data.push_front(0);
        }
    }

    fn to_vec(&self) -> Vec<u64> {
        self.data.iter().copied().collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // TODO: more unit tests - there is limited coverage with super module tests

    #[test]
    fn sparkline_shifts() {
        let mut sparks = Sparkline::default();
        let mut vec = vec![0; SPARKLINE_MAX_SIZE_DEFAULT];
        assert_eq!(sparks.to_vec(), vec);
        sparks.inc();
        vec.last_mut().map(|item| *item = 1);
        assert_eq!(sparks.to_vec(), vec);
        sparks.inc();
        vec.last_mut().map(|item| *item = 2);
        assert_eq!(sparks.to_vec(), vec);
        sparks.shift();
        let _: Vec<_> = vec.drain(0..1).collect();
        vec.push(0);
        assert_eq!(sparks.to_vec(), vec);
        sparks.shift();
        let _: Vec<_> = vec.drain(0..1).collect();
        vec.push(0);
        assert_eq!(sparks.to_vec(), vec);
        sparks.inc();
        vec.last_mut().map(|item| *item = 1);
        assert_eq!(sparks.to_vec(), vec);
    }

    #[test]
    fn sparkline_default_max_size() {
        let mut sparks = Sparkline::default();
        sparks.inc();
        let mut res = vec![1u64];
        for _ in 1..SPARKLINE_MAX_SIZE_DEFAULT {
            sparks.shift();
            res.push(0);
        }
        assert_eq!(sparks.to_vec(), res);
        sparks.shift();
        assert_eq!(sparks.to_vec(), vec![0u64; SPARKLINE_MAX_SIZE_DEFAULT]);
    }

    #[test]
    fn sparkline_custom_max_size() {
        let mut sparks = Sparkline::default();
        sparks.inc();
        sparks.set_max_size(1);
        assert_eq!(sparks.to_vec(), vec![1]);
        sparks.shift();
        assert_eq!(sparks.to_vec(), vec![0]);
        sparks.set_max_size(2);
        sparks.shift();
        assert_eq!(sparks.to_vec(), vec![0, 0]);
        sparks.inc();
        assert_eq!(sparks.to_vec(), vec![0, 1]);
        sparks.shift();
        assert_eq!(sparks.to_vec(), vec![1, 0]);
        sparks.set_max_size(3);
        sparks.shift();
        sparks.inc();
        assert_eq!(sparks.to_vec(), vec![1, 0, 1]);
        sparks.set_max_size(2);
        assert_eq!(sparks.to_vec(), vec![0, 1]);
        sparks.shift();
        sparks.inc();
        assert_eq!(sparks.to_vec(), vec![1, 1]);
    }
}
