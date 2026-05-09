use super::*;
use crate::routing::url_to_filename;
use clap::ValueEnum;
use std::path::PathBuf;

// ── parse_set_pair ──────────────────────────────────────────────────────

#[test]
fn test_parse_set_pair_string_value() {
    let (k, v) = parse_set_pair("foo=bar").unwrap();
    assert_eq!(k, "foo");
    assert_eq!(v, json!("bar"));
}

#[test]
fn test_parse_set_pair_numeric_value() {
    let (k, v) = parse_set_pair("count=42").unwrap();
    assert_eq!(k, "count");
    assert_eq!(v, json!(42));
}

#[test]
fn test_parse_set_pair_bool_value() {
    let (k, v) = parse_set_pair("flag=true").unwrap();
    assert_eq!(k, "flag");
    assert_eq!(v, json!(true));

    let (k, v) = parse_set_pair("flag=false").unwrap();
    assert_eq!(k, "flag");
    assert_eq!(v, json!(false));
}

#[test]
fn test_parse_set_pair_null_value() {
    let (k, v) = parse_set_pair("field=null").unwrap();
    assert_eq!(k, "field");
    assert_eq!(v, json!(null));
}

#[test]
fn test_parse_set_pair_json_object_value() {
    let (k, v) = parse_set_pair(r#"filter={"x":1}"#).unwrap();
    assert_eq!(k, "filter");
    assert_eq!(v, json!({"x": 1}));
}

#[test]
fn test_parse_set_pair_value_contains_equals() {
    // splitn(2, '=') must keep the rest of the string as value
    let (k, v) = parse_set_pair("url=http://x.com/a=b").unwrap();
    assert_eq!(k, "url");
    assert_eq!(v, json!("http://x.com/a=b"));
}

#[test]
fn test_parse_set_pair_empty_key_is_error() {
    assert!(parse_set_pair("=value").is_err());
}

#[test]
fn test_parse_set_pair_empty_value_is_error() {
    assert!(parse_set_pair("key=").is_err());
}

#[test]
fn test_parse_set_pair_no_separator_is_error() {
    assert!(parse_set_pair("keyonly").is_err());
}

// ── persona_defaults ────────────────────────────────────────────────────

#[test]
fn test_persona_defaults_desktop_windows() {
    let (ua, imp) = persona_defaults(PersonaPreset::DesktopWindows);
    assert!(
        ua.contains("Windows NT 10.0"),
        "UA should contain Windows NT 10.0, got: {ua}"
    );
    assert_eq!(imp, "chrome136");
}

#[test]
fn test_persona_defaults_desktop_macos() {
    let (ua, imp) = persona_defaults(PersonaPreset::DesktopMacos);
    assert!(
        ua.contains("Macintosh"),
        "UA should contain Macintosh, got: {ua}"
    );
    assert_eq!(imp, "chrome136");
}

#[test]
fn test_persona_defaults_android() {
    let (ua, imp) = persona_defaults(PersonaPreset::Android);
    assert!(
        ua.contains("Android"),
        "UA should contain Android, got: {ua}"
    );
    assert_eq!(imp, "chrome136_android");
}

#[test]
fn test_persona_defaults_ios() {
    let (ua, imp) = persona_defaults(PersonaPreset::Ios);
    assert!(ua.contains("iPhone"), "UA should contain iPhone, got: {ua}");
    assert_eq!(imp, "safari17_2_ios");
}

// ── json_to_text ────────────────────────────────────────────────────────

#[test]
fn test_json_to_text_pretty() {
    let v = json!({"a": 1, "b": [2, 3]});
    let s = json_to_text(true, &v).unwrap();
    assert!(s.contains('\n'), "pretty output should contain newlines");
}

#[test]
fn test_json_to_text_compact() {
    let v = json!({"a": 1, "b": [2, 3]});
    let s = json_to_text(false, &v).unwrap();
    assert!(
        !s.contains('\n'),
        "compact output should not contain newlines"
    );
}

#[test]
fn test_json_to_text_roundtrip() {
    let original = json!({"key": "val", "n": 42, "arr": [true, null]});
    let s = json_to_text(false, &original).unwrap();
    let parsed: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(original, parsed);
}

// ── build_app_config ────────────────────────────────────────────────────

fn make_global(backend: BackendChoice) -> GlobalArgs {
    GlobalArgs {
        timeout: 25,
        pretty: true,
        base_url: "https://www.tefas.gov.tr".to_string(),
        backend,
        tls: BackendTls::Rustls,
        persona: None,
        impersonate: None,
        proxy: None,
        network_concurrency: None,
        parse_concurrency: None,
        concurrency: None,
        quiet: false,
    }
}

#[test]
fn test_effective_concurrency_uses_requested_value() {
    assert_eq!(effective_concurrency(Some(8), 4), 8);
}

#[test]
fn test_effective_concurrency_clamps_zero_to_one() {
    assert_eq!(effective_concurrency(Some(0), 4), 1);
}

#[test]
fn test_effective_parse_concurrency_prefers_specific_flag() {
    let mut global = make_global(BackendChoice::Reqwest);
    global.concurrency = Some(9);
    global.parse_concurrency = Some(2);
    assert_eq!(effective_parse_concurrency(&global), 2);
}

#[test]
fn test_default_request_concurrency_is_conservative_for_impcurl() {
    assert_eq!(default_request_concurrency(HttpBackend::Impcurl), 2);
}

#[test]
fn test_build_app_config_no_auto_waf_preserves_backend() {
    let global = make_global(BackendChoice::Reqwest);
    let cfg = build_app_config(&global, false);
    assert_eq!(cfg.backend, HttpBackend::Reqwest);
}

#[test]
fn test_build_app_config_hyper_backend() {
    let global = make_global(BackendChoice::Hyper);
    let cfg = build_app_config(&global, false);
    assert_eq!(cfg.backend, HttpBackend::Hyper);
}

#[test]
fn test_build_app_config_impcurl_backend() {
    let global = make_global(BackendChoice::Impcurl);
    let cfg = build_app_config(&global, false);
    assert_eq!(cfg.backend, HttpBackend::Impcurl);
}

#[test]
fn test_build_app_config_persona_sets_ua_and_imp() {
    let mut global = make_global(BackendChoice::Wreq);
    global.persona = Some(PersonaPreset::DesktopWindows);
    let cfg = build_app_config(&global, false);
    assert!(cfg.auth.user_agent.contains("Windows NT 10.0"));
    assert_eq!(cfg.auth.impcurl_impersonate.as_deref(), Some("chrome136"));
}

#[test]
fn test_build_app_config_explicit_impersonate_not_overridden_by_persona() {
    let mut global = make_global(BackendChoice::Wreq);
    global.persona = Some(PersonaPreset::DesktopWindows);
    global.impersonate = Some("chrome112".to_string());
    let cfg = build_app_config(&global, false);
    // Explicit --impersonate takes priority over persona's default imp profile
    assert_eq!(cfg.auth.impcurl_impersonate.as_deref(), Some("chrome112"));
}

#[test]
fn test_build_app_config_timeout_and_proxy() {
    let mut global = make_global(BackendChoice::Wreq);
    global.timeout = 60;
    global.proxy = Some("socks5://127.0.0.1:1080".to_string());
    let cfg = build_app_config(&global, false);
    assert_eq!(cfg.timeout_secs, 60);
    assert_eq!(cfg.proxy.as_deref(), Some("socks5://127.0.0.1:1080"));
}

// ── CLI argument parsing ────────────────────────────────────────────────

#[test]
fn test_cli_fundpage_basic() {
    let cli = Cli::try_parse_from(["tefas", "fundpage", "AC5"]).unwrap();
    match cli.command {
        Commands::Fundpage { codes, output, .. } => {
            assert_eq!(codes, vec!["AC5"]);
            assert!(output.is_none());
        }
        _ => panic!("expected Fundpage command"),
    }
}

#[test]
fn test_cli_fundpage_multiple() {
    let cli = Cli::try_parse_from(["tefas", "fundpage", "AC5", "TLY"]).unwrap();
    match cli.command {
        Commands::Fundpage { codes, .. } => {
            assert_eq!(codes, vec!["AC5", "TLY"]);
        }
        _ => panic!("expected Fundpage command"),
    }
}

#[test]
fn test_cli_fundpage_with_concurrency() {
    let cli = Cli::try_parse_from(["tefas", "--concurrency", "6", "fundpage", "AC5", "TLY"])
        .unwrap();
    assert_eq!(cli.global.concurrency, Some(6));
}

#[test]
fn test_cli_fundpage_with_network_concurrency() {
    let cli =
        Cli::try_parse_from(["tefas", "--network-concurrency", "5", "fundpage", "AC5"])
            .unwrap();
    assert_eq!(cli.global.network_concurrency, Some(5));
}

#[test]
fn test_cli_query_single_operation() {
    let cli = Cli::try_parse_from(["tefas", "query", "fonBilgiGetir"]).unwrap();
    match cli.command {
        Commands::Query {
            operation,
            old,
            set,
            payload,
            format,
            ..
        } => {
            assert_eq!(operation.len(), 1);
            assert!(old.is_empty());
            assert!(set.is_empty());
            assert!(payload.is_none());
            assert_eq!(format, OutputFormat::Json);
        }
        _ => panic!("expected Query command"),
    }
}

#[test]
fn test_cli_fetch_basic() {
    let cli = Cli::try_parse_from(["tefas", "fetch", "https://www.tefas.gov.tr"]).unwrap();
    match cli.command {
        Commands::Fetch {
            urls,
            output,
            skip_preflight,
        } => {
            assert_eq!(urls, vec!["https://www.tefas.gov.tr"]);
            assert!(output.is_none());
            assert!(!skip_preflight);
        }
        _ => panic!("expected Fetch command"),
    }
}

#[test]
fn test_cli_parse_with_concurrency() {
    let cli = Cli::try_parse_from(["tefas", "--concurrency", "3", "parse", "TAR.html"])
        .unwrap();
    assert_eq!(cli.global.concurrency, Some(3));
}

#[test]
fn test_cli_parse_with_parse_concurrency() {
    let cli =
        Cli::try_parse_from(["tefas", "--parse-concurrency", "7", "parse", "TAR.html"])
            .unwrap();
    assert_eq!(cli.global.parse_concurrency, Some(7));
}

#[test]
fn test_cli_fetch_multi_url() {
    let cli = Cli::try_parse_from([
        "tefas",
        "fetch",
        "https://www.tefas.gov.tr/tr/fon-detayli-analiz/KHA",
        "https://www.tefas.gov.tr/tr/fon-detayli-analiz/TLY",
    ])
    .unwrap();
    match cli.command {
        Commands::Fetch { urls, output, .. } => {
            assert_eq!(urls.len(), 2);
            assert!(output.is_none());
        }
        _ => panic!("expected Fetch command"),
    }
}

#[test]
fn test_cli_fetch_output_dir() {
    let cli = Cli::try_parse_from([
        "tefas",
        "fetch",
        "https://www.tefas.gov.tr",
        "--output",
        "./dump",
    ])
    .unwrap();
    match cli.command {
        Commands::Fetch { output, .. } => {
            assert_eq!(output, Some(vec!["./dump".to_string()]));
        }
        _ => panic!("expected Fetch command"),
    }
}

#[test]
fn test_cli_fetch_output_flag_only() {
    // --output with no args → Some([])
    let cli =
        Cli::try_parse_from(["tefas", "fetch", "https://www.tefas.gov.tr", "--output"]).unwrap();
    match cli.command {
        Commands::Fetch { output, .. } => {
            assert_eq!(output, Some(vec![]));
        }
        _ => panic!("expected Fetch command"),
    }
}

// ── fundpage output routing ─────────────────────────────────────────────

#[test]
fn test_cli_fundpage_output_flag_only() {
    let cli = Cli::try_parse_from(["tefas", "fundpage", "KHA", "--output"]).unwrap();
    match cli.command {
        Commands::Fundpage { output, .. } => {
            assert_eq!(output, Some(vec![]));
        }
        _ => panic!("expected Fundpage command"),
    }
}

#[test]
fn test_cli_fundpage_save_html_flag_only() {
    let cli = Cli::try_parse_from(["tefas", "fundpage", "KHA", "--save-html"]).unwrap();
    match cli.command {
        Commands::Fundpage { save_html, .. } => {
            assert_eq!(save_html, Some(vec![]));
        }
        _ => panic!("expected Fundpage command"),
    }
}

#[test]
fn test_cli_fundpage_save_html_dir() {
    let cli = Cli::try_parse_from(["tefas", "fundpage", "KHA", "--save-html", "./raw"]).unwrap();
    match cli.command {
        Commands::Fundpage { save_html, .. } => {
            assert_eq!(save_html, Some(vec!["./raw".to_string()]));
        }
        _ => panic!("expected Fundpage command"),
    }
}

#[test]
fn test_cli_fundpage_fields_flag() {
    let cli = Cli::try_parse_from(["tefas", "fundpage", "KHA", "--fields"]).unwrap();
    match cli.command {
        Commands::Fundpage { fields, .. } => {
            assert!(fields);
        }
        _ => panic!("expected Fundpage command"),
    }
}

#[test]
fn test_resolve_fundpage_outputs_stdout() {
    // None + 1 code → Stdout
    let plan = resolve_fundpage_outputs(&["KHA".to_string()], None).unwrap();
    assert!(matches!(plan, FundpageOutputPlan::Stdout));
}

#[test]
fn test_resolve_fundpage_outputs_per_code_multi_none() {
    // None + multi codes → PerCode with auto names
    let plan = resolve_fundpage_outputs(&["KHA".to_string(), "TLY".to_string()], None).unwrap();
    match plan {
        FundpageOutputPlan::PerCode(pairs) => {
            assert_eq!(pairs.len(), 2);
            assert_eq!(pairs[0].0, "KHA");
            assert_eq!(pairs[0].1, PathBuf::from("KHA.json"));
            assert_eq!(pairs[1].0, "TLY");
        }
        _ => panic!("expected PerCode"),
    }
}

#[test]
fn test_resolve_fundpage_outputs_flag_only_single() {
    // Some([]) + 1 code → PerCode (not Stdout)
    let plan = resolve_fundpage_outputs(&["KHA".to_string()], Some(vec![])).unwrap();
    assert!(matches!(plan, FundpageOutputPlan::PerCode(_)));
}

#[test]
fn test_resolve_fundpage_outputs_merged_file() {
    // Some([file]) → MergedFile
    let plan = resolve_fundpage_outputs(
        &["KHA".to_string(), "TLY".to_string()],
        Some(vec!["merged.json".to_string()]),
    )
    .unwrap();
    assert!(matches!(plan, FundpageOutputPlan::MergedFile(_)));
}

#[test]
fn test_resolve_fundpage_outputs_n_paths_n_codes() {
    // N paths == N codes → PerCode with given paths
    let plan = resolve_fundpage_outputs(
        &["KHA".to_string(), "TLY".to_string()],
        Some(vec!["a.json".to_string(), "b.json".to_string()]),
    )
    .unwrap();
    match plan {
        FundpageOutputPlan::PerCode(pairs) => {
            assert_eq!(pairs[0].1, PathBuf::from("a.json"));
            assert_eq!(pairs[1].1, PathBuf::from("b.json"));
        }
        _ => panic!("expected PerCode"),
    }
}

#[test]
fn test_resolve_fundpage_outputs_bad_count_is_error() {
    // 3 paths for 2 codes → error
    let result = resolve_fundpage_outputs(
        &["KHA".to_string(), "TLY".to_string()],
        Some(vec![
            "a.json".to_string(),
            "b.json".to_string(),
            "c.json".to_string(),
        ]),
    );
    assert!(result.is_err());
}

// ── url_to_filename ─────────────────────────────────────────────────────

#[test]
fn test_url_to_filename_last_segment() {
    assert_eq!(
        url_to_filename("https://example.com/tr/fund-page", 0),
        "fund-page.html"
    );
}

#[test]
fn test_url_to_filename_with_html_extension() {
    assert_eq!(
        url_to_filename("https://example.com/page.html", 0),
        "page.html"
    );
}

#[test]
fn test_url_to_filename_strips_query() {
    assert_eq!(
        url_to_filename("https://example.com/page?foo=bar", 0),
        "page.html"
    );
}

#[test]
fn test_url_to_filename_empty_path_fallback() {
    // Truly empty last segment (bare slash) → fallback to fetched_N.html
    assert_eq!(url_to_filename("/", 3), "fetched_3.html");
}

#[test]
fn test_url_to_filename_sanitizes_special_chars() {
    let name = url_to_filename("https://example.com/foo:bar", 0);
    assert!(!name.contains(':'));
    assert!(name.ends_with(".html"));
}

// ── resolve_fetch_outputs ───────────────────────────────────────────────

#[test]
fn test_resolve_fetch_outputs_single_no_output() {
    let plan = resolve_fetch_outputs(&["https://a.com".to_string()], None).unwrap();
    assert!(matches!(plan, FetchOutputPlan::DefaultSingle));
}

#[test]
fn test_resolve_fetch_outputs_multi_no_output() {
    let plan = resolve_fetch_outputs(
        &["https://a.com".to_string(), "https://b.com".to_string()],
        None,
    )
    .unwrap();
    assert!(matches!(plan, FetchOutputPlan::PerUrl(_)));
}

#[test]
fn test_resolve_fetch_outputs_one_path_is_dir() {
    let plan = resolve_fetch_outputs(
        &[
            "https://a.com/page".to_string(),
            "https://b.com/page".to_string(),
        ],
        Some(vec!["./out".to_string()]),
    )
    .unwrap();
    match plan {
        FetchOutputPlan::PerUrl(pairs) => {
            assert!(pairs[0].1.starts_with("./out"));
        }
        _ => panic!("expected PerUrl"),
    }
}

#[test]
fn test_resolve_fetch_outputs_n_paths_n_urls() {
    let plan = resolve_fetch_outputs(
        &["https://a.com".to_string(), "https://b.com".to_string()],
        Some(vec!["a.html".to_string(), "b.html".to_string()]),
    )
    .unwrap();
    match plan {
        FetchOutputPlan::PerUrl(pairs) => {
            assert_eq!(pairs[0].1, PathBuf::from("a.html"));
            assert_eq!(pairs[1].1, PathBuf::from("b.html"));
        }
        _ => panic!("expected PerUrl"),
    }
}

#[test]
fn test_resolve_fetch_outputs_bad_count_is_error() {
    let result = resolve_fetch_outputs(
        &["https://a.com".to_string()],
        Some(vec!["a.html".to_string(), "b.html".to_string()]),
    );
    assert!(result.is_err());
}

// ── find_operation ──────────────────────────────────────────────────────

#[test]
fn test_find_operation_new_api() {
    assert!(matches!(
        find_operation("fonBilgiGetir"),
        Some(AnyOperation::New(_))
    ));
}

#[test]
fn test_find_operation_legacy() {
    assert!(matches!(
        find_operation("getAllFunds"),
        Some(AnyOperation::Old(_))
    ));
}

#[test]
fn test_find_operation_case_insensitive() {
    assert!(find_operation("FONBILGIGETIR").is_some());
}

#[test]
fn test_find_operation_unknown() {
    assert!(find_operation("doesNotExist").is_none());
}

#[test]
fn test_find_operation_resolves_all_new_operation_variants() {
    for &op in Operation::value_variants() {
        let name = op
            .to_possible_value()
            .expect("operation should have possible value")
            .get_name()
            .to_string();
        assert!(
            matches!(find_operation(&name), Some(AnyOperation::New(found)) if found == op),
            "expected new operation to resolve: {}",
            name
        );
    }
}

#[test]
fn test_find_operation_resolves_all_legacy_operation_variants() {
    for &op in OperationOld::value_variants() {
        let name = op
            .to_possible_value()
            .expect("legacy operation should have possible value")
            .get_name()
            .to_string();
        assert!(
            matches!(find_operation(&name), Some(AnyOperation::Old(found)) if found == op),
            "expected legacy operation to resolve: {}",
            name
        );
    }
}

#[test]
fn test_cli_query_parses_many_operations() {
    let mut args = vec!["tefas".to_string(), "query".to_string()];
    for &op in Operation::value_variants().iter().take(10) {
        let name = op
            .to_possible_value()
            .expect("operation should have possible value")
            .get_name()
            .to_string();
        args.push(name);
    }

    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Query { operation, .. } => {
            assert_eq!(operation.len(), 10);
        }
        _ => panic!("expected Query"),
    }
}

// ── query --list / --info CLI parsing ───────────────────────────────────

#[test]
fn test_cli_query_list_flag() {
    let cli = Cli::try_parse_from(["tefas", "query", "--list"]).unwrap();
    match cli.command {
        Commands::Query { list, .. } => assert!(list),
        _ => panic!("expected Query"),
    }
}

#[test]
fn test_cli_query_info_flag() {
    let cli = Cli::try_parse_from(["tefas", "query", "--info", "fonBilgiGetir"]).unwrap();
    match cli.command {
        Commands::Query { info, .. } => {
            assert_eq!(info.as_deref(), Some("fonBilgiGetir"));
        }
        _ => panic!("expected Query"),
    }
}

// ── resolve_save_html_paths ─────────────────────────────────────────────

#[test]
fn test_resolve_save_html_none() {
    let result = resolve_save_html_paths(&["KHA".to_string()], None).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_resolve_save_html_flag_only() {
    let result = resolve_save_html_paths(&["KHA".to_string()], Some(vec![])).unwrap();
    assert_eq!(result, Some(vec![PathBuf::from("KHA.html")]));
}

#[test]
fn test_resolve_save_html_one_dir() {
    let result = resolve_save_html_paths(
        &["KHA".to_string(), "TLY".to_string()],
        Some(vec!["./raw".to_string()]),
    )
    .unwrap();
    let paths = result.unwrap();
    assert_eq!(paths[0], PathBuf::from("./raw/KHA.html"));
    assert_eq!(paths[1], PathBuf::from("./raw/TLY.html"));
}

#[test]
fn test_cli_parse_basic() {
    let cli = Cli::try_parse_from(["tefas", "parse", "input.html"]).unwrap();
    match cli.command {
        Commands::Parse {
            inputs,
            output,
            fields,
        } => {
            assert_eq!(inputs, vec!["input.html"]);
            assert!(output.is_none());
            assert!(!fields);
        }
        _ => panic!("expected Parse command"),
    }
}

#[test]
fn test_cli_parse_fields_flag() {
    let cli = Cli::try_parse_from(["tefas", "parse", "input.html", "--fields"]).unwrap();
    match cli.command {
        Commands::Parse { fields, .. } => assert!(fields),
        _ => panic!("expected Parse command"),
    }
}

#[test]
fn test_resolve_parse_input_path_finds_shared_fixture_without_cwd_assumption() {
    let resolved = resolve_parse_input_path("AC5.html").unwrap();
    assert!(resolved.exists());
    assert_eq!(resolved.file_name().and_then(|name| name.to_str()), Some("AC5.html"));
    assert!(resolved.to_string_lossy().contains("datasets/tefas/fundpage/html"));
}

#[test]
fn test_field_paths_json_nested_shape() {
    let v = json!({
        "indicator": {
            "allocation": [
                {"name": "Hisse Senedi", "value": 88.68}
            ]
        },
        "profile": {
            "fon_kodu": "AC5"
        }
    });

    let fields = field_paths_json(&v);
    let arr = fields.as_array().expect("must be array");
    let names: std::collections::HashSet<&str> = arr.iter().filter_map(|x| x.as_str()).collect();

    assert!(names.contains("indicator"));
    assert!(names.contains("indicator.allocation"));
    assert!(names.contains("indicator.allocation[]"));
    assert!(names.contains("indicator.allocation[].name"));
    assert!(names.contains("indicator.allocation[].value"));
    assert!(names.contains("profile.fon_kodu"));
}

#[test]
fn test_cli_global_backend_flag() {
    let cli = Cli::try_parse_from(["tefas", "--backend", "reqwest", "fundpage", "AC5"]).unwrap();
    assert_eq!(cli.global.backend, BackendChoice::Reqwest);
}

#[test]
fn test_cli_global_quiet_flag() {
    let cli = Cli::try_parse_from(["tefas", "--quiet", "fundpage", "AC5"]).unwrap();
    assert!(cli.global.quiet);
}

#[test]
fn test_cli_global_timeout_flag() {
    let cli = Cli::try_parse_from(["tefas", "--timeout", "60", "fundpage", "AC5"]).unwrap();
    assert_eq!(cli.global.timeout, 60);
}

#[test]
fn test_cli_no_subcommand_is_error() {
    assert!(Cli::try_parse_from(["tefas"]).is_err());
}

// ── write_or_stdout ─────────────────────────────────────────────────────

#[test]
fn test_write_or_stdout_writes_to_file() {
    let dir = std::env::temp_dir();
    let path = dir.join("tefas_cli_test_write.txt");
    let path_str = path.to_str().unwrap();
    write_or_stdout(Some(path_str), "hello test").unwrap();
    let content = fs::read_to_string(path_str).unwrap();
    assert_eq!(content, "hello test");
    let _ = fs::remove_file(path_str);
}
