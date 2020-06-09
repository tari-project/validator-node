use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub runner_max_jobs: usize,
}
impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            runner_max_jobs: num_cpus::get() * 10,
        }
    }
}
