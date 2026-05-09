pub mod client;
pub mod executors;
pub mod runtime;

pub use client::TefasClient;
pub use executors::{
    apply_set_overrides, run_fetch_batch, run_fundpage_batch, run_query_batch, FundpageJob,
    QueryJob,
};
pub use tefas_api::{normalize_fund_summary_list, Operation, OperationOld};
pub use tefas_config::{
    AppConfig, AuthConfig, HttpBackend, RetryConfig, TlsBackend, TlsConfig, DEFAULT_USER_AGENT,
};
pub use tefas_domain::{
    DomainError, FundIdentity, FundProfile, FundReturns, QueryOperationName,
};
pub use tefas_media::{convert_getlogo_to_images, ImageConvertStats, ImageOutputFormat};
pub use tefas_network::{extract_hidden_fields, form_urlencode, NetworkClient};
pub use tefas_parser::fund_page::parse_document;
pub use tefas_parser::{FundPage, FundPageMeta, FundPageOutput, TefasPage};
pub use tefas_usecase::{
    build_fetch_batch_plan, build_fundpage_batch_plan, build_query_batch_plan, FetchBatchPlan,
    FetchBatchRequest, FundpageBatchPlan, FundpageBatchRequest, QueryBatchPlan,
    QueryBatchRequest,
};
