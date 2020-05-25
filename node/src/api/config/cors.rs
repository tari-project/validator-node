use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: String,
}
impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: "*".to_string(),
        }
    }
}
