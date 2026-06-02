//! Retry policy helpers for the TEFAS HTTP client.

use crate::StatusError;

/// Returns `true` when a request should be retried based on its HTTP status.
///
/// Retries on:
/// - No status available (network-level error, `None`).
/// - `408 Request Timeout` and `429 Too Many Requests`.
/// - Any `5xx` server-error response.
pub(super) fn should_retry(status: Option<u16>) -> bool {
    match status {
        Some(code) => code == 408 || code == 429 || code >= 500,
        None => true,
    }
}

pub(super) fn next_backoff_ms(
    current_backoff_ms: u64,
    status: Option<u16>,
    waf_challenge: bool,
) -> u64 {
    let base = current_backoff_ms.max(1);
    // Rate limits and WAF challenges typically need a stronger slowdown than generic retries.
    let factor = if waf_challenge || status == Some(429) {
        3
    } else {
        2
    };
    base.saturating_mul(factor)
}

pub(super) fn classify_failure(status: Option<u16>, waf_challenge: bool) -> &'static str {
    if waf_challenge {
        "waf_challenge"
    } else {
        match status {
            Some(429) => "rate_limited",
            Some(408) => "timeout",
            Some(code) if code >= 500 => "server_error",
            Some(401 | 403) => "blocked_or_unauthorized",
            Some(400..=499) => "client_error",
            Some(_) => "http_error",
            None => "network_error",
        }
    }
}

pub(super) fn failure_hint(url: &str, status: Option<u16>, waf_challenge: bool) -> String {
    let class = classify_failure(status, waf_challenge);
    match class {
        "waf_challenge" => format!(
            "request blocked by WAF challenge for {} (e.g. Request Rejected/TSPD). Try lower network concurrency, stronger impersonation profile, and higher retry backoff",
            url
        ),
        "rate_limited" => format!(
            "request rate-limited for {} (HTTP 429). Try lower network concurrency and increase retry backoff",
            url
        ),
        "timeout" => format!(
            "request timed out for {} (HTTP 408). Retry with higher timeout or lower concurrency",
            url
        ),
        "server_error" => format!("server-side transient error for {} (HTTP 5xx)", url),
        "blocked_or_unauthorized" => format!(
            "request rejected or unauthorized for {} (HTTP {}). Check auth headers, referer, and WAF constraints",
            url,
            status.unwrap_or_default()
        ),
        "client_error" => format!(
            "request failed for {} (HTTP {}). Check payload and endpoint contract",
            url,
            status.unwrap_or_default()
        ),
        _ => format!("request failed for {}", url),
    }
}

/// Extract the HTTP status code from an `anyhow::Error` wrapping either a
/// [`StatusError`] or a `reqwest::Error`.
#[allow(dead_code)]
pub(super) fn extract_status(err: &anyhow::Error) -> Option<u16> {
    err.downcast_ref::<StatusError>()
        .map(|e| e.status)
        .or_else(|| {
            err.downcast_ref::<reqwest::Error>()
                .and_then(|e| e.status().map(|s| s.as_u16()))
        })
}

#[cfg(test)]
mod tests {
    use super::{classify_failure, failure_hint, next_backoff_ms};

    #[test]
    fn next_backoff_stronger_for_rate_limit_and_waf() {
        assert_eq!(next_backoff_ms(400, Some(429), false), 1200);
        assert_eq!(next_backoff_ms(400, None, true), 1200);
        assert_eq!(next_backoff_ms(400, Some(500), false), 800);
    }

    #[test]
    fn classify_failure_variants() {
        assert_eq!(classify_failure(Some(429), false), "rate_limited");
        assert_eq!(
            classify_failure(Some(403), false),
            "blocked_or_unauthorized"
        );
        assert_eq!(classify_failure(None, false), "network_error");
        assert_eq!(classify_failure(None, true), "waf_challenge");
    }

    #[test]
    fn failure_hint_mentions_waf_and_429() {
        let waf = failure_hint("https://example.invalid", None, true);
        assert!(waf.contains("WAF"));
        let limited = failure_hint("https://example.invalid", Some(429), false);
        assert!(limited.contains("HTTP 429"));
    }
}
