use super::*;

#[test]
fn requested_concurrency_overrides_policy() {
    let req = QueryBatchRequest::new(vec![QueryOperationName::new("fonBilgiGetir")], Some(6));
    assert_eq!(req.effective_concurrency(), 6);
}

#[test]
fn policy_prefers_two_for_small_operation_sets() {
    let req = QueryBatchRequest::new(
        vec![
            QueryOperationName::new("fonBilgiGetir"),
            QueryOperationName::new("fonFiyatBilgiGetir"),
        ],
        None,
    );
    assert_eq!(req.effective_concurrency(), 2);
}

#[test]
fn policy_prefers_four_for_wide_operation_sets() {
    let ops = (0..10)
        .map(|i| QueryOperationName::new(format!("op{i}")))
        .collect();
    let req = QueryBatchRequest::new(ops, None);
    assert_eq!(req.effective_concurrency(), 4);
}

#[test]
fn build_plan_keeps_order_and_computes_concurrency() {
    let req = QueryBatchRequest::new(
        vec![
            QueryOperationName::new("fonBilgiGetir"),
            QueryOperationName::new("fonTipiGetir"),
        ],
        None,
    );
    let plan = build_query_batch_plan(req);
    assert_eq!(plan.concurrency, 2);
    assert_eq!(plan.operations[0].as_str(), "fonBilgiGetir");
    assert_eq!(plan.operations[1].as_str(), "fonTipiGetir");
}

#[test]
fn fundpage_defaults_to_two() {
    let req = FundpageBatchRequest::new(vec!["AC5".to_string(), "TLY".to_string()], None);
    let plan = build_fundpage_batch_plan(req);
    assert_eq!(plan.concurrency, 2);
}

#[test]
fn fetch_uses_backend_default_when_not_requested() {
    let req = FetchBatchRequest::new(vec!["https://www.tefas.gov.tr".to_string()], None, 4);
    let plan = build_fetch_batch_plan(req);
    assert_eq!(plan.concurrency, 4);
}

#[test]
fn fetch_requested_concurrency_overrides_backend_default() {
    let req = FetchBatchRequest::new(vec!["https://www.tefas.gov.tr".to_string()], Some(7), 2);
    let plan = build_fetch_batch_plan(req);
    assert_eq!(plan.concurrency, 7);
}
