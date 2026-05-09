use super::*;

#[test]
fn tls_backend_default_is_rustls() {
    assert_eq!(TlsBackend::default(), TlsBackend::Rustls);
}

#[test]
fn tls_config_default() {
    let tls = TlsConfig::default();
    assert_eq!(tls.backend, TlsBackend::Rustls);
    assert!(tls.timeout_ms.is_none());
}

#[test]
fn app_config_defaults() {
    let cfg = AppConfig::with_defaults();
    assert_eq!(cfg.timeout_secs, DEFAULT_TIMEOUT_SECS);
    assert_eq!(cfg.backend, HttpBackend::Wreq);
    assert_eq!(cfg.tls.backend, TlsBackend::Rustls);
    assert!(cfg.tls.timeout_ms.is_none());
    assert_eq!(cfg.retry.count, DEFAULT_RETRY_COUNT);
    assert_eq!(cfg.retry.backoff_ms, DEFAULT_RETRY_BACKOFF_MS);
    assert_eq!(cfg.base_url, DEFAULT_BASE_URL);
}

#[test]
fn normalized_base_url_strips_slash() {
    let mut cfg = AppConfig::with_defaults();
    cfg.base_url = "https://www.tefas.gov.tr/".to_string();
    assert_eq!(cfg.normalized_base_url(), "https://www.tefas.gov.tr");
    cfg.base_url = "https://www.tefas.gov.tr".to_string();
    assert_eq!(cfg.normalized_base_url(), "https://www.tefas.gov.tr");
}

#[test]
fn tls_backend_variants_are_distinct() {
    assert_ne!(TlsBackend::Rustls, TlsBackend::NativeTls);
    assert_ne!(TlsBackend::Rustls, TlsBackend::Insecure);
}

#[test]
fn http_backend_eq() {
    assert_eq!(HttpBackend::Reqwest, HttpBackend::Reqwest);
    assert_ne!(HttpBackend::Reqwest, HttpBackend::Hyper);
}

// ── config contract tests ────────────────────────────────────────────────────

#[test]
fn retry_config_defaults_match_constants() {
    let r = RetryConfig::default();
    assert_eq!(r.count, DEFAULT_RETRY_COUNT);
    assert_eq!(r.backoff_ms, DEFAULT_RETRY_BACKOFF_MS);
}

#[test]
fn auth_config_defaults_match_constants() {
    let a = AuthConfig::default();
    assert_eq!(a.user_agent, DEFAULT_USER_AGENT);
    assert_eq!(a.referer, DEFAULT_REFERER);
    assert!(a.token.is_none());
    assert!(a.cookies_file.is_none());
    assert!(a.impcurl_impersonate.is_none());
    assert!(!a.skip_preflight);
}

#[test]
fn app_config_default_impl_matches_with_defaults() {
    let a = AppConfig::default();
    let b = AppConfig::with_defaults();
    assert_eq!(a.timeout_secs, b.timeout_secs);
    assert_eq!(a.backend, b.backend);
    assert_eq!(a.tls.backend, b.tls.backend);
    assert_eq!(a.retry.count, b.retry.count);
    assert_eq!(a.retry.backoff_ms, b.retry.backoff_ms);
    assert_eq!(a.base_url, b.base_url);
    assert_eq!(a.auth.user_agent, b.auth.user_agent);
    assert_eq!(a.auth.referer, b.auth.referer);
}
