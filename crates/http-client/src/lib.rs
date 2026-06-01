//! HTTP client implementation for TEFAS API requests.
//!
//! Provides [`NetworkClient`], a unified async HTTP client that delegates to one of three
//! network backends ([`HttpBackend::Reqwest`], [`HttpBackend::Hyper`],
//! [`HttpBackend::Impcurl`]) combined with a configurable TLS implementation
//! ([`TlsBackend`][http_config::TlsBackend]).
//!
//! # Backend × TLS compatibility
//!
//! | Backend | Rustls | NativeTls | Insecure |
//! |---------|--------|-----------|----------|
//! | wreq    | N/A¹   | N/A¹      | N/A¹     |
//! | reqwest | ✅ | ✅ | ✅ |
//! | hyper   | ✅ | ✅ | ❌ |
//! | impcurl | — | — | — |
//!
//! ¹ wreq has built-in BoringSSL; the `--tls` flag is ignored.
//!
//! # Usage
//!
//! ```no_run
//! use http_client::{NetworkAuthConfig, NetworkClient, NetworkClientConfig};
//! use http_config::{HttpBackend, RetryConfig, TlsConfig, TlsBackend};
//!
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let cfg = NetworkClientConfig {
//!     timeout_secs: 25,
//!     base_url: "https://example.com".to_string(),
//!     backend: HttpBackend::Reqwest,
//!     retry: RetryConfig::default(),
//!     tls: TlsConfig { backend: TlsBackend::NativeTls, timeout_ms: None },
//!     auth: NetworkAuthConfig {
//!         token: None,
//!         cookies_file: None,
//!         user_agent: "curl/8.5.0".to_string(),
//!         referer: "https://example.com".to_string(),
//!         impcurl_impersonate: None,
//!         skip_preflight: false,
//!     },
//!     proxy: None,
//! };
//! let client = NetworkClient::from_config(cfg)?;
//! let json = client.get_json("https://example.com/api").await?;
//! # Ok(())
//! # }
//! ```

mod builder;
#[cfg(feature = "hyper-backend")]
mod hyper_exec;
mod impcurl;
mod postback;
mod request_utils;
mod retry;

pub use postback::{extract_hidden_fields, form_urlencode};

use anyhow::Context;
use http::Method;
use http::header::{
    ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, COOKIE, ORIGIN, REFERER,
    USER_AGENT,
};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use http_config::{HttpBackend, RetryConfig, TlsBackend, TlsConfig};
use serde_json::Value;
use std::fmt;
use std::fs;
use std::sync::atomic::Ordering;
use std::time::Duration;
#[cfg(feature = "tefas-compat")]
use tefas_config::AppConfig;
use tokio::time::{sleep, timeout};
use tracing::{debug, warn};

/// Error type used when a request returns a non-success HTTP status code.
#[derive(Debug)]
pub struct StatusError {
    pub status: u16,
    pub url: String,
}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "non-success status {} for {}", self.status, self.url)
    }
}

impl std::error::Error for StatusError {}

/// Type aliases re-exported from the builder module.
#[cfg(feature = "hyper-backend")]
use builder::{HyperBody, HyperClient, HyperRustlsConnector};
use std::sync::Arc;

#[cfg(all(feature = "hyper-backend", feature = "native-tls"))]
use builder::HyperNativeTlsConnector;

enum BackendClient {
    /// BoringSSL-backed wreq client (default).
    #[cfg(feature = "wreq-backend")]
    Wreq(wreq::Client),
    /// High-level reqwest client (handles redirects automatically).
    Reqwest(reqwest::Client),
    /// Low-level hyper client with rustls TLS connector.
    ///
    /// Boxed because `HyperClient` is large (~300 bytes) and would otherwise
    /// dominate the size of every `BackendClient` discriminant.
    #[cfg(feature = "hyper-backend")]
    HyperRustls(Box<HyperClient<HyperRustlsConnector, HyperBody>>),
    /// Low-level hyper client with native-TLS connector.
    /// Uses OpenSSL on Linux, Schannel on Windows, SecureTransport on macOS.
    #[cfg(all(feature = "hyper-backend", feature = "native-tls"))]
    HyperNativeTls(Box<HyperClient<HyperNativeTlsConnector, HyperBody>>),
    /// Path to the resolved `curl-impersonate` CLI binary.
    Impcurl {
        binary: Option<String>,
        library: reqwest::Client,
    },
}

#[derive(Clone)]
pub struct NetworkClient {
    inner: Arc<NetworkClientInner>,
}

struct NetworkClientInner {
    backend: BackendClient,
    cfg: NetworkClientConfig,
    base_headers: HeaderMap,
}

#[derive(Debug, Clone)]
pub struct NetworkAuthConfig {
    pub token: Option<String>,
    pub cookies_file: Option<String>,
    pub user_agent: String,
    pub referer: String,
    pub impcurl_impersonate: Option<String>,
    pub skip_preflight: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkClientConfig {
    pub timeout_secs: u64,
    pub base_url: String,
    pub backend: HttpBackend,
    pub retry: RetryConfig,
    pub tls: TlsConfig,
    pub auth: NetworkAuthConfig,
    pub proxy: Option<String>,
}

impl NetworkClientConfig {
    pub fn normalized_base_url(&self) -> String {
        self.base_url.trim_end_matches('/').to_string()
    }
}

#[cfg(feature = "tefas-compat")]
impl From<&AppConfig> for NetworkClientConfig {
    fn from(value: &AppConfig) -> Self {
        Self {
            timeout_secs: value.timeout_secs,
            base_url: value.base_url.clone(),
            backend: value.backend,
            retry: value.retry.clone(),
            tls: value.tls.clone(),
            auth: NetworkAuthConfig {
                token: value.auth.token.clone(),
                cookies_file: value.auth.cookies_file.clone(),
                user_agent: value.auth.user_agent.clone(),
                referer: value.auth.referer.clone(),
                impcurl_impersonate: value.auth.impcurl_impersonate.clone(),
                skip_preflight: value.auth.skip_preflight,
            },
            proxy: value.proxy.clone(),
        }
    }
}

impl NetworkClient {
    const NAVIGATION_ACCEPT: &'static str = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7";
    const API_ACCEPT: &'static str = "*/*";

    fn debug_http_enabled() -> bool {
        request_utils::debug_http_enabled()
    }

    fn dump_http_request(method: &Method, url: &str, headers: &HeaderMap) {
        request_utils::dump_http_request(method, url, headers)
    }

    fn looks_like_waf_challenge(text: &str) -> bool {
        request_utils::looks_like_waf_challenge(text)
    }

    fn compute_sec_fetch_site(url: &str, referer: Option<&str>) -> &'static str {
        request_utils::compute_sec_fetch_site(url, referer)
    }

    #[cfg(feature = "tefas-compat")]
    pub fn new(cfg: &AppConfig) -> anyhow::Result<Self> {
        Self::from_config(NetworkClientConfig::from(cfg))
    }

    pub fn from_config(cfg: NetworkClientConfig) -> anyhow::Result<Self> {
        let backend = match cfg.backend {
            HttpBackend::Wreq => {
                #[cfg(feature = "wreq-backend")]
                {
                    BackendClient::Wreq(builder::build_wreq(&cfg)?)
                }
                #[cfg(not(feature = "wreq-backend"))]
                {
                    return Err(anyhow::anyhow!(
                        "HttpBackend::Wreq requires the `wreq-backend` cargo feature; \
                         rebuild with `--features wreq-backend`"
                    ));
                }
            }
            HttpBackend::Reqwest => {
                let tls = cfg.tls.backend;
                BackendClient::Reqwest(builder::build_reqwest(&cfg, tls)?)
            }
            HttpBackend::Hyper => {
                #[cfg(feature = "hyper-backend")]
                match cfg.tls.backend {
                    TlsBackend::Rustls => {
                        BackendClient::HyperRustls(Box::new(builder::build_hyper_rustls(&cfg)?))
                    }
                    TlsBackend::NativeTls => {
                        #[cfg(feature = "native-tls")]
                        {
                            BackendClient::HyperNativeTls(Box::new(
                                builder::build_hyper_native_tls(&cfg)?,
                            ))
                        }
                        #[cfg(not(feature = "native-tls"))]
                        {
                            return Err(anyhow::anyhow!(
                                "TlsBackend::NativeTls requires the `native-tls` cargo feature; \
                                 rebuild with `--features native-tls`"
                            ));
                        }
                    }
                    TlsBackend::Insecure => {
                        return Err(anyhow::anyhow!(
                            "TlsBackend::Insecure is not supported with the hyper backend; \
                             use --backend reqwest --tls insecure"
                        ));
                    }
                }
                #[cfg(not(feature = "hyper-backend"))]
                {
                    return Err(anyhow::anyhow!(
                        "HttpBackend::Hyper requires the `hyper-backend` cargo feature; \
                         rebuild with `--features hyper-backend`"
                    ));
                }
            }
            HttpBackend::Impcurl => {
                let library = builder::build_impcurl_library_client(&cfg)?;
                let binary = impcurl::find_binary().ok();
                BackendClient::Impcurl { binary, library }
            }
        };

        let base_headers = Self::build_base_headers(&cfg)?;

        Ok(Self {
            inner: Arc::new(NetworkClientInner {
                backend,
                cfg: cfg.clone(),
                base_headers,
            }),
        })
    }

    fn build_base_headers(cfg: &NetworkClientConfig) -> anyhow::Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&cfg.auth.user_agent).context("invalid user-agent header")?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static(Self::NAVIGATION_ACCEPT));
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("tr-TR,tr;q=0.9,en-US;q=0.8,en;q=0.7"),
        );
        headers.insert(
            ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br, zstd"),
        );

        if let Some(token) = &cfg.auth.token {
            let val = format!("Bearer {}", token.trim());
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&val).context("invalid authorization header")?,
            );
        }

        if let Some(path) = &cfg.auth.cookies_file {
            let cookie = Self::parse_cookie_file(path);
            if !cookie.is_empty() {
                headers.insert(
                    COOKIE,
                    HeaderValue::from_str(&cookie).context("invalid cookie header")?,
                );
            }
        }

        Ok(headers)
    }

    fn parse_cookie_file(path: &str) -> String {
        let content = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return String::new(),
        };
        let mut cookies = Vec::new();
        for line in content.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if t.contains('\t') {
                let mut cols = t.split('\t');
                let name = cols.nth(5);
                let value = cols.next();
                if let (Some(name), Some(value)) = (name, value) {
                    cookies.push(format!("{}={}", name, value));
                    continue;
                }
            }
            if let Some((name, value)) = t.split_once('=') {
                let name = name.trim();
                let value = value.trim();
                if !name.is_empty() {
                    cookies.push(format!("{}={}", name, value));
                }
            }
        }
        cookies.join("; ")
    }

    fn headers(&self) -> anyhow::Result<HeaderMap> {
        Ok(self.inner.base_headers.clone())
    }

    async fn request_text_via_reqwest_client(
        &self,
        client: &reqwest::Client,
        method: Method,
        url: &str,
        body: Option<Vec<u8>>,
        referer: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut req = client.request(method.clone(), url);
        let mut headers = self.headers()?;
        let effective_referer = referer.or(Some(self.inner.cfg.auth.referer.as_str()));
        if let Some(rf) = effective_referer {
            headers.insert(
                REFERER,
                HeaderValue::from_str(rf).context("invalid referer header")?,
            );
        }

        let is_navigation = method == Method::GET && body.is_none();
        let sec_fetch_site = Self::compute_sec_fetch_site(url, effective_referer);
        headers.insert(
            HeaderName::from_static("sec-fetch-site"),
            HeaderValue::from_str(sec_fetch_site).context("invalid sec-fetch-site header")?,
        );
        if is_navigation {
            headers.insert(ACCEPT, HeaderValue::from_static(Self::NAVIGATION_ACCEPT));
            headers.insert(
                HeaderName::from_static("sec-fetch-mode"),
                HeaderValue::from_static("navigate"),
            );
            headers.insert(
                HeaderName::from_static("sec-fetch-dest"),
                HeaderValue::from_static("document"),
            );
            headers.insert(
                HeaderName::from_static("sec-fetch-user"),
                HeaderValue::from_static("?1"),
            );
            headers.insert(
                HeaderName::from_static("upgrade-insecure-requests"),
                HeaderValue::from_static("1"),
            );
        } else {
            headers.insert(ACCEPT, HeaderValue::from_static(Self::API_ACCEPT));
            headers.insert(
                HeaderName::from_static("sec-fetch-mode"),
                HeaderValue::from_static("cors"),
            );
            headers.insert(
                HeaderName::from_static("sec-fetch-dest"),
                HeaderValue::from_static("empty"),
            );
            headers.remove(HeaderName::from_static("sec-fetch-user"));
            headers.remove(HeaderName::from_static("upgrade-insecure-requests"));
        }

        if Self::debug_http_enabled() {
            Self::dump_http_request(&method, url, &headers);
        }
        req = req.headers(headers);
        if let Some(body_bytes) = body {
            req = req
                .header(CONTENT_TYPE, "application/json; charset=UTF-8")
                .header(ORIGIN, self.inner.cfg.normalized_base_url())
                .header(
                    HeaderName::from_static("x-requested-with"),
                    "XMLHttpRequest",
                )
                .body(body_bytes);
        }

        let request = req.build().context("failed to build reqwest request")?;

        let send_fut = client.execute(request);
        let resp = if let Some(timeout_ms) = self.inner.cfg.tls.timeout_ms {
            timeout(Duration::from_millis(timeout_ms), send_fut)
                .await
                .context("request timeout")??
        } else {
            send_fut.await?
        };

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.is_success() {
            Ok(text)
        } else {
            Err(anyhow::Error::new(StatusError {
                status: status.as_u16(),
                url: url.to_string(),
            }))
        }
    }

    /// Send a request using the wreq BoringSSL-backed client.
    #[cfg(feature = "wreq-backend")]
    async fn request_text_via_wreq_client(
        &self,
        client: &wreq::Client,
        method: Method,
        url: &str,
        body: Option<Vec<u8>>,
        referer: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut headers = self.headers()?;
        let effective_referer = referer.or(Some(self.inner.cfg.auth.referer.as_str()));
        if let Some(rf) = effective_referer {
            headers.insert(
                REFERER,
                HeaderValue::from_str(rf).context("invalid referer header")?,
            );
        }
        if body.is_none() {
            headers.insert(ACCEPT, HeaderValue::from_static(Self::NAVIGATION_ACCEPT));
        } else {
            headers.insert(ACCEPT, HeaderValue::from_static(Self::API_ACCEPT));
        }

        if Self::debug_http_enabled() {
            Self::dump_http_request(&method, url, &headers);
        }

        let wreq_method = wreq::Method::from_bytes(method.as_str().as_bytes())
            .context("invalid HTTP method for wreq")?;
        let mut req = client.request(wreq_method, url).headers(headers);
        if let Some(data) = body {
            req = req
                .header(
                    wreq::header::CONTENT_TYPE,
                    "application/json; charset=UTF-8",
                )
                .header(wreq::header::ORIGIN, self.inner.cfg.normalized_base_url())
                .header(
                    wreq::header::HeaderName::from_static("x-requested-with"),
                    "XMLHttpRequest",
                )
                .body(data);
        }

        let send_fut = req.send();
        let resp = if let Some(timeout_ms) = self.inner.cfg.tls.timeout_ms {
            timeout(Duration::from_millis(timeout_ms), send_fut)
                .await
                .context("wreq request timeout")??
        } else {
            send_fut.await.context("wreq request failed")?
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.is_success() {
            Ok(text)
        } else {
            Err(anyhow::Error::new(StatusError {
                status: status.as_u16(),
                url: url.to_string(),
            }))
        }
    }

    async fn request_text_via_impcurl_process(
        &self,
        binary: &str,
        method: Method,
        url: &str,
        body: Option<Vec<u8>>,
        referer: Option<&str>,
    ) -> anyhow::Result<String> {
        let lib_dir = impcurl::lib_dir_for(binary);
        let mut args: Vec<String> = vec!["-L".to_string(), "-s".to_string()];
        let profile = self.inner.cfg.auth.impcurl_impersonate.as_deref();
        if let Some(p) = profile {
            args.push("--impersonate".to_string());
            args.push(p.to_string());
        }
        if let Some(proxy_url) = &self.inner.cfg.proxy {
            args.push("-x".to_string());
            args.push(proxy_url.clone());
        }

        if method == Method::POST {
            let json_body = String::from_utf8(body.unwrap_or_default())
                .context("POST body is not valid UTF-8")?;
            let post_referer = referer.unwrap_or(self.inner.cfg.auth.referer.as_str());
            let origin = self.inner.cfg.normalized_base_url();
            args.extend_from_slice(&[
                "-X".to_string(),
                "POST".to_string(),
                "-H".to_string(),
                "Content-Type: application/json; charset=UTF-8".to_string(),
                "-H".to_string(),
                format!("Origin: {}", origin),
                "-H".to_string(),
                "X-Requested-With: XMLHttpRequest".to_string(),
                "-H".to_string(),
                format!("Referer: {}", post_referer),
                "-d".to_string(),
                json_body,
                url.to_string(),
            ]);
        } else {
            args.push(url.to_string());
        }

        let mut output = impcurl::run(binary, lib_dir.as_ref(), &args).await?;

        if profile.is_some() && !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if impcurl::unknown_impersonate(&stderr) {
                if !impcurl::IMPCURL_IMPERSONATE_FALLBACK_WARNED.swap(true, Ordering::Relaxed) {
                    eprintln!(
                        "warning: curl-impersonate binary does not support --impersonate; retrying without profile"
                    );
                }
                let mut fallback_args: Vec<String> = Vec::with_capacity(args.len());
                let mut i = 0usize;
                while i < args.len() {
                    if args[i] == "--impersonate" {
                        i += 2;
                        continue;
                    }
                    fallback_args.push(args[i].clone());
                    i += 1;
                }
                output = impcurl::run(binary, lib_dir.as_ref(), &fallback_args).await?;
            }
        }

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!(
                "curl-impersonate exited with {}: {}",
                output.status,
                stderr.trim()
            ))
        }
    }

    async fn postback_via_reqwest_client(
        &self,
        client: &reqwest::Client,
        url: &str,
        headers: &HeaderMap,
        fields: &[(String, String)],
    ) -> anyhow::Result<String> {
        let form_pairs: Vec<(&str, &str)> = fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let mut req = client.post(url);
        for (name, value) in headers {
            req = req.header(name.clone(), value.clone());
        }
        req = req.form(&form_pairs);

        let send_fut = req.send();
        let resp = if let Some(timeout_ms) = self.inner.cfg.tls.timeout_ms {
            timeout(Duration::from_millis(timeout_ms), send_fut)
                .await
                .context("postback: request timeout")??
        } else {
            send_fut.await.context("postback: POST failed")?
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.is_success() {
            Ok(text)
        } else {
            Err(anyhow::anyhow!(
                "postback: non-success status {} for {}",
                status,
                url
            ))
        }
    }

    #[cfg(feature = "wreq-backend")]
    async fn postback_via_wreq_client(
        &self,
        client: &wreq::Client,
        url: &str,
        headers: &HeaderMap,
        fields: &[(String, String)],
    ) -> anyhow::Result<String> {
        let form_pairs: Vec<(&str, &str)> = fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let mut req = client.post(url);
        for (name, value) in headers {
            let wreq_name = wreq::header::HeaderName::from_bytes(name.as_ref())
                .context("postback: invalid header name for wreq")?;
            let wreq_value = wreq::header::HeaderValue::from_bytes(value.as_bytes())
                .context("postback: invalid header value for wreq")?;
            req = req.header(wreq_name, wreq_value);
        }
        req = req.form(&form_pairs);

        let send_fut = req.send();
        let resp = if let Some(timeout_ms) = self.inner.cfg.tls.timeout_ms {
            timeout(Duration::from_millis(timeout_ms), send_fut)
                .await
                .context("postback: wreq request timeout")??
        } else {
            send_fut.await.context("postback: wreq POST failed")?
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.is_success() {
            Ok(text)
        } else {
            Err(anyhow::anyhow!(
                "postback: non-success status {} for {}",
                status,
                url
            ))
        }
    }

    async fn postback_via_impcurl_process(
        &self,
        binary: &str,
        url: &str,
        form_body: Vec<u8>,
    ) -> anyhow::Result<String> {
        let lib_dir = impcurl::lib_dir_for(binary);
        let mut args: Vec<String> = vec!["-L".to_string(), "-s".to_string()];
        let profile = self.inner.cfg.auth.impcurl_impersonate.as_deref();
        if let Some(p) = profile {
            args.push("--impersonate".to_string());
            args.push(p.to_string());
        }
        if let Some(proxy_url) = &self.inner.cfg.proxy {
            args.push("-x".to_string());
            args.push(proxy_url.clone());
        }
        let body_str =
            String::from_utf8(form_body).context("postback: form body is not valid UTF-8")?;
        args.extend_from_slice(&[
            "-X".to_string(),
            "POST".to_string(),
            "-H".to_string(),
            "Content-Type: application/x-www-form-urlencoded".to_string(),
            "-H".to_string(),
            format!("Referer: {url}"),
            "--data".to_string(),
            body_str,
            url.to_string(),
        ]);
        let output = impcurl::run(binary, lib_dir.as_ref(), &args).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!(
                "postback: curl-impersonate exited with {}: {}",
                output.status,
                stderr.trim()
            ))
        }
    }

    pub async fn preflight(&self) -> anyhow::Result<()> {
        if self.inner.cfg.auth.skip_preflight {
            return Ok(());
        }

        self.fetch_text(&self.inner.cfg.auth.referer)
            .await
            .context("preflight request failed")?;
        Ok(())
    }

    async fn request_text(
        &self,
        method: Method,
        url: &str,
        body: Option<Vec<u8>>,
        referer: Option<&str>,
    ) -> anyhow::Result<String> {
        debug!(method = method.as_str(), url, "request_text dispatching");
        match &self.inner.backend {
            #[cfg(feature = "wreq-backend")]
            BackendClient::Wreq(client) => {
                self.request_text_via_wreq_client(client, method, url, body, referer)
                    .await
            }
            BackendClient::Reqwest(client) => {
                self.request_text_via_reqwest_client(client, method, url, body, referer)
                    .await
            }
            #[cfg(feature = "hyper-backend")]
            BackendClient::HyperRustls(client) => {
                let headers = self.headers()?;
                hyper_exec::execute_hyper_request(
                    client,
                    &self.inner.cfg,
                    method,
                    url,
                    body,
                    referer,
                    headers,
                    None,
                )
                .await
            }
            #[cfg(all(feature = "hyper-backend", feature = "native-tls"))]
            BackendClient::HyperNativeTls(client) => {
                let headers = self.headers()?;
                hyper_exec::execute_hyper_request(
                    client,
                    &self.inner.cfg,
                    method,
                    url,
                    body,
                    referer,
                    headers,
                    None,
                )
                .await
            }
            BackendClient::Impcurl { binary, library } => {
                let method_for_process = method.clone();
                let body_for_process = body.clone();
                let referer_for_process = referer.map(|v| v.to_string());

                match self
                    .request_text_via_reqwest_client(library, method, url, body, referer)
                    .await
                {
                    Ok(text) => {
                        if Self::looks_like_waf_challenge(&text)
                            && let Some(binary_path) = binary.as_deref()
                        {
                            warn!(
                                backend = "impcurl",
                                "detected bot-challenge HTML from in-process client, forcing process fallback"
                            );
                            return self
                                    .request_text_via_impcurl_process(
                                        binary_path,
                                        method_for_process,
                                        url,
                                        body_for_process,
                                        referer_for_process.as_deref(),
                                    )
                                    .await
                                    .context("impcurl process fallback failed after challenge-page detection");
                        }
                        Ok(text)
                    }
                    Err(library_err) => {
                        if let Some(binary_path) = binary.as_deref() {
                            if !impcurl::IMPCURL_PROCESS_FALLBACK_WARNED
                                .swap(true, Ordering::Relaxed)
                            {
                                warn!(
                                    backend = "impcurl",
                                    "in-process request failed, enabling process fallback"
                                );
                            }

                            self.request_text_via_impcurl_process(
                                binary_path,
                                method_for_process,
                                url,
                                body_for_process,
                                referer_for_process.as_deref(),
                            )
                            .await
                            .with_context(|| {
                                format!(
                                    "impcurl process fallback failed after in-process error: {library_err}"
                                )
                            })
                        } else {
                            Err(library_err).with_context(|| {
                                "in-process impcurl request failed and no curl-impersonate binary was found for fallback"
                            })
                        }
                    }
                }
            }
        }
    }

    pub async fn fetch_text(&self, url: &str) -> anyhow::Result<String> {
        let max_retry = self.inner.cfg.retry.count;
        let mut attempt: u32 = 0;
        let mut backoff_ms = self.inner.cfg.retry.backoff_ms;

        loop {
            debug!(url, attempt, "GET request");
            match self.request_text(Method::GET, url, None, None).await {
                Ok(text) => {
                    if Self::looks_like_waf_challenge(&text) {
                        if attempt < max_retry {
                            attempt += 1;
                            warn!(
                                url,
                                attempt,
                                backoff_ms,
                                class = retry::classify_failure(None, true),
                                "GET returned WAF challenge page, retrying"
                            );
                            sleep(Duration::from_millis(backoff_ms)).await;
                            backoff_ms = retry::next_backoff_ms(backoff_ms, Some(429), true);
                            continue;
                        }

                        return Err(anyhow::anyhow!(retry::failure_hint(url, None, true)));
                    }

                    debug!(url, bytes = text.len(), "GET succeeded");
                    return Ok(text);
                }
                Err(err) => {
                    let status = retry::extract_status(&err);

                    if attempt < max_retry && retry::should_retry(status) {
                        attempt += 1;
                        warn!(
                            url,
                            ?status,
                            attempt,
                            backoff_ms,
                            class = retry::classify_failure(status, false),
                            "GET failed, retrying"
                        );
                        sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = retry::next_backoff_ms(backoff_ms, status, false);
                        continue;
                    }
                    warn!(
                        url,
                        ?status,
                        class = retry::classify_failure(status, false),
                        "GET failed, giving up"
                    );
                    return Err(err).with_context(|| retry::failure_hint(url, status, false));
                }
            }
        }
    }

    pub async fn get_json(&self, url: &str) -> anyhow::Result<Value> {
        let body = self.fetch_text(url).await?;
        serde_json::from_str(&body).with_context(|| format!("invalid JSON response for {url}"))
    }

    pub async fn post_json(&self, url: &str, payload: &Value) -> anyhow::Result<Value> {
        self.post_json_with_referer(url, &self.inner.cfg.auth.referer, payload)
            .await
    }

    pub async fn post_json_with_referer(
        &self,
        url: &str,
        referer: &str,
        payload: &Value,
    ) -> anyhow::Result<Value> {
        let max_retry = self.inner.cfg.retry.count;
        let mut attempt: u32 = 0;
        let mut backoff_ms = self.inner.cfg.retry.backoff_ms;
        let payload_bytes =
            serde_json::to_vec(payload).context("failed to serialize request payload")?;

        loop {
            debug!(url, referer, attempt, "POST request");
            match self
                .request_text(
                    Method::POST,
                    url,
                    Some(payload_bytes.clone()),
                    Some(referer),
                )
                .await
            {
                Ok(text) => {
                    if Self::looks_like_waf_challenge(&text) {
                        if attempt < max_retry {
                            attempt += 1;
                            warn!(
                                url,
                                ?referer,
                                attempt,
                                backoff_ms,
                                class = retry::classify_failure(None, true),
                                "POST returned WAF challenge page, retrying"
                            );
                            sleep(Duration::from_millis(backoff_ms)).await;
                            backoff_ms = retry::next_backoff_ms(backoff_ms, Some(429), true);
                            continue;
                        }

                        return Err(anyhow::anyhow!(retry::failure_hint(url, None, true)));
                    }

                    debug!(url, bytes = text.len(), "POST succeeded");
                    return serde_json::from_str::<Value>(&text)
                        .with_context(|| format!("invalid JSON response for {url}"));
                }
                Err(err) => {
                    let status = retry::extract_status(&err);

                    if attempt < max_retry && retry::should_retry(status) {
                        attempt += 1;
                        warn!(
                            url,
                            ?status,
                            attempt,
                            backoff_ms,
                            class = retry::classify_failure(status, false),
                            "POST failed, retrying"
                        );
                        sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = retry::next_backoff_ms(backoff_ms, status, false);
                        continue;
                    }
                    warn!(
                        url,
                        ?status,
                        class = retry::classify_failure(status, false),
                        "POST failed, giving up"
                    );
                    return Err(err).with_context(|| retry::failure_hint(url, status, false));
                }
            }
        }
    }

    /// Perform an ASP.NET WebForms–style postback (or any HTML form POST).
    ///
    /// # Workflow
    ///
    /// 1. `GET url` — fetches the page HTML using the configured network backend.
    /// 2. Extract all `<input type="hidden">` fields from the response (captures
    ///    `__VIEWSTATE`, `__VIEWSTATEGENERATOR`, `__EVENTVALIDATION`, and any other
    ///    hidden fields present in the form).
    /// 3. Merge `extra_fields` — values provided here override same-named fields
    ///    extracted from the page; new keys are appended.
    /// 4. `POST url` with the merged fields as `application/x-www-form-urlencoded`.
    ///
    /// # Example
    /// ```no_run
    /// # use http_client::{NetworkAuthConfig, NetworkClient, NetworkClientConfig};
    /// # use http_config::{HttpBackend, RetryConfig, TlsConfig};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let cfg = NetworkClientConfig {
    ///     timeout_secs: 25,
    ///     base_url: "https://example.com".to_string(),
    ///     backend: HttpBackend::Reqwest,
    ///     retry: RetryConfig::default(),
    ///     tls: TlsConfig::default(),
    ///     auth: NetworkAuthConfig {
    ///         token: None,
    ///         cookies_file: None,
    ///         user_agent: "curl/8.5.0".to_string(),
    ///         referer: "https://example.com".to_string(),
    ///         impcurl_impersonate: None,
    ///         skip_preflight: false,
    ///     },
    ///     proxy: None,
    /// };
    /// let client = NetworkClient::from_config(cfg)?;
    /// let html = client
    ///     .fetch_postback(
    ///         "https://example.com/Default.aspx",
    ///         &[("__EVENTTARGET".into(), "ctl00$btnSearch".into())],
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_postback(
        &self,
        url: &str,
        extra_fields: &[(String, String)],
    ) -> anyhow::Result<String> {
        // Step 1: GET the page.
        let html = self
            .fetch_text(url)
            .await
            .with_context(|| format!("postback: failed to GET {url}"))?;

        // Step 2: Extract hidden form fields.
        let mut fields = extract_hidden_fields(&html);

        // Step 3: Merge caller-supplied overrides.
        for (key, val) in extra_fields {
            if let Some(existing) = fields.iter_mut().find(|(k, _)| k == key) {
                existing.1 = val.clone();
            } else {
                fields.push((key.clone(), val.clone()));
            }
        }

        // Step 4: POST the form.
        let referer = Some(url);
        let form_body = form_urlencode(&fields);
        let headers = self.headers()?;

        match &self.inner.backend {
            #[cfg(feature = "wreq-backend")]
            BackendClient::Wreq(client) => {
                self.postback_via_wreq_client(client, url, &headers, &fields)
                    .await
            }
            BackendClient::Reqwest(client) => {
                self.postback_via_reqwest_client(client, url, &headers, &fields)
                    .await
            }
            BackendClient::HyperRustls(client) => {
                hyper_exec::execute_hyper_request(
                    client,
                    &self.inner.cfg,
                    Method::POST,
                    url,
                    Some(form_body),
                    referer,
                    headers,
                    Some("application/x-www-form-urlencoded"),
                )
                .await
            }
            #[cfg(feature = "native-tls")]
            BackendClient::HyperNativeTls(client) => {
                hyper_exec::execute_hyper_request(
                    client,
                    &self.inner.cfg,
                    Method::POST,
                    url,
                    Some(form_body),
                    referer,
                    headers,
                    Some("application/x-www-form-urlencoded"),
                )
                .await
            }
            BackendClient::Impcurl { binary, library } => {
                let form_body_for_process = form_body.clone();
                match self
                    .postback_via_reqwest_client(library, url, &headers, &fields)
                    .await
                {
                    Ok(text) => Ok(text),
                    Err(library_err) => {
                        if let Some(binary_path) = binary.as_deref() {
                            if !impcurl::IMPCURL_PROCESS_FALLBACK_WARNED
                                .swap(true, Ordering::Relaxed)
                            {
                                warn!(
                                    backend = "impcurl",
                                    "in-process postback failed, enabling process fallback"
                                );
                            }
                            self.postback_via_impcurl_process(binary_path, url, form_body_for_process)
                                .await
                                .with_context(|| {
                                    format!(
                                        "impcurl postback process fallback failed after in-process error: {library_err}"
                                    )
                                })
                        } else {
                            Err(library_err).with_context(|| {
                                "in-process impcurl postback failed and no curl-impersonate binary was found for fallback"
                            })
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
