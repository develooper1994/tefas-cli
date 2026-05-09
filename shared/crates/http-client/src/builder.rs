//! HTTP client construction helpers.
//!
//! Each function builds one concrete backend client from [`crate::NetworkClientConfig`].
//! They are free functions (not methods) so they can be tested and reused independently
//! of [`crate::NetworkClient`].

use std::time::Duration;

use anyhow::Context;
use http_config::TlsBackend;
use tracing::warn;

use crate::NetworkClientConfig;

#[cfg(feature = "hyper-backend")]
use bytes::Bytes;
#[cfg(feature = "hyper-backend")]
use http_body_util::Full;
#[cfg(feature = "hyper-backend")]
use hyper_rustls::HttpsConnectorBuilder;
#[cfg(feature = "hyper-backend")]
pub(super) use hyper_util::client::legacy::Client as HyperClient;
#[cfg(feature = "hyper-backend")]
use hyper_util::client::legacy::connect::HttpConnector;
#[cfg(feature = "hyper-backend")]
use hyper_util::rt::TokioExecutor;

#[cfg(feature = "hyper-backend")]
pub(super) type HyperRustlsConnector = hyper_rustls::HttpsConnector<HttpConnector>;
#[cfg(all(feature = "hyper-backend", feature = "native-tls"))]
pub(super) type HyperNativeTlsConnector = hyper_tls::HttpsConnector<HttpConnector>;
#[cfg(feature = "hyper-backend")]
pub(super) type HyperBody = Full<Bytes>;

/// Build a `reqwest` client that wraps an in-process `curl-impersonate` library.
///
/// Falls back to rustls on failure.
pub(super) fn build_impcurl_library_client(
    cfg: &NetworkClientConfig,
) -> anyhow::Result<reqwest::Client> {
    let preferred_tls = cfg.tls.backend;
    match build_reqwest(cfg, preferred_tls) {
        Ok(client) => Ok(client),
        Err(primary_err) => {
            warn!(
                backend = "impcurl",
                tls = ?preferred_tls,
                "in-process client build failed, falling back to rustls"
            );
            build_reqwest(cfg, TlsBackend::Rustls).with_context(|| {
                format!("failed to build in-process impcurl client (primary error: {primary_err})")
            })
        }
    }
}

/// Build a `reqwest` client for the given TLS backend.
pub(super) fn build_reqwest(
    cfg: &NetworkClientConfig,
    tls: TlsBackend,
) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder().timeout(Duration::from_secs(cfg.timeout_secs));

    builder = match tls {
        TlsBackend::Rustls => builder.use_rustls_tls(),
        TlsBackend::NativeTls => {
            #[cfg(feature = "native-tls")]
            {
                builder.use_native_tls()
            }
            #[cfg(not(feature = "native-tls"))]
            {
                return Err(anyhow::anyhow!(
                    "TlsBackend::NativeTls requires the `native-tls` cargo feature; rebuild with `--features native-tls`"
                ));
            }
        }
        TlsBackend::Insecure => builder.use_rustls_tls().danger_accept_invalid_certs(true),
    };

    if let Some(timeout_ms) = cfg.tls.timeout_ms {
        builder = builder.connect_timeout(Duration::from_millis(timeout_ms));
    }

    if let Some(proxy_url) = &cfg.proxy {
        let proxy = reqwest::Proxy::all(proxy_url.as_str()).context("invalid proxy URL")?;
        builder = builder.proxy(proxy);
    }

    builder
        .build()
        .context("failed to build reqwest HTTP client")
}

/// Build a `wreq` client (BoringSSL built-in; TLS backend flag is ignored).
#[cfg(feature = "wreq-backend")]
pub(super) fn build_wreq(cfg: &NetworkClientConfig) -> anyhow::Result<wreq::Client> {
    let mut builder = wreq::Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .cookie_store(true);
    if let Some(timeout_ms) = cfg.tls.timeout_ms {
        builder = builder.connect_timeout(Duration::from_millis(timeout_ms));
    }
    if let Some(proxy_url) = &cfg.proxy {
        let proxy = wreq::Proxy::all(proxy_url.as_str()).context("invalid proxy URL")?;
        builder = builder.proxy(proxy);
    }
    builder.build().context("failed to build wreq HTTP client")
}

/// Build a hyper client backed by rustls + ring.
#[cfg(feature = "hyper-backend")]
pub(super) fn build_hyper_rustls(
    _cfg: &NetworkClientConfig,
) -> anyhow::Result<HyperClient<HyperRustlsConnector, HyperBody>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Prefer HTTP/1.1 (some WAFs react poorly to HTTP/2 ALPN fingerprints)
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .context("failed to load native root certificates")?
        .https_or_http()
        .enable_http1()
        .build();

    Ok(HyperClient::builder(TokioExecutor::new()).build(https))
}

/// Build a hyper client backed by the platform native TLS stack.
///
/// Uses OpenSSL on Linux, Schannel on Windows, SecureTransport on macOS.
/// On Linux, requires `libssl-dev` (Debian/Ubuntu) or `openssl-devel` (Fedora) at
/// **build** time.
#[cfg(all(feature = "hyper-backend", feature = "native-tls"))]
pub(super) fn build_hyper_native_tls(
    _cfg: &NetworkClientConfig,
) -> anyhow::Result<HyperClient<HyperNativeTlsConnector, HyperBody>> {
    let https = hyper_tls::HttpsConnector::new();
    Ok(HyperClient::builder(TokioExecutor::new()).build(https))
}
