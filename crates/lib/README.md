# tefas library

This crate exposes TEFAS workflows as a reusable Rust library.

External projects can consume it with crate name `tefas`.

## Add dependency from another Rust project

If you use git dependency:

```toml
[dependencies]
tefas = { git = "https://github.com/<org>/<repo>.git", package = "lib" }
```

If you use path dependency:

```toml
[dependencies]
tefas = { path = "../tefas-cli/crates/lib", package = "lib" }
```

## Quick start

```rust
use tefas::{FundpageRunOptions, TefasClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = TefasClient::from_defaults()?;

    let results = client
        .collect_fundpages(
            vec!["AC5".to_string(), "TLY".to_string()],
            FundpageRunOptions {
                network_concurrency: Some(8),
                parse_concurrency: Some(8),
                quiet: true,
            },
        )
        .await;

    for (code, parsed) in results {
        println!("{} -> {}", code, parsed.is_ok());
    }

    Ok(())
}
```

## Useful high-level methods

- `TefasClient::from_defaults`
- `TefasClient::from_base_url`
- `TefasClient::collect_fundpages`
- `TefasClient::query_names`
- `TefasClient::query_one`
- `TefasClient::query_legacy_one`

## Example matrix

- Fundpage (offline fixture parse): `cargo run -q -p lib --example offline_parse_fixture`
- Query (bounded async parallel): `cargo run -q -p lib --example query_parallel_usage`
- Fetch (bounded async parallel): `cargo run -q -p lib --example fetch_parallel_usage`
- Combined endpoint workflow: `cargo run -q -p lib --example endpoint_parallel_usage`

## Versioning and releases

- SemVer policy: [SEMVER_POLICY.md](SEMVER_POLICY.md)
- Changelog: [CHANGELOG.md](CHANGELOG.md)

## Endpoint workflows (bounded async parallel)

`query` and `fetch` workflows run with bounded async concurrency.

```rust
use tefas::TefasClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = TefasClient::from_defaults()?;

    // Multiple endpoints in one batch (bounded async parallel)
    let merged = client
        .query_names(
            vec![
                "fonBilgiGetir".to_string(),
                "fonGetiriBazliBilgiGetir".to_string(),
            ],
            Some(8),
            vec![],
            None,
        )
        .await?;
    println!("{}", merged);

    // Raw endpoint fetch in bounded async parallel
    let fetched = client
        .fetch_urls(
            tefas::FetchBatchRequest::new(
                vec![
                    "https://www.tefas.gov.tr".to_string(),
                    "https://www.takasbank.com.tr".to_string(),
                ],
                Some(8),
                8,
            ),
            true,
        )
        .await;
    println!("fetched urls: {}", fetched.len());

    Ok(())
}
```

## Tested offline example (no network)

This repository includes an offline library example that parses a local fixture.

```bash
cargo run -q -p lib --example offline_parse_fixture
```

Expected output contains `code=AC5` when fixture parsing succeeds.

The same flow is covered by test:

```bash
cargo test -p lib library_example_parses_local_fixture -- --nocapture
```
