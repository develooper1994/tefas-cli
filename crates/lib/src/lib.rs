pub mod client;
pub mod executors;
pub mod runtime;

pub use client::TefasClient;
pub use executors::{
    FundpageJob, QueryJob, apply_set_overrides, run_fetch_batch, run_fundpage_batch,
    run_query_batch,
};
pub use tefas_api::{Operation, OperationOld, normalize_fund_summary_list};
pub use tefas_config::{
    AppConfig, AuthConfig, DEFAULT_USER_AGENT, HttpBackend, RetryConfig, TlsBackend, TlsConfig,
};
pub use tefas_domain::{DomainError, FundIdentity, FundProfile, FundReturns, QueryOperationName};
pub use tefas_media::{ImageConvertStats, ImageOutputFormat, convert_getlogo_to_images};
pub use tefas_network::{NetworkClient, extract_hidden_fields, form_urlencode};
pub use tefas_parser::fund_page::parse_document;
pub use tefas_parser::{FundPage, FundPageMeta, FundPageOutput, TefasPage};
pub use tefas_usecase::{
    FetchBatchPlan, FetchBatchRequest, FundpageBatchPlan, FundpageBatchRequest, QueryBatchPlan,
    QueryBatchRequest, build_fetch_batch_plan, build_fundpage_batch_plan, build_query_batch_plan,
};
