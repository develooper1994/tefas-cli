use serde::{Deserialize, Serialize};

pub const DEFAULT_TIMEOUT_SECS: u64 = 25;
pub const DEFAULT_RETRY_COUNT: u32 = 2;
pub const DEFAULT_RETRY_BACKOFF_MS: u64 = 400;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum HttpBackend {
    Wreq,
    Reqwest,
    Hyper,
    Impcurl,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TlsBackend {
    #[default]
    Rustls,
    NativeTls,
    Insecure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub backend: TlsBackend,
    pub timeout_ms: Option<u64>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            backend: TlsBackend::Rustls,
            timeout_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub count: u32,
    pub backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            count: DEFAULT_RETRY_COUNT,
            backoff_ms: DEFAULT_RETRY_BACKOFF_MS,
        }
    }
}
