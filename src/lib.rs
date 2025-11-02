pub mod server;

use serde::{Deserialize, Serialize};
use url::Url;

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/78.0.3904.108 Safari/537.36";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub api_key: String,
    pub base_path: Url,
    pub rate_limit: i64,
    pub proxies: Vec<String>,
}

impl AppConfig {
    pub fn try_create() -> anyhow::Result<Self> {
        let config_str = std::fs::read_to_string("assets/config.toml")?;
        let config: AppConfig = toml::from_str(&config_str)?;

        Ok(config)
    }
}
