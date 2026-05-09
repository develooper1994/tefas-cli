use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use serde_json::Value;
use tokio::sync::Semaphore;

use crate::{NetworkClient, parse_document};

// ── Fundpage ──────────────────────────────────────────────────────────────────

/// A single fund-page fetch+parse job.
pub struct FundpageJob {
    pub code: String,
    /// If set, raw HTML is written to this path after fetching.
    pub html_save_dest: Option<PathBuf>,
}

/// Fetch and parse fund pages in bounded-parallel fashion.
///
/// Returns one `(code, Result<Value>)` per input job, in the **original order** of `jobs`.
pub async fn run_fundpage_batch(
    client: &NetworkClient,
    base_url: &str,
    jobs: Vec<FundpageJob>,
    concurrency: usize,
    quiet: bool,
) -> Vec<(String, anyhow::Result<Value>)> {
    let permits = Arc::new(Semaphore::new(concurrency.max(1)));

    let handles: Vec<_> = jobs
        .into_iter()
        .map(|job| {
            let client = client.clone();
            let base = base_url.to_string();
            let permits = permits.clone();
            tokio::spawn(async move {
                let code = job.code.clone();
                let result: anyhow::Result<Value> = async {
                    let _permit = permits
                        .acquire_owned()
                        .await
                        .map_err(|e| anyhow::anyhow!("semaphore closed: {e}"))?;
                    let url = format!("{}/tr/fon-detayli-analiz/{}", base, code);
                    if !quiet {
                        eprintln!("Fetching fund {}...", code);
                    }
                    let html = client.fetch_text(&url).await?;
                    if let Some(dest) = job.html_save_dest {
                        if let Some(p) = dest.parent() {
                            let _ = std::fs::create_dir_all(p);
                        }
                        let _ = std::fs::write(&dest, &html);
                    }
                    let grouped = tokio::task::spawn_blocking(move || {
                        let (grouped, _) = parse_document(&html);
                        grouped
                    })
                    .await
                    .context("parse task join error")?;
                    Ok(grouped)
                }
                .await;
                (code, result)
            })
        })
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for h in handles {
        match h.await {
            Ok((code, result)) => results.push((code, result)),
            Err(e) => {
                // Join errors have no associated code; surface as a nameless error entry
                results.push((
                    "(unknown)".to_string(),
                    Err(anyhow::anyhow!("task join error: {e}")),
                ));
            }
        }
    }
    results
}

// ── Query ─────────────────────────────────────────────────────────────────────

/// A single API query job, fully resolved from a CLI `Operation` or `OperationOld`.
pub struct QueryJob {
    /// Original index used to restore result order.
    pub idx: usize,
    /// Operation name as reported in the merged output key.
    pub name: String,
    /// Full absolute URL for the HTTP POST.
    pub url: String,
    /// Referer header value.
    pub referer: &'static str,
    /// Default payload for this operation (will be overridden by set_overrides / custom_payload).
    pub default_payload: Value,
}

/// Execute API query jobs in bounded-parallel fashion.
///
/// Returns `(idx, name, Value)` triples sorted by `idx`.
/// Propagates errors via `anyhow::Result` so callers can decide how to handle them.
pub async fn run_query_batch(
    client: &NetworkClient,
    jobs: Vec<QueryJob>,
    concurrency: usize,
    set_overrides: Vec<(String, Value)>,
    custom_payload: Option<Value>,
) -> anyhow::Result<Vec<(usize, String, Value)>> {
    let permits = Arc::new(Semaphore::new(concurrency.max(1)));
    let overrides = Arc::new(set_overrides);
    let custom = custom_payload.map(Arc::new);

    let handles: Vec<_> = jobs
        .into_iter()
        .map(|job| {
            let client = client.clone();
            let permits = permits.clone();
            let overrides = Arc::clone(&overrides);
            let custom = custom.clone();
            tokio::spawn(async move {
                let _permit = permits
                    .acquire_owned()
                    .await
                    .map_err(|e| anyhow::anyhow!("semaphore closed: {e}"))?;
                let mut payload = custom
                    .as_ref()
                    .map(|v| v.as_ref().clone())
                    .unwrap_or_else(|| job.default_payload);
                apply_set_overrides(&mut payload, overrides.as_ref());
                let res = client
                    .post_json_with_referer(&job.url, job.referer, &payload)
                    .await?;
                Ok::<_, anyhow::Error>((job.idx, job.name, res))
            })
        })
        .collect();

    let mut ordered = Vec::with_capacity(handles.len());
    for h in handles {
        let (idx, name, val) = h.await??;
        ordered.push((idx, name, val));
    }
    ordered.sort_by_key(|(idx, _, _)| *idx);
    Ok(ordered)
}

// ── Fetch ─────────────────────────────────────────────────────────────────────

/// Execute bounded-parallel URL fetches.
///
/// Returns one `(url, Result<html_body>)` per input URL, in original order.
pub async fn run_fetch_batch(
    client: &NetworkClient,
    urls: Vec<String>,
    concurrency: usize,
    quiet: bool,
) -> Vec<(String, anyhow::Result<String>)> {
    let permits = Arc::new(Semaphore::new(concurrency.max(1)));

    let handles: Vec<_> = urls
        .into_iter()
        .map(|url| {
            let client = client.clone();
            let permits = permits.clone();
            tokio::spawn(async move {
                let url_clone = url.clone();
                let result: anyhow::Result<String> = async {
                    let _permit = permits
                        .acquire_owned()
                        .await
                        .map_err(|e| anyhow::anyhow!("semaphore closed: {e}"))?;
                    if !quiet {
                        eprintln!("Fetching {}...", url);
                    }
                    client.fetch_text(&url).await
                }
                .await;
                (url_clone, result)
            })
        })
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for h in handles {
        match h.await {
            Ok((url, result)) => results.push((url, result)),
            Err(e) => results.push((
                "(unknown)".to_string(),
                Err(anyhow::anyhow!("task join error: {e}")),
            )),
        }
    }
    results
}

// ── Shared helpers ────────────────────────────────────────────────────────────

pub fn apply_set_overrides(payload: &mut Value, overrides: &[(String, Value)]) {
    if let Value::Object(obj) = payload {
        for (k, v) in overrides {
            obj.insert(k.clone(), v.clone());
        }
    }
}
