use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Context;
use serde_json::Value;
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;

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
    fetch_concurrency: usize,
    parse_concurrency: usize,
    quiet: bool,
) -> Vec<(String, anyhow::Result<Value>)> {
    let fetch_window = fetch_concurrency.max(1);
    let fetch_permits = Arc::new(Semaphore::new(fetch_window));
    let parse_workers = parse_concurrency.max(1);
    let parse_queue_capacity = parse_workers.saturating_mul(2).max(1);
    let len = jobs.len();
    let mut ordered_results: Vec<Option<(String, anyhow::Result<Value>)>> =
        std::iter::repeat_with(|| None).take(len).collect();

    let mut fetch_set = JoinSet::new();
    let mut parse_worker_threads = Vec::with_capacity(parse_workers);

    let (parse_tx, parse_rx) = mpsc::channel::<(usize, String, String)>(parse_queue_capacity);
    let parse_rx = Arc::new(Mutex::new(parse_rx));
    let (parse_result_tx, mut parse_result_rx) =
        mpsc::channel::<(usize, String, anyhow::Result<Value>)>(parse_queue_capacity);

    for _ in 0..parse_workers {
        let parse_rx = parse_rx.clone();
        let parse_result_tx = parse_result_tx.clone();
        parse_worker_threads.push(thread::spawn(move || {
            loop {
                let next = {
                    let mut guard = match parse_rx.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    guard.blocking_recv()
                };
                let Some((idx, code, html)) = next else {
                    break;
                };

                let parsed = std::panic::catch_unwind(|| {
                    let (grouped, _) = parse_document(&html);
                    grouped
                })
                .map_err(|_| anyhow::anyhow!("parse worker panicked"));

                if parse_result_tx.blocking_send((idx, code, parsed)).is_err() {
                    break;
                }
            }
        }));
    }
    drop(parse_result_tx);

    let mut parse_jobs = 0usize;

    let mut pending_fetch_jobs = jobs.into_iter().enumerate();

    for _ in 0..fetch_window {
        let Some((idx, job)) = pending_fetch_jobs.next() else {
            break;
        };
        spawn_fundpage_fetch_task(
            &mut fetch_set,
            client,
            base_url,
            fetch_permits.clone(),
            idx,
            job,
            quiet,
        );
    }

    while let Some(joined) = fetch_set.join_next().await {
        match joined {
            Ok((idx, code, Ok(html))) => {
                match parse_tx.send((idx, code.clone(), html)).await {
                    Ok(()) => {
                        parse_jobs += 1;
                    }
                    Err(_e) => {
                        ordered_results[idx] = Some((
                            code,
                            Err(anyhow::anyhow!(
                                "parse queue closed before accepting fetched document"
                            )),
                        ));
                    }
                }
            }
            Ok((idx, code, Err(e))) => {
                ordered_results[idx] = Some((code, Err(e)));
            }
            Err(e) => {
                if !quiet {
                    eprintln!("Fetch task join error: {e}");
                }
            }
        }

        if let Some((idx, job)) = pending_fetch_jobs.next() {
            spawn_fundpage_fetch_task(
                &mut fetch_set,
                client,
                base_url,
                fetch_permits.clone(),
                idx,
                job,
                quiet,
            );
        }
    }
    drop(parse_tx);

    for _ in 0..parse_jobs {
        if let Some((idx, code, parsed)) = parse_result_rx.recv().await {
            ordered_results[idx] = Some((code, parsed));
        } else {
            break;
        }
    }

    for join_handle in parse_worker_threads {
        if join_handle.join().is_err() && !quiet {
            eprintln!("Parse worker thread panicked");
        }
    }

    ordered_results
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            item.unwrap_or_else(|| {
                (
                    format!("(unknown:{idx})"),
                    Err(anyhow::anyhow!("missing fetch/parse result")),
                )
            })
        })
        .collect()
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
    let submit_window = concurrency.max(1);
    let overrides = Arc::new(set_overrides);
    let custom = custom_payload.map(Arc::new);

    let mut set = JoinSet::new();
    let mut pending_jobs = jobs.into_iter();

    for _ in 0..submit_window {
        let Some(job) = pending_jobs.next() else {
            break;
        };
        spawn_query_task(&mut set, client, Arc::clone(&overrides), custom.clone(), job);
    }

    let mut ordered = Vec::new();
    while let Some(joined) = set.join_next().await {
        let (idx, name, val) = joined.context("query task join error")??;
        ordered.push((idx, name, val));
        if let Some(job) = pending_jobs.next() {
            spawn_query_task(&mut set, client, Arc::clone(&overrides), custom.clone(), job);
        }
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
    let submit_window = concurrency.max(1);
    let len = urls.len();
    let mut ordered_results: Vec<Option<(String, anyhow::Result<String>)>> =
        std::iter::repeat_with(|| None).take(len).collect();

    let mut set = JoinSet::new();
    let mut pending_urls = urls.into_iter().enumerate();

    for _ in 0..submit_window {
        let Some((idx, url)) = pending_urls.next() else {
            break;
        };
        spawn_fetch_task(&mut set, client, idx, url, quiet);
    }

    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((idx, url, result)) => {
                ordered_results[idx] = Some((url, result));
            }
            Err(e) => {
                if !quiet {
                    eprintln!("Fetch task join error: {e}");
                }
            }
        }

        if let Some((idx, url)) = pending_urls.next() {
            spawn_fetch_task(&mut set, client, idx, url, quiet);
        }
    }

    ordered_results
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            item.unwrap_or_else(|| {
                (
                    format!("(unknown:{idx})"),
                    Err(anyhow::anyhow!("missing fetch result")),
                )
            })
        })
        .collect()
}

// ── Shared helpers ────────────────────────────────────────────────────────────

pub fn apply_set_overrides(payload: &mut Value, overrides: &[(String, Value)]) {
    if let Value::Object(obj) = payload {
        for (k, v) in overrides {
            obj.insert(k.clone(), v.clone());
        }
    }
}

fn spawn_fundpage_fetch_task(
    fetch_set: &mut JoinSet<(usize, String, anyhow::Result<String>)>,
    client: &NetworkClient,
    base_url: &str,
    fetch_permits: Arc<Semaphore>,
    idx: usize,
    job: FundpageJob,
    quiet: bool,
) {
    let client = client.clone();
    let base = base_url.to_string();
    fetch_set.spawn(async move {
        let code = job.code;
        let fetch_result: anyhow::Result<String> = async {
            let _fetch_permit = fetch_permits
                .acquire_owned()
                .await
                .map_err(|e| anyhow::anyhow!("semaphore closed: {e}"))?;
            let url = format!("{}/tr/fon-detayli-analiz/{}", base, code);
            if !quiet {
                eprintln!("Fetching fund {}...", code);
            }
            let html = client.fetch_text(&url).await?;
            if let Some(dest) = job.html_save_dest {
                if let Some(p) = dest.parent()
                    && let Err(e) = std::fs::create_dir_all(p)
                {
                    eprintln!("Failed to create HTML output directory for {}: {}", code, e);
                }
                if let Err(e) = std::fs::write(&dest, &html) {
                    eprintln!("Failed to write HTML output for {}: {}", code, e);
                }
            }
            Ok(html)
        }
        .await;
        (idx, code, fetch_result)
    });
}

fn spawn_query_task(
    set: &mut JoinSet<anyhow::Result<(usize, String, Value)>>,
    client: &NetworkClient,
    overrides: Arc<Vec<(String, Value)>>,
    custom: Option<Arc<Value>>,
    job: QueryJob,
) {
    let client = client.clone();
    set.spawn(async move {
        let mut payload = custom
            .as_ref()
            .map(|v| v.as_ref().clone())
            .unwrap_or_else(|| job.default_payload);
        apply_set_overrides(&mut payload, overrides.as_ref());
        let res = client
            .post_json_with_referer(&job.url, job.referer, &payload)
            .await?;
        Ok::<_, anyhow::Error>((job.idx, job.name, res))
    });
}

fn spawn_fetch_task(
    set: &mut JoinSet<(usize, String, anyhow::Result<String>)>,
    client: &NetworkClient,
    idx: usize,
    url: String,
    quiet: bool,
) {
    let client = client.clone();
    set.spawn(async move {
        let result = if !quiet {
            eprintln!("Fetching {}...", url);
            client.fetch_text(&url).await
        } else {
            client.fetch_text(&url).await
        };
        (idx, url, result)
    });
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::apply_set_overrides;

    #[test]
    fn apply_set_overrides_updates_and_inserts() {
        let mut payload = json!({"a": 1, "b": "x"});
        let overrides = vec![
            ("a".to_string(), json!(99)),
            ("c".to_string(), json!(true)),
        ];

        apply_set_overrides(&mut payload, &overrides);

        assert_eq!(payload["a"], json!(99));
        assert_eq!(payload["b"], json!("x"));
        assert_eq!(payload["c"], json!(true));
    }

    #[test]
    fn apply_set_overrides_ignores_non_object_payload() {
        let mut payload = json!([1, 2, 3]);
        let before = payload.clone();
        let overrides = vec![("a".to_string(), json!(99))];

        apply_set_overrides(&mut payload, &overrides);

        assert_eq!(payload, before);
    }
}
