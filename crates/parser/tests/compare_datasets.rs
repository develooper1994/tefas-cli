use serde_json::Value;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tefas_parser::fund_page::parse_document;
use tefas_parser::{FundPage, TefasPage};

fn nearly_equal(a: f64, b: f64) -> bool {
    let diff = (a - b).abs();
    if diff <= 1e-6 {
        return true;
    }
    let largest = a.abs().max(b.abs()).max(1.0);
    diff / largest <= 1e-6
}

fn compare_json(expected: &Value, actual: &Value) -> Result<(), String> {
    match (expected, actual) {
        (Value::Object(eo), Value::Object(ao)) => {
            for (k, ev) in eo.iter() {
                if let Some(av) = ao.get(k) {
                    compare_json(ev, av).map_err(|s| format!("{} -> {}", k, s))?;
                } else {
                    return Err(format!("missing key '{}' in actual", k));
                }
            }
            Ok(())
        }
        (Value::Array(ea), Value::Array(aa)) => {
            if ea.len() != aa.len() {
                return Err(format!(
                    "array length mismatch {} != {}",
                    ea.len(),
                    aa.len()
                ));
            }
            for (i, (ev, av)) in ea.iter().zip(aa.iter()).enumerate() {
                compare_json(ev, av).map_err(|s| format!("[{}] {}", i, s))?;
            }
            Ok(())
        }
        (Value::Number(en), Value::Number(an)) => {
            let ef = en.as_f64().ok_or("expected not f64")?;
            let af = an.as_f64().ok_or("actual not f64")?;
            if nearly_equal(ef, af) {
                Ok(())
            } else {
                Err(format!("number mismatch {} != {}", ef, af))
            }
        }
        (Value::String(es), Value::String(asv)) => {
            if es.trim() == asv.trim() {
                Ok(())
            } else {
                Err(format!("string mismatch '{}' != '{}'", es, asv))
            }
        }
        (Value::Bool(eb), Value::Bool(ab)) => {
            if eb == ab {
                Ok(())
            } else {
                Err(format!("bool mismatch {} != {}", eb, ab))
            }
        }
        (Value::Null, Value::Null) => Ok(()),
        (e, a) => Err(format!("type mismatch {:?} vs {:?}", e, a)),
    }
}

fn strict_dataset_mode() -> bool {
    std::env::var("TEFAS_STRICT_DATASET_PARITY").ok().as_deref() == Some("1")
}

fn resolve_dataset_root(candidates: &[PathBuf], label: &str) -> Option<PathBuf> {
    if let Some(path) = candidates.iter().find_map(|p| p.canonicalize().ok()) {
        return Some(path);
    }

    let checked = candidates
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    if strict_dataset_mode() {
        panic!(
            "{}: strict modda dataset dizini bulunamadi. Kontrol edilen yollar: {}",
            label, checked
        );
    }
    eprintln!(
        "{}: skip - dataset dizini bulunamadi. Kontrol edilen yollar: {} (strict icin CI=true veya TEFAS_STRICT_DATASET_PARITY=1 kullan)",
        label, checked
    );
    None
}

fn validate_metadata_docs() -> bool {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let datasets_candidates = [
        manifest.join("../../datasets"),
        manifest.join("../../../datasets"),
        manifest.join("../../../../../datasets"),
        manifest.join("../../../../../tefas/datasets"),
    ];
    let Some(datasets_root) = resolve_dataset_root(&datasets_candidates, "validate_metadata_docs")
    else {
        return false;
    };

    let api_schema_path = datasets_root.join("api/metadata.schema.json");
    let api_meta_path = datasets_root.join("api/metadata.json");
    let fund_schema_path = datasets_root.join("fundpage/metadata.schema.json");
    let fund_meta_path = datasets_root.join("fundpage/metadata.json");

    assert!(
        api_schema_path.exists(),
        "missing api metadata schema: {}",
        api_schema_path.display()
    );
    assert!(
        api_meta_path.exists(),
        "missing api metadata document: {}",
        api_meta_path.display()
    );
    assert!(
        fund_schema_path.exists(),
        "missing fundpage metadata schema: {}",
        fund_schema_path.display()
    );
    assert!(
        fund_meta_path.exists(),
        "missing fundpage metadata document: {}",
        fund_meta_path.display()
    );

    let api_meta: Value = serde_json::from_str(
        &fs::read_to_string(&api_meta_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", api_meta_path.display(), e)),
    )
    .unwrap_or_else(|e| panic!("failed to parse {}: {}", api_meta_path.display(), e));
    let api_endpoints = api_meta
        .get("endpoints")
        .and_then(Value::as_array)
        .expect("api metadata must include 'endpoints' array");

    let api_expected_count = fs::read_dir(datasets_root.join("api"))
        .expect("failed to read shared datasets api")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| p.join("input.req.txt").exists() && p.join("output.body").exists())
        .count();

    assert_eq!(
        api_endpoints.len(),
        api_expected_count,
        "api metadata endpoint count mismatch"
    );

    let fund_meta: Value = serde_json::from_str(
        &fs::read_to_string(&fund_meta_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", fund_meta_path.display(), e)),
    )
    .unwrap_or_else(|e| panic!("failed to parse {}: {}", fund_meta_path.display(), e));
    let funds = fund_meta
        .get("funds")
        .and_then(Value::as_array)
        .expect("fundpage metadata must include 'funds' array");

    let fund_expected_count = fs::read_dir(datasets_root.join("fundpage/html"))
        .expect("failed to read shared datasets fundpage html")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("html"))
        .count();

    assert_eq!(
        funds.len(),
        fund_expected_count,
        "fundpage metadata fund count mismatch"
    );

    true
}

#[test]
fn validate_dataset_metadata_documents() {
    let _ = validate_metadata_docs();
}

#[test]
fn compare_datasets() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../../tests/datasets/raw"),
        manifest.join("../../../tefas/FundPage/datasets/raw"),
        manifest.join("../../../FundPage/datasets/raw"),
    ];

    let dir = match candidates.iter().find_map(|p| p.canonicalize().ok()) {
        Some(dir) => dir,
        None => {
            let checked = candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let strict_mode = std::env::var("TEFAS_STRICT_DATASET_PARITY").ok().as_deref() == Some("1");
            if strict_mode {
                panic!(
                    "compare_datasets: strict modda datasets/raw fixture dizini bulunamadi. Kontrol edilen yollar: {}",
                    checked
                );
            }
            eprintln!(
                "compare_datasets: skip - datasets/raw fixture dizini bulunamadi. Kontrol edilen yollar: {} (strict icin CI=true veya TEFAS_STRICT_DATASET_PARITY=1 kullan)",
                checked
            );
            return;
        }
    };

    let mut html_file_count = 0usize;
    let mut failures = Vec::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "compare_datasets: read_dir basarisiz ({}): {}",
            dir.display(),
            e
        )
    }) {
        let entry =
            entry.unwrap_or_else(|e| panic!("compare_datasets: dir entry okunamadi: {}", e));
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("html") {
            html_file_count += 1;
            let expected = path.with_file_name(format!(
                "{}_parsed.json",
                path.file_stem().unwrap().to_string_lossy()
            ));
            if !expected.exists() {
                eprintln!(
                    "Skipping {} (no expected {})",
                    path.display(),
                    expected.display()
                );
                continue;
            }
            let html = fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!(
                    "compare_datasets: html okunamadi ({}): {}",
                    path.display(),
                    e
                )
            });
            let (grouped, _flat) = parse_document(&html);
            let expected_v: Value =
                serde_json::from_str(&fs::read_to_string(&expected).unwrap_or_else(|e| {
                    panic!(
                        "compare_datasets: expected json okunamadi ({}): {}",
                        expected.display(),
                        e
                    )
                }))
                .unwrap_or_else(|e| {
                    panic!(
                        "compare_datasets: expected json parse edilemedi ({}): {}",
                        expected.display(),
                        e
                    )
                });
            match compare_json(&expected_v, &grouped) {
                Ok(()) => {
                    println!("OK: {}", path.display());
                }
                Err(e) => {
                    eprintln!("DIFF: {} -> {}", path.display(), e);
                    // print expected vs actual for debugging
                    if let Ok(s) = serde_json::to_string_pretty(&expected_v) {
                        eprintln!("EXPECTED:\n{}", s)
                    }
                    if let Ok(s) = serde_json::to_string_pretty(&grouped) {
                        eprintln!("ACTUAL:\n{}", s)
                    }
                    failures.push(format!("{}: {}", path.display(), e));
                }
            }
        }
    }
    assert!(
        html_file_count > 0,
        "compare_datasets: fixture dizininde hicbir .html dosyasi yok ({})",
        dir.display()
    );
    if !failures.is_empty() {
        panic!("Dataset comparison failures:\n{}", failures.join("\n"));
    }
}

#[test]
fn compare_fundpage_datasets() {
    if !validate_metadata_docs() {
        return;
    }

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root_candidates = [
        manifest.join("../../datasets/fundpage"),
        manifest.join("../../../datasets/fundpage"),
        manifest.join("../../../../../datasets/fundpage"),
        manifest.join("../../../../../tefas/datasets/fundpage"),
    ];
    let Some(root) = resolve_dataset_root(&root_candidates, "compare_fundpage_datasets") else {
        return;
    };
    let html_dir = root.join("html");
    let json_dir = root.join("json");

    let mut failures = Vec::new();
    for entry in fs::read_dir(&html_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("html") {
            let expected = json_dir.join(format!(
                "{}.json",
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .expect("invalid html file stem")
            ));
            if !expected.exists() {
                failures.push(format!(
                    "{}: missing expected {}",
                    path.display(),
                    expected.display()
                ));
                continue;
            }
            let html = fs::read_to_string(&path).expect("read fundpage html");
            let (grouped, _flat) = parse_document(&html);
            let expected_v: Value = serde_json::from_str(
                &fs::read_to_string(&expected).expect("read fundpage expected"),
            )
            .expect("parse fundpage expected json");
            if let Err(e) = compare_json(&expected_v, &grouped) {
                failures.push(format!("{}: {}", path.display(), e));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Fundpage dataset comparison failures:\n{}",
            failures.join("\n")
        );
    }
}

/// parse_document must never panic on empty or minimal input, and must return
/// two well-formed JSON values.
#[test]
fn test_parse_returns_two_values() {
    let (chart, info) = parse_document("");
    // Both outputs must be valid JSON values (any variant is fine for empty input)
    let _ = serde_json::to_string(&chart).expect("chart must be JSON-serialisable");
    let _ = serde_json::to_string(&info).expect("info must be JSON-serialisable");
}

/// FundPage::validate must return false when "failureconfig" is present and
/// true when it is absent.
#[test]
fn test_validate_failureconfig() {
    assert!(
        !FundPage::validate("some page with failureconfig marker"),
        "validate should return false when 'failureconfig' is present"
    );
    assert!(
        FundPage::validate("normal page content without the marker"),
        "validate should return true for a normal page"
    );
}

/// Profile-parity test: skips gracefully when no fixture directory exists.
/// When fixtures are added, this will verify parse_document output matches
/// expected profile JSON.
#[test]
#[ignore = "profile fixture'lari eklendiginde aktiflestirilecek"]
fn test_profile_parity() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest.join("tests/datasets/raw");
    if !fixture_dir.exists() {
        eprintln!(
            "test_profile_parity: skipping — no fixtures at {:?}",
            fixture_dir
        );
    }
    // TODO: iterate fixture_dir and compare parse_document output against
    //       expected *_profile.json counterparts when fixtures are available.
}

/// Generate golden `_parsed.json` files for any HTML fixtures that are missing them.
///
/// Run with:
///   cargo test -p tefas-parser generate_golden_parsed -- --ignored --nocapture
///
/// This writes `<name>_parsed.json` next to each `<name>.html` that does not
/// already have a golden file, using the current `parse_document` output as the
/// reference. Review the generated files before committing them.
#[test]
#[ignore]
fn generate_golden_parsed() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../../tests/datasets/raw"),
        manifest.join("../../../tefas/FundPage/datasets/raw"),
        manifest.join("../../../FundPage/datasets/raw"),
    ];

    let dir = candidates
        .iter()
        .find_map(|p| p.canonicalize().ok())
        .expect("datasets/raw not found");

    let mut generated = 0usize;
    let mut skipped = 0usize;

    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("html") {
            continue;
        }
        let golden = path.with_file_name(format!(
            "{}_parsed.json",
            path.file_stem().unwrap().to_string_lossy()
        ));
        if golden.exists() {
            eprintln!("SKIP (exists): {}", golden.display());
            skipped += 1;
            continue;
        }
        let html = fs::read_to_string(&path).expect("read html");
        let (grouped, _flat) = parse_document(&html);
        let json = serde_json::to_string_pretty(&grouped).expect("serialise");
        fs::write(&golden, &json).expect("write golden");
        eprintln!("WROTE: {}", golden.display());
        generated += 1;
    }
    println!("generate_golden_parsed: wrote {generated}, skipped {skipped}");
}
