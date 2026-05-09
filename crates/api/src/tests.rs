use super::{FundSummary, Operation, normalize_fund_summary_list};
use serde_json::json;

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
