//! Shared hyper request executor with redirect following.
//!
//! Generic over the hyper connector so that rustls, native-tls, and boring-tls
//! variants all reuse the same redirect loop.

use anyhow::Context;
use bytes::Bytes;
use http::Method;
use http::header::{HeaderMap, HeaderName, HeaderValue, ORIGIN, REFERER};
use http_body_util::{BodyExt, Full};
use hyper::header::{CONTENT_TYPE, HOST, LOCATION};
use hyper::http::Uri;
use hyper_util::client::legacy::Client as HyperClient;
use std::time::Duration;
use tokio::time::timeout;

use crate::{NetworkClientConfig, StatusError};

const MAX_REDIRECTS: usize = 10;

/// Execute a single request (plus redirect loop) through any hyper connector.
///
/// Extracts the shared redirect-following logic so that all typed hyper connector
/// variants (HyperRustls, HyperNativeTls, and the optional HyperBoring) can reuse
/// it without code duplication.
///
/// `C` must implement [`hyper_util::client::legacy::connect::Connect`], which is
/// satisfied by all connectors produced by `hyper-rustls`, `hyper-tls`, and
/// `hyper-boring`.
#[allow(clippy::too_many_arguments)] // request shape mirrors HTTP semantics
pub(crate) async fn execute_hyper_request<C>(
    client: &HyperClient<C, Full<Bytes>>,
    cfg: &NetworkClientConfig,
    method: Method,
    url: &str,
    body: Option<Vec<u8>>,
    referer: Option<&str>,
    base_headers: HeaderMap,
    // content_type: if Some, overrides the POST body Content-Type
    // (None → "application/json; charset=UTF-8").
    content_type: Option<&str>,
) -> anyhow::Result<String>
where
    C: hyper_util::client::legacy::connect::Connect + Clone + Send + Sync + 'static,
{
    // hyper_util's legacy Client does not follow redirects; we implement the
    // standard redirect loop here (up to MAX_REDIRECTS hops).
    let mut current_url = url.to_string();
    let mut current_method = method.clone();
    let mut current_body = body.clone();
    let mut redirects = 0usize;

    loop {
        let mut req_builder = hyper::Request::builder()
            .method(current_method.clone())
            .uri(current_url.as_str());

        let mut headers = base_headers.clone();
        if let Some(rf) = referer {
            headers.insert(
                REFERER,
                HeaderValue::from_str(rf).context("invalid referer header")?,
            );
        }
        if current_body.is_some() {
            let ct = content_type.unwrap_or("application/json; charset=UTF-8");
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_str(ct).context("invalid content-type header")?,
            );
            headers.insert(
                ORIGIN,
                HeaderValue::from_str(&cfg.normalized_base_url())
                    .context("invalid origin header")?,
            );
            headers.insert(
                HeaderName::from_static("x-requested-with"),
                HeaderValue::from_static("XMLHttpRequest"),
            );
        }

        let req_headers = req_builder
            .headers_mut()
            .ok_or_else(|| anyhow::anyhow!("failed to build request headers"))?;

        // Explicit Host header from URL authority
        let uri: Uri = current_url.parse().context("invalid URL")?;
        if let Some(authority) = uri.authority() {
            req_headers.insert(
                HOST,
                HeaderValue::from_str(authority.as_str()).context("invalid host header")?,
            );
        }

        for (name, value) in &headers {
            req_headers.insert(name, value.clone());
        }

        let request = req_builder
            .body(Full::new(Bytes::from(
                current_body.clone().unwrap_or_default(),
            )))
            .context("failed to build hyper request")?;

        let send_fut = client.request(request);
        let response = if let Some(timeout_ms) = cfg.tls.timeout_ms {
            timeout(Duration::from_millis(timeout_ms), send_fut)
                .await
                .context("request timeout")??
        } else {
            send_fut.await?
        };

        let status = response.status();

        // Follow 3xx redirects manually
        if status.is_redirection() {
            if redirects >= MAX_REDIRECTS {
                return Err(anyhow::anyhow!(
                    "too many redirects (>{}) for {}",
                    MAX_REDIRECTS,
                    url
                ));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .ok_or_else(|| anyhow::anyhow!("redirect {} has no Location header", status))?
                .to_str()
                .context("Location header is not valid UTF-8")?
                .to_string();

            // Resolve relative locations against current URL
            current_url = if location.starts_with("http://") || location.starts_with("https://") {
                location
            } else {
                let base: Uri = current_url.parse().context("invalid base URL")?;
                let scheme = base.scheme_str().unwrap_or("https");
                let authority = base
                    .authority()
                    .ok_or_else(|| anyhow::anyhow!("base URL has no authority"))?
                    .as_str();
                format!(
                    "{}://{}{}",
                    scheme,
                    authority,
                    if location.starts_with('/') {
                        location.clone()
                    } else {
                        format!("/{}", location)
                    }
                )
            };

            // 301/302/303: switch to GET, drop body (standard browser behaviour)
            if status == hyper::StatusCode::MOVED_PERMANENTLY
                || status == hyper::StatusCode::FOUND
                || status == hyper::StatusCode::SEE_OTHER
            {
                current_method = Method::GET;
                current_body = None;
            }

            redirects += 1;
            continue;
        }

        let bytes = response
            .into_body()
            .collect()
            .await
            .context("failed collecting response body")?
            .to_bytes();
        let text = String::from_utf8_lossy(&bytes).to_string();
        return if status.is_success() {
            Ok(text)
        } else {
            Err(anyhow::Error::new(StatusError {
                status: status.as_u16(),
                url: url.to_string(),
            }))
        };
    }
}
