use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub workers: Option<usize>,
}
impl Default for ConsensusConfig {
    fn default() -> Self {
        Self { workers: None }
    }
}
