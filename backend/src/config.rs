use anyhow::{Context, Result};
use serde_json::Value as JsonValue;

#[derive(Clone, Debug)]
pub struct Config {
    pub registry_url: String,
    pub protocol_scope: String,
    pub protocol_name: String,
    pub protocol_parameters: JsonValue,
    pub u5c_url: String,
    pub u5c_api_key: Option<String>,
    pub database_path: String,
    pub txs: Vec<String>,
    pub server_addr: String,
    pub blockfrost_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let registry_url = get_env("REGISTRY_URL")?;
        let protocol_scope = get_env("PROTOCOL_SCOPE")?;
        let protocol_name = get_env("PROTOCOL_NAME")?;
        let protocol_parameters = serde_json::from_str(&get_env_optional("PROTOCOL_PARAMETERS").unwrap_or_else(|| "{}".to_string()))?;
        let u5c_url = get_env("U5C_URL")?;
        let database_path = get_env("DATABASE_PATH")?;
        let u5c_api_key = get_env_optional("U5C_API_KEY");
        let server_addr = get_env_optional("SERVER_ADDR").unwrap_or_else(|| "0.0.0.0:3000".to_string());
        let blockfrost_url = get_env("BLOCKFROST_URL")?;
        let txs = get_env_optional("TXS")
            .map(|value| {
                value
                    .split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(Self {
            registry_url,
            protocol_scope,
            protocol_name,
            protocol_parameters,
            u5c_url,
            u5c_api_key,
            database_path,
            txs,
            server_addr,
            blockfrost_url
        })
    }
}

fn get_env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("Missing env var: {key}"))
}

fn get_env_optional(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}
