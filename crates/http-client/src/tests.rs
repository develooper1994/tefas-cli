use super::*;
use tefas_config::{AppConfig, HttpBackend, TlsBackend, TlsConfig};

#[test]
fn parse_cookie_file_empty() {
    // Write a temp file that is empty
    let dir = std::env::temp_dir();
    let path = dir.join("tefas_test_cookies_empty.txt");
    std::fs::write(&path, "").unwrap();
    assert_eq!(NetworkClient::parse_cookie_file(path.to_str().unwrap()), "");
}

#[test]
fn parse_cookie_file_comments_only() {
    let dir = std::env::temp_dir();
    let path = dir.join("tefas_test_cookies_comments.txt");
    std::fs::write(&path, "# Netscape HTTP Cookie File\n# comment\n").unwrap();
    assert_eq!(NetworkClient::parse_cookie_file(path.to_str().unwrap()), "");
}

#[test]
fn parse_cookie_file_simple_kv() {
    let dir = std::env::temp_dir();
    let path = dir.join("tefas_test_cookies_kv.txt");
    std::fs::write(&path, "SESSION=abc123\nFOO=bar\n").unwrap();
    assert_eq!(
        NetworkClient::parse_cookie_file(path.to_str().unwrap()),
        "SESSION=abc123; FOO=bar"
    );
}

#[test]
fn parse_cookie_file_netscape_format() {
    let dir = std::env::temp_dir();
    let path = dir.join("tefas_test_cookies_netscape.txt");
    // Netscape format: domain, flag, path, secure, expiry, name, value
    std::fs::write(
        &path,
        ".tefas.gov.tr\tTRUE\t/\tFALSE\t1999999999\tSESSION\tXYZ\n",
    )
    .unwrap();
    assert_eq!(
        NetworkClient::parse_cookie_file(path.to_str().unwrap()),
        "SESSION=XYZ"
    );
}

#[test]
fn build_reqwest_rustls_succeeds() {
    let cfg = AppConfig::with_defaults(); // backend = Reqwest, tls = Rustls
    assert!(NetworkClient::new(&cfg).is_ok());
}

#[cfg(feature = "native-tls")]
#[test]
fn build_reqwest_native_tls_succeeds() {
    let cfg = AppConfig {
        backend: HttpBackend::Reqwest,
        tls: TlsConfig {
            backend: TlsBackend::NativeTls,
            timeout_ms: None,
        },
        ..AppConfig::with_defaults()
    };
    assert!(NetworkClient::new(&cfg).is_ok());
}

#[cfg(not(feature = "native-tls"))]
#[test]
fn build_reqwest_native_tls_errors_without_feature() {
    let cfg = AppConfig {
        backend: HttpBackend::Reqwest,
        tls: TlsConfig {
            backend: TlsBackend::NativeTls,
            timeout_ms: None,
        },
        ..AppConfig::with_defaults()
    };
    assert!(NetworkClient::new(&cfg).is_err());
}

#[cfg(not(feature = "native-tls"))]
#[test]
fn build_hyper_native_tls_errors_without_feature() {
    let cfg = AppConfig {
        backend: HttpBackend::Hyper,
        tls: TlsConfig {
            backend: TlsBackend::NativeTls,
            timeout_ms: None,
        },
        ..AppConfig::with_defaults()
    };
    assert!(NetworkClient::new(&cfg).is_err());
}

#[test]
fn build_hyper_insecure_errors() {
    let cfg = AppConfig {
        backend: HttpBackend::Hyper,
        tls: TlsConfig {
            backend: TlsBackend::Insecure,
            timeout_ms: None,
        },
        ..AppConfig::with_defaults()
    };
    assert!(NetworkClient::new(&cfg).is_err());
}

#[test]
fn impcurl_build_default_tls_succeeds() {
    let cfg = AppConfig {
        backend: HttpBackend::Impcurl,
        ..AppConfig::with_defaults()
    };
    assert!(NetworkClient::new(&cfg).is_ok());
}

#[test]
fn impcurl_unknown_impersonate_detects_curl_style_error() {
    let stderr = "curl: option --impersonate: is unknown";
    assert!(impcurl::unknown_impersonate(stderr));
}

#[test]
fn impcurl_unknown_impersonate_detects_unrecognized_option_error() {
    let stderr = "error: unrecognized option '--impersonate'";
    assert!(impcurl::unknown_impersonate(stderr));
}

#[test]
fn impcurl_unknown_impersonate_ignores_other_errors() {
    let stderr = "curl: (6) Could not resolve host: example.invalid";
    assert!(!impcurl::unknown_impersonate(stderr));
}

// --- postback helpers ---

#[test]
fn extract_hidden_fields_double_quoted() {
    let html = r#"<form><input type="hidden" name="__VIEWSTATE" value="abc123==" /><input type="hidden" name="__EVENTVALIDATION" value="xyz+Q==" /></form>"#;
    let fields = extract_hidden_fields(html);
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0], ("__VIEWSTATE".into(), "abc123==".into()));
    assert_eq!(fields[1], ("__EVENTVALIDATION".into(), "xyz+Q==".into()));
}

#[test]
fn extract_hidden_fields_single_quoted() {
    let html = "<input type='hidden' name='csrf' value='tok123' />";
    let fields = extract_hidden_fields(html);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0], ("csrf".into(), "tok123".into()));
}

#[test]
fn extract_hidden_fields_skips_non_hidden() {
    let html = r#"<input type="text" name="user" value="me" /><input type="hidden" name="tok" value="t1" />"#;
    let fields = extract_hidden_fields(html);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].0, "tok");
}

#[test]
fn extract_hidden_fields_empty_value() {
    let html = r#"<input type="hidden" name="__VIEWSTATEGENERATOR" value="" />"#;
    let fields = extract_hidden_fields(html);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0], ("__VIEWSTATEGENERATOR".into(), "".into()));
}

#[test]
fn form_urlencode_basic() {
    let fields = vec![("key".into(), "value".into()), ("a".into(), "b c".into())];
    let encoded = String::from_utf8(form_urlencode(&fields)).unwrap();
    assert_eq!(encoded, "key=value&a=b+c");
}

#[test]
fn form_urlencode_special_chars() {
    let fields = vec![("vs".into(), "a+b/c=d".into())];
    let encoded = String::from_utf8(form_urlencode(&fields)).unwrap();
    // + → %2B, / → %2F, = → %3D
    assert_eq!(encoded, "vs=a%2Bb%2Fc%3Dd");
}

#[test]
fn form_urlencode_empty() {
    let encoded = String::from_utf8(form_urlencode(&[])).unwrap();
    assert_eq!(encoded, "");
}

// --- header-order normalization (#13) ---

fn make_client_for_header_test() -> NetworkClient {
    use tefas_config::{AuthConfig, RetryConfig, TlsConfig};
    let app_cfg = AppConfig {
        backend: HttpBackend::Reqwest,
        tls: TlsConfig::default(),
        auth: AuthConfig::default(),
        retry: RetryConfig::default(),
        ..AppConfig::with_defaults()
    };
    let cfg = NetworkClientConfig::from(&app_cfg);
    let base_headers = NetworkClient::build_base_headers(&cfg).unwrap();

    NetworkClient {
        inner: Arc::new(NetworkClientInner {
            backend: BackendClient::Reqwest(reqwest::Client::new()),
            cfg,
            base_headers,
        }),
    }
}

#[test]
fn headers_contain_required_headers() {
    let client = make_client_for_header_test();
    let headers = client.headers().unwrap();

    // Core headers required for WAF bypass: tool UA + standard negotiation headers.
    // Browser-specific headers (Sec-CH-UA, Sec-Fetch-*, Upgrade-Insecure-Requests) are
    // intentionally absent — they would be inconsistent with a non-browser UA and
    // could trigger the F5/TSPD WAF JS challenge.
    assert!(headers.contains_key(reqwest::header::USER_AGENT));
    assert!(headers.contains_key(reqwest::header::ACCEPT));
    assert!(headers.contains_key(reqwest::header::ACCEPT_LANGUAGE));
    assert!(headers.contains_key(reqwest::header::ACCEPT_ENCODING));
    assert!(
        !headers.contains_key(HeaderName::from_static("sec-ch-ua")),
        "sec-ch-ua must not be present with a non-browser UA"
    );
    assert!(
        !headers.contains_key(HeaderName::from_static("sec-fetch-site")),
        "sec-fetch-site must not be present with a non-browser UA"
    );
}

#[test]
fn headers_order() {
    let client = make_client_for_header_test();
    let headers = client.headers().unwrap();

    // Collect header names in insertion order
    let names: Vec<&str> = headers.keys().map(|k| k.as_str()).collect();

    // UA must come before Accept-Encoding
    let ua_pos = names.iter().position(|&n| n == "user-agent").unwrap();
    let enc_pos = names.iter().position(|&n| n == "accept-encoding").unwrap();

    assert!(
        ua_pos < enc_pos,
        "user-agent should precede accept-encoding"
    );
}

#[test]
fn headers_accept_encoding_value() {
    let client = make_client_for_header_test();
    let headers = client.headers().unwrap();
    let enc = headers
        .get(reqwest::header::ACCEPT_ENCODING)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(enc.contains("gzip"));
    assert!(enc.contains("br"));
}

#[test]
fn compute_sec_fetch_site_same_origin() {
    let site = NetworkClient::compute_sec_fetch_site(
        "https://www.tefas.gov.tr/tr/fon-detayli-analiz/ADE",
        Some("https://www.tefas.gov.tr/tr/fon-getirileri"),
    );
    assert_eq!(site, "same-origin");
}

#[test]
fn compute_sec_fetch_site_none_without_referer() {
    let site = NetworkClient::compute_sec_fetch_site(
        "https://www.tefas.gov.tr/tr/fon-detayli-analiz/ADE",
        None,
    );
    assert_eq!(site, "none");
}

#[test]
fn request_profile_navigation_vs_api_fields() {
    let client = make_client_for_header_test();

    let nav_site = NetworkClient::compute_sec_fetch_site(
        "https://www.tefas.gov.tr/tr/fon-detayli-analiz/ADE",
        Some("https://www.tefas.gov.tr/tr/fon-getirileri"),
    );
    assert_eq!(nav_site, "same-origin");

    let mut nav_headers = client.headers().unwrap();
    nav_headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static(NetworkClient::NAVIGATION_ACCEPT),
    );
    nav_headers.insert(
        HeaderName::from_static("sec-fetch-mode"),
        HeaderValue::from_static("navigate"),
    );
    nav_headers.insert(
        HeaderName::from_static("sec-fetch-dest"),
        HeaderValue::from_static("document"),
    );
    nav_headers.insert(
        HeaderName::from_static("sec-fetch-user"),
        HeaderValue::from_static("?1"),
    );
    assert!(nav_headers.contains_key("sec-fetch-user"));
    assert_eq!(
        nav_headers
            .get(reqwest::header::ACCEPT)
            .unwrap()
            .to_str()
            .unwrap(),
        NetworkClient::NAVIGATION_ACCEPT
    );

    let mut api_headers = client.headers().unwrap();
    api_headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static(NetworkClient::API_ACCEPT),
    );
    api_headers.insert(
        HeaderName::from_static("sec-fetch-mode"),
        HeaderValue::from_static("cors"),
    );
    api_headers.insert(
        HeaderName::from_static("sec-fetch-dest"),
        HeaderValue::from_static("empty"),
    );
    api_headers.remove(HeaderName::from_static("sec-fetch-user"));
    assert!(!api_headers.contains_key("sec-fetch-user"));
    assert_eq!(
        api_headers
            .get(reqwest::header::ACCEPT)
            .unwrap()
            .to_str()
            .unwrap(),
        NetworkClient::API_ACCEPT
    );
}

#[test]
fn looks_like_waf_challenge_detects_request_rejected_signature() {
    let text = "<html><body>Request Rejected. Your support ID is: 12345</body></html>";
    assert!(NetworkClient::looks_like_waf_challenge(text));
}

// ── WAF bypass validation (#20) ─────────────────────────────────────────

/// The `proxy` field round-trips through `AppConfig`.
#[test]
fn proxy_field_in_appconfig() {
    let proxy = "http://127.0.0.1:8080".to_string();
    let cfg = AppConfig {
        proxy: Some(proxy.clone()),
        ..AppConfig::with_defaults()
    };
    assert_eq!(cfg.proxy.as_deref(), Some("http://127.0.0.1:8080"));
}

/// A valid proxy URL passes the reqwest client builder.
#[test]
fn build_reqwest_with_proxy_succeeds() {
    let cfg = AppConfig {
        proxy: Some("http://127.0.0.1:8080".to_string()),
        ..AppConfig::with_defaults()
    };
    assert!(NetworkClient::new(&cfg).is_ok());
}

/// An invalid proxy URL is rejected at client-build time.
#[test]
fn build_reqwest_with_invalid_proxy_fails() {
    // reqwest validates the proxy scheme: an empty string has no scheme and is rejected.
    let cfg = AppConfig {
        proxy: Some(String::new()),
        ..AppConfig::with_defaults()
    };
    assert!(NetworkClient::new(&cfg).is_err());
}

/// `proxy: None` (the default) builds the client without errors.
#[test]
fn build_reqwest_without_proxy_succeeds() {
    let cfg = AppConfig::with_defaults();
    assert!(cfg.proxy.is_none());
    assert!(NetworkClient::new(&cfg).is_ok());
}

/// Browser-only headers must be absent to avoid triggering the F5/TSPD WAF JS challenge.
/// The WAF only issues challenges to browser UAs; non-browser UAs pass through cleanly.
#[test]
fn browser_only_headers_absent() {
    let client = make_client_for_header_test();
    let headers = client.headers().unwrap();
    assert!(
        !headers.contains_key("sec-ch-ua"),
        "sec-ch-ua must not be sent — inconsistent with tool UA and triggers WAF"
    );
    assert!(
        !headers.contains_key("upgrade-insecure-requests"),
        "upgrade-insecure-requests must not be sent with a non-browser UA"
    );
}
