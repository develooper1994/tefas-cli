use super::{FundSummary, Operation, normalize_fund_summary_list};
use clap::ValueEnum;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

fn resolve_api_dataset_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../../datasets/api"),
        manifest.join("../../../datasets/api"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }
    panic!("datasets/api not found from {}", manifest.display());
}

fn contains_key_recursive(value: &serde_json::Value, target: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            if map.contains_key(target) {
                return true;
            }
            map.values().any(|v| contains_key_recursive(v, target))
        }
        serde_json::Value::Array(items) => items.iter().any(|v| contains_key_recursive(v, target)),
        _ => false,
    }
}

#[test]
fn typed_default_payloads_match_dynamic_defaults() {
    for op in [
        Operation::FonBilgiGetir,
        Operation::FonProfilDtyGetir,
        Operation::FonGetiriBazliBilgiGetir,
    ] {
        assert_eq!(
            op.default_payload_typed(),
            Some(op.default_payload()),
            "{op:?}"
        );
    }
}

#[test]
fn normalize_fund_summary_from_data_array() {
    let input = json!({
        "data": [
            {"fonKodu": "ABC123", "fonUnvan": "Fund A"},
            {"fonKod": "XYZ789", "unvan": "Fund B"}
        ]
    });
    let result = normalize_fund_summary_list(&input);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].fund_code.as_deref(), Some("ABC123"));
    assert_eq!(result[0].fund_name.as_deref(), Some("Fund A"));
    assert_eq!(result[1].fund_code.as_deref(), Some("XYZ789"));
    assert_eq!(result[1].fund_name.as_deref(), Some("Fund B"));
}

#[test]
fn normalize_fund_summary_from_result_list() {
    let input = json!({
        "resultList": [
            {"fonKodu": "RSLT01", "fonUnvan": "Result Fund"}
        ],
        "data": [
            {"fonKodu": "DATA01", "fonUnvan": "Data Fund"}
        ]
    });
    let result = normalize_fund_summary_list(&input);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].fund_code.as_deref(), Some("RSLT01"));
    assert_eq!(result[0].fund_name.as_deref(), Some("Result Fund"));
}

#[test]
fn normalize_fund_summary_single_object() {
    let input = json!({"fonKodu": "SINGLE", "fonUnvan": "Single Fund"});
    let result = normalize_fund_summary_list(&input);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].fund_code.as_deref(), Some("SINGLE"));
    assert_eq!(result[0].fund_name.as_deref(), Some("Single Fund"));
}

#[test]
fn fund_summary_human_line_missing_fields() {
    let empty = FundSummary::default();
    assert_eq!(empty.human_line(), " ()");

    let name_only = FundSummary {
        fund_name: Some("Only Name".to_string()),
        fund_code: None,
    };
    assert_eq!(name_only.human_line(), "Only Name ()");

    let code_only = FundSummary {
        fund_name: None,
        fund_code: Some("CODE".to_string()),
    };
    assert_eq!(code_only.human_line(), " (CODE)");
}

#[test]
fn operation_payload_contract_matches_captured_input_keys() {
    for op in Operation::value_variants() {
        let payload = op.default_payload();
        assert!(
            payload.is_object(),
            "default payload for {:?} should be a JSON object",
            op
        );
    }

    let expected_contracts: Vec<(Operation, Vec<&str>)> = vec![
        (Operation::FonBilgiGetir, vec!["dil", "fonKodu"]),
        (
            Operation::FonProfilDtyGetir,
            vec!["dil", "fonKodu", "periyod"],
        ),
        (
            Operation::FonGetiriBazliBilgiGetir,
            vec!["dil", "fonTipi", "fonTurKod", "fonGrubu"],
        ),
        (Operation::GetLogo, vec!["memberCodes"]),
        (Operation::GetBanners, vec!["dil", "cacheTTL"]),
    ];

    for (op, keys) in expected_contracts {
        let payload = op.default_payload();
        let obj = payload
            .as_object()
            .unwrap_or_else(|| panic!("default payload should be object for {:?}", op));
        for key in keys {
            assert!(
                obj.contains_key(key),
                "default payload for {:?} is missing expected key '{}'",
                op,
                key
            );
        }
    }
}

#[test]
fn endpoint_response_contract_has_expected_shape_and_keys() {
    let dataset_root = resolve_api_dataset_root();
    let metadata_path = dataset_root.join("metadata.json");
    let metadata: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&metadata_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", metadata_path.display(), e)),
    )
    .unwrap_or_else(|e| panic!("invalid metadata JSON {}: {}", metadata_path.display(), e));

    let endpoints = metadata["endpoints"]
        .as_array()
        .unwrap_or_else(|| panic!("metadata endpoints should be array"));

    for endpoint in endpoints {
        let name = endpoint["name"]
            .as_str()
            .unwrap_or_else(|| panic!("endpoint name should be string"));
        let base_dir = dataset_root.join(name);
        let output_path = base_dir.join("output.body");
        if !output_path.exists() {
            continue;
        }

        let output_json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&output_path)
                .unwrap_or_else(|e| panic!("failed to read {}: {}", output_path.display(), e)),
        )
        .unwrap_or_else(|e| panic!("invalid response JSON {}: {}", output_path.display(), e));

        assert!(
            output_json.is_array() || output_json.is_object(),
            "response contract for {} must be object or array",
            name
        );

        let keys_path = base_dir.join("keys.txt");
        if keys_path.exists() {
            let expected_keys: Vec<String> = serde_json::from_str(
                &fs::read_to_string(&keys_path)
                    .unwrap_or_else(|e| panic!("failed to read {}: {}", keys_path.display(), e)),
            )
            .unwrap_or_else(|e| panic!("invalid keys JSON {}: {}", keys_path.display(), e));

            for expected in expected_keys {
                assert!(
                    contains_key_recursive(&output_json, &expected),
                    "response contract for {} missing expected key '{}'",
                    name,
                    expected
                );
            }
        }
    }
}
