use super::*;

#[test]
fn query_operation_name_roundtrip() {
    let op = QueryOperationName::new("fonBilgiGetir");
    assert_eq!(op.as_str(), "fonBilgiGetir");
}

#[test]
fn domain_error_display_prefixes_category() {
    let err = DomainError::Validation("missing code".to_string());
    assert!(err.to_string().starts_with("validation error:"));
}
