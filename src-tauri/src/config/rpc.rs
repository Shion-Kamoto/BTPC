use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
}
