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
