use tefas::{
    FetchBatchRequest, FundpageBatchRequest, FundpageRunOptions, QueryBatchRequest,
    QueryOperationName, TefasClient,
};

#[test]
fn external_consumer_can_use_public_api_surface() {
    let _client = TefasClient::from_defaults().expect("default client should construct");
    let _client2 = TefasClient::from_base_url("https://www.tefas.gov.tr")
        .expect("base-url client should construct");

    let _fetch = FetchBatchRequest::new(vec!["https://www.tefas.gov.tr".to_string()], None, 4);
    let _fund = FundpageBatchRequest::new(vec!["AC5".to_string()], Some(4));
    let _query = QueryBatchRequest::new(
        vec![QueryOperationName::new("fonBilgiGetir")],
        Some(4),
    );

    let opts = FundpageRunOptions {
        network_concurrency: Some(8),
        parse_concurrency: Some(8),
        quiet: true,
    };
    assert_eq!(opts.network_concurrency, Some(8));
    assert_eq!(opts.parse_concurrency, Some(8));
    assert!(opts.quiet);
}
