use std::collections::HashSet;

use anyhow::Context;
use clap::ValueEnum;
use serde_json::{Map, Value, json};

use crate::{
    AppConfig, FetchBatchRequest, FundpageBatchRequest, FundpageJob, ImageConvertStats,
    ImageOutputFormat, NetworkClient, Operation, OperationOld, QueryBatchRequest, QueryJob,
    QueryOperationName, build_fetch_batch_plan, build_fundpage_batch_plan, build_query_batch_plan,
    convert_getlogo_to_images, run_fetch_batch, run_fundpage_batch, run_query_batch,
};

/// High-level library facade for workflows that the CLI performs.
///
/// This keeps CLI usage patterns available for non-CLI consumers while staying
/// fully async and composable.
pub struct TefasClient {
    cfg: AppConfig,
    base_url: String,
    client: NetworkClient,
}

#[derive(Clone, Debug, Default)]
pub struct FundpageRunOptions {
    pub network_concurrency: Option<usize>,
    pub parse_concurrency: Option<usize>,
    pub quiet: bool,
}

enum AnyOperation {
    New(Operation),
    Old(OperationOld),
}

fn default_parse_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 16)
}

impl TefasClient {
    /// Create a client using default AppConfig values.
    pub fn from_defaults() -> anyhow::Result<Self> {
        Self::new(AppConfig::default())
    }

    /// Create a client from defaults while overriding only the base URL.
    pub fn from_base_url(base_url: impl Into<String>) -> anyhow::Result<Self> {
        let cfg = AppConfig {
            base_url: base_url.into(),
            ..AppConfig::default()
        };
        Self::new(cfg)
    }

    /// Create a high-level TEFAS client from application config.
    pub fn new(cfg: AppConfig) -> anyhow::Result<Self> {
        let client = NetworkClient::new(&cfg)?;
        let base_url = cfg.normalized_base_url().to_string();
        Ok(Self {
            cfg,
            base_url,
            client,
        })
    }

    pub fn config(&self) -> &AppConfig {
        &self.cfg
    }

    pub fn network_client(&self) -> &NetworkClient {
        &self.client
    }

    /// Optional preflight call before running network-heavy workflows.
    pub async fn preflight(&self) -> anyhow::Result<()> {
        self.client.preflight().await
    }

    /// CLI-equivalent multi-URL fetch workflow.
    pub async fn fetch_urls(
        &self,
        request: FetchBatchRequest,
        quiet: bool,
    ) -> Vec<(String, anyhow::Result<String>)> {
        let plan = build_fetch_batch_plan(request);
        run_fetch_batch(&self.client, plan.urls, plan.concurrency, quiet).await
    }

    /// CLI-equivalent fundpage workflow over plain code list.
    pub async fn fundpages(
        &self,
        request: FundpageBatchRequest,
        quiet: bool,
    ) -> Vec<(String, anyhow::Result<Value>)> {
        let jobs = request
            .codes
            .iter()
            .map(|code| FundpageJob {
                code: code.to_uppercase(),
                html_save_dest: None,
            })
            .collect::<Vec<_>>();

        let plan = build_fundpage_batch_plan(request);
        run_fundpage_batch(
            &self.client,
            &self.base_url,
            jobs,
            plan.concurrency,
            default_parse_concurrency(),
            quiet,
        )
        .await
    }

    /// High-level convenience method for collecting multiple fund pages with
    /// optional network/parse concurrency overrides.
    pub async fn collect_fundpages(
        &self,
        codes: Vec<String>,
        options: FundpageRunOptions,
    ) -> Vec<(String, anyhow::Result<Value>)> {
        let jobs = codes
            .iter()
            .map(|code| FundpageJob {
                code: code.to_uppercase(),
                html_save_dest: None,
            })
            .collect::<Vec<_>>();

        let plan = build_fundpage_batch_plan(FundpageBatchRequest::new(
            codes,
            options.network_concurrency,
        ));

        run_fundpage_batch(
            &self.client,
            &self.base_url,
            jobs,
            plan.concurrency,
            options
                .parse_concurrency
                .unwrap_or_else(default_parse_concurrency),
            options.quiet,
        )
        .await
    }

    /// Convenience API for single-operation query calls.
    pub async fn query_one(
        &self,
        operation: Operation,
        set_overrides: Vec<(String, Value)>,
        custom_payload: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.query(vec![operation], vec![], None, set_overrides, custom_payload)
            .await
    }

    /// Convenience API for single legacy-operation query calls.
    pub async fn query_legacy_one(
        &self,
        operation: OperationOld,
        set_overrides: Vec<(String, Value)>,
        custom_payload: Option<Value>,
    ) -> anyhow::Result<Value> {
        self.query(vec![], vec![operation], None, set_overrides, custom_payload)
            .await
    }

    /// Dynamic convenience API that resolves operation names (new + legacy)
    /// and executes a single merged query batch.
    pub async fn query_names(
        &self,
        names: Vec<String>,
        requested_concurrency: Option<usize>,
        set_overrides: Vec<(String, Value)>,
        custom_payload: Option<Value>,
    ) -> anyhow::Result<Value> {
        let operation_names = names
            .into_iter()
            .map(QueryOperationName::new)
            .collect::<Vec<_>>();
        self.query_by_names(
            QueryBatchRequest::new(operation_names, requested_concurrency),
            set_overrides,
            custom_payload,
        )
        .await
    }

    /// CLI-equivalent fundpage workflow with explicit jobs (supports HTML save destinations).
    pub async fn fundpage_jobs(
        &self,
        jobs: Vec<FundpageJob>,
        requested_concurrency: Option<usize>,
        quiet: bool,
    ) -> Vec<(String, anyhow::Result<Value>)> {
        let codes = jobs.iter().map(|job| job.code.clone()).collect::<Vec<_>>();
        let plan =
            build_fundpage_batch_plan(FundpageBatchRequest::new(codes, requested_concurrency));
        run_fundpage_batch(
            &self.client,
            &self.base_url,
            jobs,
            plan.concurrency,
            default_parse_concurrency(),
            quiet,
        )
        .await
    }

    /// Query by explicit enum operations.
    ///
    /// Returns merged JSON object where keys are operation names.
    pub async fn query(
        &self,
        operations: Vec<Operation>,
        legacy_operations: Vec<OperationOld>,
        requested_concurrency: Option<usize>,
        set_overrides: Vec<(String, Value)>,
        custom_payload: Option<Value>,
    ) -> anyhow::Result<Value> {
        let operation_names = operations
            .iter()
            .map(|op| QueryOperationName::new(op.to_possible_value().expect("value").get_name()))
            .chain(legacy_operations.iter().map(|op| {
                QueryOperationName::new(op.to_possible_value().expect("value").get_name())
            }))
            .collect::<Vec<_>>();

        self.query_by_names(
            QueryBatchRequest::new(operation_names, requested_concurrency),
            set_overrides,
            custom_payload,
        )
        .await
    }

    /// Query by operation name strings (`Operation` + legacy `OperationOld` supported).
    ///
    /// This is useful for dynamic integrations that do not want to depend on enum types.
    pub async fn query_by_names(
        &self,
        request: QueryBatchRequest,
        set_overrides: Vec<(String, Value)>,
        custom_payload: Option<Value>,
    ) -> anyhow::Result<Value> {
        let plan = build_query_batch_plan(request);

        let mut jobs = Vec::with_capacity(plan.operations.len());
        let mut seen_names = HashSet::new();

        for (idx, op_name) in plan.operations.into_iter().enumerate() {
            let resolved = Self::find_operation(op_name.as_str())
                .with_context(|| format!("unknown operation: {}", op_name.as_str()))?;

            match resolved {
                AnyOperation::New(op) => {
                    let name = op
                        .to_possible_value()
                        .expect("value")
                        .get_name()
                        .to_string();
                    if !seen_names.insert(name.clone()) {
                        continue;
                    }
                    let spec = op.spec();
                    let url = Self::resolve_url(&self.base_url, spec.endpoint);
                    jobs.push(QueryJob {
                        idx,
                        name,
                        url,
                        referer: spec.referer,
                        default_payload: op
                            .default_payload_typed()
                            .unwrap_or_else(|| op.default_payload()),
                    });
                }
                AnyOperation::Old(op) => {
                    let name = op
                        .to_possible_value()
                        .expect("value")
                        .get_name()
                        .to_string();
                    if !seen_names.insert(name.clone()) {
                        continue;
                    }
                    let spec = op.spec();
                    let url = Self::resolve_url(&self.base_url, spec.endpoint);
                    jobs.push(QueryJob {
                        idx,
                        name,
                        url,
                        referer: spec.referer,
                        default_payload: op.default_payload(),
                    });
                }
            }
        }

        let ordered_results = run_query_batch(
            &self.client,
            jobs,
            plan.concurrency,
            set_overrides,
            custom_payload,
        )
        .await?;

        let mut merged = Map::new();
        for (_, name, value) in ordered_results {
            merged.insert(name, value);
        }

        Ok(Value::Object(merged))
    }

    /// Raw `getLogo` operation response.
    pub async fn fetch_logos_raw(&self, member_codes: Vec<String>) -> anyhow::Result<Value> {
        let spec = Operation::GetLogo.spec();
        let url = Self::resolve_url(&self.base_url, spec.endpoint);
        let payload = json!({"memberCodes": member_codes});
        self.client
            .post_json_with_referer(&url, spec.referer, &payload)
            .await
    }

    /// End-to-end logo workflow: fetch + convert into PNG/JPEG files.
    pub async fn fetch_logos_and_convert(
        &self,
        member_codes: Vec<String>,
        outdir: &str,
        format: ImageOutputFormat,
        quality: u8,
        verbose: bool,
    ) -> anyhow::Result<ImageConvertStats> {
        let raw = self.fetch_logos_raw(member_codes).await?;
        convert_getlogo_to_images(&raw, outdir, format, quality, verbose)
    }

    fn resolve_url(base_url: &str, endpoint: &str) -> String {
        if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("{}{}", base_url, endpoint)
        }
    }

    fn find_operation(name: &str) -> Option<AnyOperation> {
        for &op in Operation::value_variants() {
            if let Some(pv) = op.to_possible_value()
                && pv.matches(name, true)
            {
                return Some(AnyOperation::New(op));
            }
        }

        for &op in OperationOld::value_variants() {
            if let Some(pv) = op.to_possible_value()
                && pv.matches(name, true)
            {
                return Some(AnyOperation::Old(op));
            }
        }

        None
    }
}
