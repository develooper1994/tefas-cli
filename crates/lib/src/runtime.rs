use anyhow::Context;
use serde_json::Value;
use tefas_config::AppConfig;
use tefas_network::NetworkClient;
use tokio::runtime::Runtime;

/// Simple blocking runtime wrapper for non-async consumers.
pub struct BlockingClient {
    runtime: Runtime,
    client: NetworkClient,
}

impl BlockingClient {
    pub fn new(cfg: &AppConfig) -> anyhow::Result<Self> {
        let runtime = Runtime::new().context("failed to create tokio runtime")?;
        let client = NetworkClient::new(cfg)?;
        Ok(Self { runtime, client })
    }

    pub fn preflight(&self) -> anyhow::Result<()> {
        self.runtime.block_on(self.client.preflight())
    }

    pub fn fetch_text(&self, url: &str) -> anyhow::Result<String> {
        self.runtime.block_on(self.client.fetch_text(url))
    }

    pub fn get_json(&self, url: &str) -> anyhow::Result<Value> {
        self.runtime.block_on(self.client.get_json(url))
    }

    pub fn post_json_with_referer(
        &self,
        url: &str,
        referer: &str,
        payload: &Value,
    ) -> anyhow::Result<Value> {
        self.runtime
            .block_on(self.client.post_json_with_referer(url, referer, payload))
    }
}
