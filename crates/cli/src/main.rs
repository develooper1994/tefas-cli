use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tefas::{
    AppConfig, AuthConfig, DEFAULT_USER_AGENT, FetchBatchRequest, FundpageBatchRequest,
    FundpageJob, HttpBackend, NetworkClient, Operation, OperationOld, QueryBatchRequest, QueryJob,
    QueryOperationName, RetryConfig, TlsBackend, TlsConfig, build_fetch_batch_plan,
    build_fundpage_batch_plan, build_query_batch_plan, convert_getlogo_to_images, parse_document,
    run_fetch_batch, run_fundpage_batch, run_query_batch,
};
use tokio::sync::Semaphore;

// ── Shared Types ─────────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Json,
    Humanize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ImageOutputFormat {
    Png,
    Jpeg,
}

impl From<ImageOutputFormat> for tefas::ImageOutputFormat {
    fn from(f: ImageOutputFormat) -> Self {
        match f {
            ImageOutputFormat::Png => tefas::ImageOutputFormat::Png,
            ImageOutputFormat::Jpeg => tefas::ImageOutputFormat::Jpeg,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum BackendChoice {
    Wreq,
    Reqwest,
    Hyper,
    Impcurl,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum BackendTls {
    Rustls,
    #[value(name = "nativetls")]
    NativeTls,
    Insecure,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum PersonaPreset {
    #[value(name = "desktop-windows")]
    DesktopWindows,
    #[value(name = "desktop-macos")]
    DesktopMacos,
    #[value(name = "android")]
    Android,
    #[value(name = "ios")]
    Ios,
}

// ── CLI Definition ───────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "tefas",
    about = "Unified TEFAS CLI - Modern fund data tooling",
    version
)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    global: GlobalArgs,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Shortcut to fetch and parse fund pages by their codes (e.g. tefas fundpage AC5 TLY)
    #[command(alias = "fp")]
    Fundpage {
        /// Fund codes (e.g. AC5, TLY). Uppercase applied automatically.
        #[arg(required = true, num_args = 1..)]
        codes: Vec<String>,

        /// Output path(s). Absent + 1 code: stdout. Absent + multi: <CODE>.json per code.
        /// --output (no args): <CODE>.json per code.
        /// --output file.json (1 path): merged {"CODE":{...}} → file.
        /// --output a.json b.json (N == codes): individual files.
        #[arg(short, long, num_args = 0.., value_name = "FILE")]
        output: Option<Vec<String>>,

        /// HTML save path(s). --save-html (no args): <CODE>.html in cwd.
        /// --save-html dir (1 path): <dir>/<CODE>.html.
        /// --save-html a.html b.html (N == codes): individual paths.
        #[arg(long, num_args = 0.., value_name = "PATH")]
        save_html: Option<Vec<String>>,

        /// Print extracted field names instead of full values.
        /// Paths are returned in dot-notation (arrays use []), e.g. profile.fon_kodu.
        #[arg(long)]
        fields: bool,
    },

    /// Run Takasbank/TEFAS JSON API operations
    #[command(alias = "q")]
    Query {
        /// Operation name (e.g. fonBilgiGetir).
        #[arg(value_enum)]
        operation: Vec<Operation>,

        /// Legacy ASMX operation name (e.g. getAllFunds).
        #[arg(long, value_enum)]
        old: Vec<OperationOld>,

        /// List all available operations
        #[arg(long)]
        list: bool,

        /// Show details (endpoint, payload keys) for a named operation (new or legacy)
        #[arg(long, value_name = "OP_NAME")]
        info: Option<String>,

        /// Override a field in the request payload (KEY=VALUE). Repeatable.
        #[arg(short, long, value_name = "KEY=VALUE")]
        set: Vec<String>,

        /// Replace the entire request payload with a JSON string
        #[arg(long)]
        payload: Option<String>,

        /// Output format: json (default) or humanize
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },

    /// Fetch raw HTML from one or more URLs with WAF bypass defaults
    #[command(alias = "f")]
    Fetch {
        /// URLs to fetch (1 or more)
        #[arg(required = true, num_args = 1..)]
        urls: Vec<String>,

        /// Output path(s). Absent + 1 URL: tefas_fetched.html.
        /// Absent + multi URL or --output (no args): auto-named per URL in cwd.
        /// --output dir (1 path): write auto-named files into that directory.
        /// --output a.html b.html (N == urls): individual files.
        #[arg(short, long, num_args = 0.., value_name = "PATH")]
        output: Option<Vec<String>>,

        /// Skip the preflight session warmup
        #[arg(long)]
        skip_preflight: bool,
    },

    /// Parse a local HTML fund page into JSON
    #[command(alias = "p")]
    Parse {
        /// Input HTML file paths
        #[arg(required = true, num_args = 1..)]
        inputs: Vec<String>,

        /// Output JSON file path (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Print extracted field names instead of full values.
        /// Paths are returned in dot-notation (arrays use []).
        #[arg(long)]
        fields: bool,
    },

    /// Download and convert fund logos to images
    #[command(alias = "l")]
    Logo {
        /// Fund/Member codes to fetch logos for
        codes: Vec<String>,

        /// Directory to write images into
        #[arg(short, long, default_value = "./logos")]
        outdir: String,

        /// Image format: png (default) or jpeg
        #[arg(short, long, value_enum, default_value_t = ImageOutputFormat::Png)]
        format: ImageOutputFormat,

        /// JPEG quality 1-100
        #[arg(long, default_value_t = 85)]
        quality: u8,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell type: bash, zsh, fish, powershell, elvish
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Args, Debug, Clone)]
struct GlobalArgs {
    /// Request timeout in seconds
    #[arg(long, default_value_t = 25, env = "TEFAS_TIMEOUT", global = true)]
    timeout: u64,

    /// Pretty-print JSON output
    #[arg(long, default_value_t = true, global = true)]
    pretty: bool,

    /// TEFAS base URL
    #[arg(
        long,
        default_value = "https://www.tefas.gov.tr",
        env = "TEFAS_BASEURL",
        global = true
    )]
    base_url: String,

    /// HTTP client backend: wreq, reqwest, hyper, impcurl (default: wreq)
    #[arg(long, value_enum, default_value_t = BackendChoice::Wreq, env = "TEFAS_BACKEND_NETWORK", global = true)]
    backend: BackendChoice,

    /// TLS backend: rustls, nativetls, insecure
    #[arg(long, value_enum, default_value_t = BackendTls::Rustls, env = "TEFAS_BACKEND_TLS", global = true)]
    tls: BackendTls,

    /// Browser/device persona preset
    #[arg(long, value_enum, env = "TEFAS_PERSONA", global = true)]
    persona: Option<PersonaPreset>,

    /// curl-impersonate profile (e.g. chrome136)
    #[arg(long, env = "TEFAS_IMPCURL_IMPERSONATE", global = true)]
    impersonate: Option<String>,

    /// HTTP/HTTPS/SOCKS5 proxy URL
    #[arg(long, env = "TEFAS_PROXY", global = true)]
    proxy: Option<String>,

    /// Maximum number of concurrent network tasks for fundpage/fetch/query.
    /// Lower values reduce WAF pressure on network-heavy commands.
    #[arg(short = 'n', long, env = "TEFAS_NETWORK_CONCURRENCY", global = true)]
    network_concurrency: Option<usize>,

    /// Maximum number of concurrent local parse tasks.
    #[arg(short = 'P', long, env = "TEFAS_PARSE_CONCURRENCY", global = true)]
    parse_concurrency: Option<usize>,

    /// Legacy fallback concurrency knob. Used only if the more specific flags
    /// are not provided.
    #[arg(long, env = "TEFAS_CONCURRENCY", global = true)]
    concurrency: Option<usize>,

    /// Suppress informational messages
    #[arg(short, long, default_value_t = false, global = true)]
    quiet: bool,
}

// ── Logic Helpers ────────────────────────────────────────────────────────────

fn persona_defaults(preset: PersonaPreset) -> (&'static str, &'static str) {
    match preset {
        PersonaPreset::DesktopWindows => (
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
            "chrome136",
        ),
        PersonaPreset::DesktopMacos => (
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_6_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
            "chrome136",
        ),
        PersonaPreset::Android => (
            "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Mobile Safari/537.36",
            "chrome136_android",
        ),
        PersonaPreset::Ios => (
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
            "safari17_2_ios",
        ),
    }
}

fn has_impcurl() -> bool {
    Command::new("curl-impersonate-chrome")
        .arg("--version")
        .output()
        .is_ok()
        || Command::new("curl_chrome136")
            .arg("--version")
            .output()
            .is_ok()
}

fn build_app_config(global: &GlobalArgs, auto_waf: bool) -> AppConfig {
    let mut backend = match global.backend {
        BackendChoice::Wreq => HttpBackend::Wreq,
        BackendChoice::Reqwest => HttpBackend::Reqwest,
        BackendChoice::Hyper => HttpBackend::Hyper,
        BackendChoice::Impcurl => HttpBackend::Impcurl,
    };

    let (mut ua, mut imp) = (DEFAULT_USER_AGENT.to_string(), global.impersonate.clone());

    if let Some(preset) = global.persona {
        let (p_ua, p_imp) = persona_defaults(preset);
        ua = p_ua.to_string();
        imp = imp.or_else(|| Some(p_imp.to_string()));
    }

    // Smart defaults for WAF bypass
    if auto_waf && global.backend == BackendChoice::Wreq && has_impcurl() {
        backend = HttpBackend::Impcurl;
        if global.persona.is_none() {
            let (p_ua, p_imp) = persona_defaults(PersonaPreset::DesktopWindows);
            ua = p_ua.to_string();
            imp = Some(p_imp.to_string());
        }
    }

    AppConfig {
        timeout_secs: global.timeout,
        pretty: global.pretty,
        base_url: global.base_url.clone(),
        backend,
        retry: RetryConfig::default(),
        tls: TlsConfig {
            backend: match global.tls {
                BackendTls::Rustls => TlsBackend::Rustls,
                BackendTls::NativeTls => TlsBackend::NativeTls,
                BackendTls::Insecure => TlsBackend::Insecure,
            },
            timeout_ms: None,
        },
        auth: AuthConfig {
            user_agent: ua,
            impcurl_impersonate: imp,
            ..AuthConfig::default()
        },
        proxy: global.proxy.clone(),
    }
}

async fn get_client(cfg: &AppConfig) -> anyhow::Result<NetworkClient> {
    let client = NetworkClient::new(cfg)?;
    if !cfg.auth.skip_preflight {
        let _ = client.preflight().await;
    }
    Ok(client)
}

fn default_request_concurrency(backend: HttpBackend) -> usize {
    match backend {
        HttpBackend::Impcurl => 2,
        HttpBackend::Wreq | HttpBackend::Reqwest | HttpBackend::Hyper => 4,
    }
}

const MAX_NETWORK_CONCURRENCY: usize = 64;
const MAX_PARSE_CONCURRENCY: usize = 32;

fn default_parse_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(1, 16)
}

fn effective_concurrency(requested: Option<usize>, default_limit: usize) -> usize {
    requested.unwrap_or(default_limit).max(1)
}

fn clamp_concurrency(limit: usize, max_limit: usize) -> usize {
    limit.min(max_limit)
}

fn effective_network_concurrency(global: &GlobalArgs, default_limit: usize) -> usize {
    let raw = effective_concurrency(
        global.network_concurrency.or(global.concurrency),
        default_limit,
    );
    clamp_concurrency(raw, MAX_NETWORK_CONCURRENCY)
}

fn effective_parse_concurrency(global: &GlobalArgs) -> usize {
    let raw = effective_concurrency(
        global.parse_concurrency.or(global.concurrency),
        default_parse_concurrency(),
    );
    clamp_concurrency(raw, MAX_PARSE_CONCURRENCY)
}

fn parse_set_pair(input: &str) -> anyhow::Result<(String, Value)> {
    let mut parts = input.splitn(2, '=');
    let key = parts.next().unwrap_or_default().trim();
    let raw_value = parts.next().unwrap_or_default().trim();
    if key.is_empty() || raw_value.is_empty() {
        return Err(anyhow::anyhow!(
            "invalid --set value '{input}', expected KEY=VALUE"
        ));
    }
    let value = serde_json::from_str::<Value>(raw_value)
        .unwrap_or_else(|_| Value::String(raw_value.to_string()));
    Ok((key.to_string(), value))
}

fn json_to_text(pretty: bool, value: &Value) -> anyhow::Result<String> {
    if pretty {
        Ok(serde_json::to_string_pretty(value)?)
    } else {
        Ok(serde_json::to_string(value)?)
    }
}

fn collect_field_paths(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                out.insert(next.clone());
                collect_field_paths(child, &next, out);
            }
        }
        Value::Array(items) => {
            let next = format!("{prefix}[]");
            out.insert(next.clone());
            for child in items {
                collect_field_paths(child, &next, out);
            }
        }
        _ => {}
    }
}

fn field_paths_json(value: &Value) -> Value {
    let mut fields = BTreeSet::new();
    collect_field_paths(value, "", &mut fields);
    Value::Array(fields.into_iter().map(Value::String).collect())
}

fn ensure_parent_dir(path: &str) -> anyhow::Result<()> {
    if let Some(parent) = Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_or_stdout(path: Option<&str>, content: &str) -> anyhow::Result<()> {
    match path {
        Some(p) => {
            ensure_parent_dir(p)?;
            fs::write(p, content)?;
        }
        None => println!("{}", content),
    }
    Ok(())
}

// ── Routing Module ─────────────────────────────────────────────────────────

mod routing;
use routing::{
    AnyOperation, FetchOutputPlan, FundpageOutputPlan, find_operation, resolve_fetch_outputs,
    resolve_fundpage_outputs, resolve_parse_input_path, resolve_save_html_paths,
};

// ── Main Entry ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fundpage {
            codes,
            output,
            save_html,
            fields,
        } => {
            let cfg = build_app_config(&cli.global, true);
            let network_concurrency = effective_network_concurrency(
                &cli.global,
                default_request_concurrency(cfg.backend),
            );
            let fundpage_plan = build_fundpage_batch_plan(FundpageBatchRequest::new(
                codes.clone(),
                Some(network_concurrency),
            ));
            let parse_concurrency = effective_parse_concurrency(&cli.global);

            let out_plan = resolve_fundpage_outputs(&codes, output)?;
            let html_paths = resolve_save_html_paths(&codes, save_html)?;

            let client = get_client(&cfg).await?;

            let jobs: Vec<FundpageJob> = codes
                .iter()
                .enumerate()
                .map(|(idx, raw_code)| FundpageJob {
                    code: raw_code.to_uppercase(),
                    html_save_dest: html_paths.as_ref().map(|v| v[idx].clone()),
                })
                .collect();

            let raw_results = run_fundpage_batch(
                &client,
                &cfg.normalized_base_url(),
                jobs,
                fundpage_plan.concurrency,
                parse_concurrency,
                cli.global.quiet,
            )
            .await;

            let mut ordered: Vec<(String, Value)> = Vec::with_capacity(raw_results.len());
            for (code, result) in raw_results {
                match result {
                    Ok(val) => {
                        let payload = if fields { field_paths_json(&val) } else { val };
                        ordered.push((code, payload));
                    }
                    Err(e) => {
                        if !cli.global.quiet {
                            eprintln!("Error fetching {}: {}", code, e);
                        }
                    }
                }
            }

            match out_plan {
                FundpageOutputPlan::Stdout => {
                    let val = ordered
                        .into_iter()
                        .next()
                        .map(|(_, v)| v)
                        .unwrap_or(Value::Null);
                    println!("{}", json_to_text(cfg.pretty, &val)?);
                }
                FundpageOutputPlan::MergedFile(path) => {
                    let mut map = serde_json::Map::new();
                    for (code, val) in ordered {
                        map.insert(code, val);
                    }
                    let content = json_to_text(cfg.pretty, &Value::Object(map))?;
                    ensure_parent_dir(path.to_str().unwrap_or(""))?;
                    fs::write(&path, content)?;
                    if !cli.global.quiet {
                        eprintln!("Wrote merged output → {}", path.display());
                    }
                }
                FundpageOutputPlan::PerCode(plan) => {
                    let result_map: std::collections::HashMap<String, Value> =
                        ordered.into_iter().collect();
                    for (code, dest) in plan {
                        if let Some(val) = result_map.get(&code) {
                            let content = json_to_text(cfg.pretty, val)?;
                            ensure_parent_dir(dest.to_str().unwrap_or(""))?;
                            fs::write(&dest, content)?;
                            if !cli.global.quiet {
                                eprintln!("Wrote {} → {}", code, dest.display());
                            }
                        }
                    }
                }
            }
        }

        Commands::Query {
            operation,
            old,
            list,
            info,
            set,
            payload,
            format,
        } => {
            let cfg = build_app_config(&cli.global, false);
            let network_concurrency =
                effective_network_concurrency(&cli.global, default_request_concurrency(cfg.backend));

            // --list and --info are discovery-only; reject mixing with operation args
            if (list || info.is_some()) && (!operation.is_empty() || !old.is_empty()) {
                anyhow::bail!("--list/--info cannot be combined with operation arguments");
            }

            // Require at least one operation, --list, or --info
            if operation.is_empty() && old.is_empty() && !list && info.is_none() {
                anyhow::bail!(
                    "the following required arguments were not provided:\n  <OPERATION>...\n\nUsage: tefas-cli query [OPERATION]...\n\nFor more information, try '--help'."
                );
            }

            if list {
                if format == OutputFormat::Json {
                    let ops: Vec<Value> = Operation::value_variants()
                        .iter()
                        .map(|op| {
                            let spec = op.spec();
                            let default_p = op.default_payload();
                            let pkeys: Vec<&str> = if let Value::Object(ref m) = default_p {
                                m.keys().map(|k| k.as_str()).collect()
                            } else {
                                vec![]
                            };
                            json!({
                                "name": op.to_possible_value().unwrap().get_name(),
                                "endpoint": spec.endpoint,
                                "referer": spec.referer,
                                "description": op.description(),
                                "payload_keys": pkeys,
                            })
                        })
                        .collect();
                    let legacy: Vec<Value> = OperationOld::value_variants()
                        .iter()
                        .map(|op| {
                            let spec = op.spec();
                            let default_p = op.default_payload();
                            let pkeys: Vec<&str> = if let Value::Object(ref m) = default_p {
                                m.keys().map(|k| k.as_str()).collect()
                            } else {
                                vec![]
                            };
                            json!({
                                "name": op.to_possible_value().unwrap().get_name(),
                                "endpoint": spec.endpoint,
                                "referer": spec.referer,
                                "description": op.description(),
                                "payload_keys": pkeys,
                            })
                        })
                        .collect();
                    println!(
                        "{}",
                        json_to_text(cfg.pretty, &json!({"operations": ops, "legacy": legacy}))?
                    );
                } else {
                    println!("Operations (new API):");
                    for op in Operation::value_variants() {
                        println!(
                            "  {:<35}  {}",
                            op.to_possible_value().unwrap().get_name(),
                            op.description()
                        );
                    }
                    println!("\nLegacy (ASMX):");
                    for op in OperationOld::value_variants() {
                        println!(
                            "  {:<35}  {}",
                            op.to_possible_value().unwrap().get_name(),
                            op.description()
                        );
                    }
                }
                return Ok(());
            }

            if let Some(ref name) = info {
                match find_operation(name) {
                    None => {
                        anyhow::bail!("unknown operation '{}'; use --list to see all names", name)
                    }
                    Some(AnyOperation::New(op)) => {
                        let spec = op.spec();
                        let p = op.default_payload();
                        let pkeys: Vec<&str> = if let Value::Object(ref m) = p {
                            m.keys().map(|k| k.as_str()).collect()
                        } else {
                            vec![]
                        };
                        if format == OutputFormat::Json {
                            println!(
                                "{}",
                                json_to_text(
                                    cfg.pretty,
                                    &json!({
                                        "name": op.to_possible_value().unwrap().get_name(),
                                        "api": "new",
                                        "endpoint": spec.endpoint,
                                        "referer": spec.referer,
                                        "description": op.description(),
                                        "payload_keys": pkeys,
                                    })
                                )?
                            );
                        } else {
                            println!(
                                "Name:         {}",
                                op.to_possible_value().unwrap().get_name()
                            );
                            println!("API:          new");
                            println!("Endpoint:     {}", spec.endpoint);
                            println!("Referer:      {}", spec.referer);
                            println!("Description:  {}", op.description());
                            println!("Payload keys: {}", pkeys.join(", "));
                        }
                    }
                    Some(AnyOperation::Old(op)) => {
                        let spec = op.spec();
                        let p = op.default_payload();
                        let pkeys: Vec<&str> = if let Value::Object(ref m) = p {
                            m.keys().map(|k| k.as_str()).collect()
                        } else {
                            vec![]
                        };
                        if format == OutputFormat::Json {
                            println!(
                                "{}",
                                json_to_text(
                                    cfg.pretty,
                                    &json!({
                                        "name": op.to_possible_value().unwrap().get_name(),
                                        "api": "legacy",
                                        "endpoint": spec.endpoint,
                                        "referer": spec.referer,
                                        "description": op.description(),
                                        "payload_keys": pkeys,
                                    })
                                )?
                            );
                        } else {
                            println!(
                                "Name:         {}",
                                op.to_possible_value().unwrap().get_name()
                            );
                            println!("API:          legacy");
                            println!("Endpoint:     {}", spec.endpoint);
                            println!("Referer:      {}", spec.referer);
                            println!("Description:  {}", op.description());
                            println!("Payload keys: {}", pkeys.join(", "));
                        }
                    }
                }
                return Ok(());
            }

            let client = get_client(&cfg).await?;
            let mut operation_names: Vec<QueryOperationName> = operation
                .iter()
                .map(|op| QueryOperationName::new(op.to_possible_value().unwrap().get_name()))
                .collect();
            operation_names.extend(
                old.iter()
                    .map(|op| QueryOperationName::new(op.to_possible_value().unwrap().get_name())),
            );
            let query_plan = build_query_batch_plan(QueryBatchRequest::new(
                operation_names,
                Some(network_concurrency),
            ));
            let set_overrides: Vec<(String, Value)> = set
                .iter()
                .map(|kv| parse_set_pair(kv))
                .collect::<anyhow::Result<_>>()?;
            let custom_payload: Option<Value> = payload
                .as_ref()
                .map(|s| serde_json::from_str::<Value>(s))
                .transpose()?;
            let base_url = cfg.normalized_base_url().to_string();

            // Build jobs with deduplication: same operation names are not queried multiple times
            let mut jobs: Vec<QueryJob> = Vec::with_capacity(operation.len() + old.len());
            let mut seen_names: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut job_idx = 0;

            for op in operation.into_iter() {
                let name = op.to_possible_value().unwrap().get_name().to_string();
                if !seen_names.insert(name.clone()) {
                    continue; // Skip duplicate operation
                }
                let spec = op.spec();
                let url = if spec.endpoint.starts_with("http") {
                    spec.endpoint.to_string()
                } else {
                    format!("{}{}", base_url, spec.endpoint)
                };
                jobs.push(QueryJob {
                    idx: job_idx,
                    name,
                    url,
                    referer: spec.referer,
                    default_payload: op
                        .default_payload_typed()
                        .unwrap_or_else(|| op.default_payload()),
                });
                job_idx += 1;
            }
            for op in old.into_iter() {
                let name = op.to_possible_value().unwrap().get_name().to_string();
                if !seen_names.insert(name.clone()) {
                    continue; // Skip duplicate operation
                }
                let spec = op.spec();
                jobs.push(QueryJob {
                    idx: job_idx,
                    name,
                    url: format!("{}{}", base_url, spec.endpoint),
                    referer: spec.referer,
                    default_payload: op.default_payload(),
                });
                job_idx += 1;
            }

            let ordered_results = run_query_batch(
                &client,
                jobs,
                query_plan.concurrency,
                set_overrides,
                custom_payload,
            )
            .await?;

            let mut merged = serde_json::Map::new();
            for (_, name, res) in ordered_results {
                merged.insert(name, res);
            }

            let final_val = Value::Object(merged);
            if format == OutputFormat::Humanize
                && final_val.as_object().map(|m| m.len()).unwrap_or(0) == 1
            {
                if let Some(raw) = final_val
                    .as_object()
                    .and_then(|map| map.values().next())
                {
                    let normalized = tefas::normalize_fund_summary_list(raw);
                    if normalized.is_empty() {
                        println!("{}", json_to_text(cfg.pretty, &final_val)?);
                    } else {
                        for item in normalized {
                            println!("{}", item.human_line());
                        }
                    }
                } else {
                    println!("{}", json_to_text(cfg.pretty, &final_val)?);
                }
            } else {
                println!("{}", json_to_text(cfg.pretty, &final_val)?);
            }
        }

        Commands::Fetch {
            urls,
            output,
            skip_preflight,
        } => {
            let out_plan = resolve_fetch_outputs(&urls, output)?;

            let mut global_mod = cli.global.clone();
            if skip_preflight {
                global_mod.quiet = true;
            }
            let mut cfg = build_app_config(&global_mod, true);
            cfg.auth.skip_preflight = skip_preflight;
            let network_concurrency =
                effective_network_concurrency(&cli.global, default_request_concurrency(cfg.backend));

            let client = get_client(&cfg).await?;
            let fetch_plan = build_fetch_batch_plan(FetchBatchRequest::new(
                urls.clone(),
                Some(network_concurrency),
                default_request_concurrency(cfg.backend),
            ));

            let fetch_results =
                run_fetch_batch(&client, urls, fetch_plan.concurrency, cli.global.quiet).await;

            // Collect results in original URL order
            match out_plan {
                FetchOutputPlan::DefaultSingle => {
                    let (url, body) = fetch_results.into_iter().next().unwrap_or_else(|| {
                        (
                            "(none)".to_string(),
                            Err(anyhow::anyhow!("no URLs provided")),
                        )
                    });
                    match body {
                        Ok(text) => {
                            fs::write("tefas_fetched.html", text)?;
                            if !cli.global.quiet {
                                println!("Fetched {} → tefas_fetched.html", url);
                            }
                        }
                        Err(e) => {
                            if !cli.global.quiet {
                                eprintln!("Error fetching {}: {}", url, e);
                            }
                        }
                    }
                }
                FetchOutputPlan::PerUrl(plan) => {
                    for ((url, body), (_, dest)) in fetch_results.into_iter().zip(plan.into_iter())
                    {
                        match body {
                            Ok(text) => {
                                if let Some(p) = dest.parent()
                                    && !p.as_os_str().is_empty()
                                {
                                    fs::create_dir_all(p)?;
                                }
                                fs::write(&dest, text)?;
                                if !cli.global.quiet {
                                    println!("Fetched {} → {}", url, dest.display());
                                }
                            }
                            Err(e) => {
                                if !cli.global.quiet {
                                    eprintln!("Error fetching {}: {}", url, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Commands::Parse {
            inputs,
            output,
            fields,
        } => {
            let mut results = serde_json::Map::new();
            let concurrency = effective_parse_concurrency(&cli.global);
            let permits = Arc::new(Semaphore::new(concurrency));
            let handles: Vec<_> = inputs
                .into_iter()
                .map(|input| {
                    let permits = permits.clone();
                    tokio::spawn(async move {
                        let _permit = permits
                            .acquire_owned()
                            .await
                            .map_err(|e| anyhow::anyhow!("concurrency semaphore closed: {e}"))?;
                        tokio::task::spawn_blocking(move || -> anyhow::Result<(String, Value)> {
                            let resolved = resolve_parse_input_path(&input)?;
                            let html = fs::read_to_string(&resolved).map_err(|e| {
                                anyhow::anyhow!(
                                    "Failed to read {} (resolved from {}): {}",
                                    resolved.display(),
                                    input,
                                    e
                                )
                            })?;
                            let (grouped, _) = parse_document(&html);
                            let payload = if fields {
                                field_paths_json(&grouped)
                            } else {
                                grouped
                            };
                            Ok((input, payload))
                        })
                        .await?
                    })
                })
                .collect();

            for h in handles {
                let (input, grouped) = h.await??;
                results.insert(input, grouped);
            }
            let content = json_to_text(cli.global.pretty, &Value::Object(results))?;
            write_or_stdout(output.as_deref(), &content)?;
        }

        Commands::Logo {
            codes,
            outdir,
            format,
            quality,
        } => {
            let cfg = build_app_config(&cli.global, false);
            let client = get_client(&cfg).await?;
            let spec = Operation::GetLogo.spec();
            let payload = json!({"memberCodes": codes});
            let url = format!("{}{}", cfg.normalized_base_url(), spec.endpoint);

            let res = client
                .post_json_with_referer(&url, spec.referer, &payload)
                .await?;
            let stats = convert_getlogo_to_images(
                &res,
                &outdir,
                format.into(),
                quality,
                !cli.global.quiet,
            )?;

            if !cli.global.quiet {
                eprintln!(
                    "Logo conversion: wrote={} skipped={} errors={}",
                    stats.wrote, stats.skipped, stats.errors
                );
            }
        }

        Commands::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "tefas", &mut io::stdout());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
