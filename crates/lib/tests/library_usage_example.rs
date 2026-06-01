use std::fs;
use std::path::{Path, PathBuf};

use tefas::parse_document;

fn resolve_fixture_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../../datasets/fundpage/html/AC5.html"),
        manifest.join("../../../datasets/fundpage/html/AC5.html"),
    ];

    for path in candidates {
        if path.exists() {
            return path;
        }
    }

    panic!("fixture not found: datasets/fundpage/html/AC5.html");
}

#[test]
fn library_example_parses_local_fixture() {
    let fixture = resolve_fixture_path();
    let html = fs::read_to_string(&fixture)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", fixture.display(), e));

    assert!(
        !html.contains("Request Rejected"),
        "fixture should not contain WAF rejection: {}",
        fixture.display()
    );

    let (grouped, _meta) = parse_document(&html);

    let code = grouped
        .pointer("/profile/fon_kodu")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    assert_eq!(code, "AC5", "unexpected fund code in parsed fixture");
}
