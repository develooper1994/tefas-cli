//! Core configuration types shared across all `tefas-*` crates.
//!
//! The main entry point is [`AppConfig`], which aggregates network, TLS, retry, and auth
//! configuration. It is typically constructed from CLI arguments (see `tefas-cli`)
//! and passed to `tefas-network`'s `NetworkClient`.
//!
//! # Type hierarchy
//!
//! ```text
//! AppConfig
//! ├── backend:  HttpBackend   (which network stack to use)
//! ├── tls:      TlsConfig     (TLS backend + optional connect timeout)
//! │   └── backend: TlsBackend
//! ├── retry:    RetryConfig   (retry count + back-off delay)
//! └── auth:     AuthConfig    (token, cookies, User-Agent, Referer)
//! ```
//!
//! # Quick start
//!
//! ```rust
//! use tefas_config::{AppConfig, HttpBackend, TlsBackend, TlsConfig};
//!
//! let cfg = AppConfig {
//!     backend: HttpBackend::Hyper,
//!     tls: TlsConfig { backend: TlsBackend::NativeTls, timeout_ms: Some(5_000) },
//!     ..AppConfig::with_defaults()
//! };
//! ```

use serde::{Deserialize, Serialize};

pub use http_config::{
    DEFAULT_RETRY_BACKOFF_MS, DEFAULT_RETRY_COUNT, DEFAULT_TIMEOUT_SECS, HttpBackend, RetryConfig,
    TlsBackend, TlsConfig,
};
/// Default TEFAS base URL.
pub const DEFAULT_BASE_URL: &str = "https://www.tefas.gov.tr";
/// Default `User-Agent` header — uses a non-browser tool UA to bypass WAF JavaScript challenges.
/// The TEFAS WAF only issues JS challenges to browser UAs (Chrome/Firefox/Safari);
/// tool UAs such as `curl/8.5.0` receive a plain 307 redirect to the real page content.
pub const DEFAULT_USER_AGENT: &str = "curl/8.5.0";
/// Default `Referer` header used for preflight and operation requests.
pub const DEFAULT_REFERER: &str = "https://www.tefas.gov.tr/tr/fon-karsilastirma";

/// Authentication and browser-identity configuration.
///
/// Controls the `Authorization`, `Cookie`, `User-Agent`, and `Referer` headers,
/// and whether a preflight GET request is performed before the first API call.
///
/// See [`AppConfig::auth`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Optional `Authorization: Bearer <token>` value.
    pub token: Option<String>,
    /// Path to a Netscape-format cookies file. Cookies are parsed and sent in
    /// the `Cookie` header.
    pub cookies_file: Option<String>,
    /// `User-Agent` header. Defaults to [`DEFAULT_USER_AGENT`] (Chrome/Linux spoof).
    pub user_agent: String,
    /// `Referer` header used for preflight and all operation requests.
    /// Defaults to [`DEFAULT_REFERER`].
    pub referer: String,
    /// Optional `curl-impersonate` profile passed as `--impersonate <target>`.
    ///
    /// Only used when [`HttpBackend::Impcurl`] is selected. Examples include
    /// `chrome124`, `chrome136`, and `safari17_2_ios` depending on installed
    /// `curl-impersonate` build.
    pub impcurl_impersonate: Option<String>,
    /// When `true`, skip the preflight GET that warms up the session cookie.
    /// Useful when a valid session cookie is supplied via [`cookies_file`][Self::cookies_file].
    pub skip_preflight: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            token: None,
            cookies_file: None,
            user_agent: DEFAULT_USER_AGENT.to_string(),
            referer: DEFAULT_REFERER.to_string(),
            impcurl_impersonate: None,
            skip_preflight: false,
        }
    }
}

/// Top-level runtime configuration passed to `tefas-network`'s `NetworkClient`.
///
/// Aggregates all subsystem configurations. Construct with [`AppConfig::with_defaults`]
/// and override individual fields, or build entirely from CLI arguments (see `tefas-cli`).
///
/// # Fields at a glance
///
/// | Field | Type | Controls |
/// |-------|------|---------|
/// | `timeout_secs` | `u64` | Global request timeout |
/// | `backend` | [`HttpBackend`] | Network stack (reqwest / hyper / impcurl) |
/// | `tls` | [`TlsConfig`] | TLS backend + connect timeout |
/// | `retry` | [`RetryConfig`] | Retry count and back-off |
/// | `auth` | [`AuthConfig`] | Token, cookies, User-Agent, Referer |
///
/// # Example
///
/// ```rust
/// use tefas_config::{AppConfig, HttpBackend, TlsConfig, TlsBackend};
///
/// let cfg = AppConfig {
///     backend: HttpBackend::Reqwest,
///     tls: TlsConfig { backend: TlsBackend::Rustls, timeout_ms: None },
///     ..AppConfig::with_defaults()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Maximum total request duration in seconds.
    pub timeout_secs: u64,
    /// Pretty-print JSON output when `true`.
    pub pretty: bool,
    /// Base URL for TEFAS endpoints. Trailing slashes are stripped via
    /// [`normalized_base_url`][Self::normalized_base_url].
    pub base_url: String,
    /// HTTP client backend. See [`HttpBackend`].
    pub backend: HttpBackend,
    /// Retry policy for transient failures. See [`RetryConfig`].
    pub retry: RetryConfig,
    /// TLS backend and optional connect timeout. See [`TlsConfig`] and [`TlsBackend`].
    pub tls: TlsConfig,
    /// Authentication and browser-identity settings. See [`AuthConfig`].
    pub auth: AuthConfig,
    /// Optional HTTP/HTTPS proxy URL (e.g. `http://127.0.0.1:8080`).
    ///
    /// When set, all outbound requests are routed through this proxy. Supports
    /// `http://`, `https://`, and `socks5://` schemes. Also passed as `-x` to
    /// `curl-impersonate` when using the [`HttpBackend::Impcurl`] process fallback.
    pub proxy: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl AppConfig {
    /// Create an `AppConfig` with all fields set to sensible defaults.
    ///
    /// Uses [`DEFAULT_TIMEOUT_SECS`], [`DEFAULT_BASE_URL`], [`HttpBackend::Reqwest`],
    /// [`TlsBackend::Rustls`], and default retry/auth configurations.
    pub fn with_defaults() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            pretty: true,
            base_url: DEFAULT_BASE_URL.to_string(),
            backend: HttpBackend::Wreq,
            retry: RetryConfig::default(),
            tls: TlsConfig::default(),
            auth: AuthConfig::default(),
            proxy: None,
        }
    }

    /// Return [`base_url`][Self::base_url] with any trailing `/` characters stripped.
    ///
    /// Always use this when constructing endpoint URLs to avoid double-slashes.
    pub fn normalized_base_url(&self) -> String {
        self.base_url.trim_end_matches('/').to_string()
    }
}

#[cfg(test)]
mod tests;
