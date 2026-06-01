use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tefas::parse_document;

fn resolve_fixture_path() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../../datasets/fundpage/html/AC5.html"),
        manifest.join("../../../datasets/fundpage/html/AC5.html"),
    ];

    for path in candidates {
        if path.exists() {
            return Ok(path);
        }
    }

    bail!("AC5 fixture not found under datasets/fundpage/html");
}

fn main() -> Result<()> {
    let fixture = resolve_fixture_path()?;
    let html = fs::read_to_string(&fixture)
        .with_context(|| format!("failed to read fixture: {}", fixture.display()))?;

    if html.contains("Request Rejected") {
        bail!("fixture contains WAF rejection page: {}", fixture.display());
    }

    let (grouped, _meta) = parse_document(&html);

    let code = grouped
        .pointer("/profile/fon_kodu")
        .and_then(|v| v.as_str())
        .context("missing /profile/fon_kodu")?;
    let name = grouped
        .pointer("/indicator/fon_adi")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    println!("parsed fixture {} => code={}, name={}", fixture.display(), code, name);
    Ok(())
}
