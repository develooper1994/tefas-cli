use http::Method;
use http::header::HeaderMap;
use url::Url;

pub(crate) fn debug_http_enabled() -> bool {
    std::env::var("NETWORK_DEBUG_HTTP")
        .or_else(|_| std::env::var("TEFAS_DEBUG_HTTP"))
        .map(|v| {
            let s = v.trim();
            s == "1"
                || s.eq_ignore_ascii_case("true")
                || s.eq_ignore_ascii_case("yes")
                || s.eq_ignore_ascii_case("on")
        })
        .unwrap_or(false)
}

pub(crate) fn dump_http_request(method: &Method, url: &str, headers: &HeaderMap) {
    eprintln!("[tefas-network][req] {} {}", method.as_str(), url);
    for (name, value) in headers {
        let value_text = value.to_str().unwrap_or("<non-utf8>");
        eprintln!(
            "[tefas-network][req][header] {}: {}",
            name.as_str(),
            value_text
        );
    }
}

pub(crate) fn looks_like_waf_challenge(text: &str) -> bool {
    text.contains("window[\"failureConfig\"]")
        || text.contains("/TSPD/")
        || text.contains("Your support ID is:")
        || text.contains("Request Rejected")
        || text.contains("request rejected")
        || text.contains("Please enable JavaScript to view the page content")
}

pub(crate) fn compute_sec_fetch_site(url: &str, referer: Option<&str>) -> &'static str {
    let Some(rf) = referer else {
        return "none";
    };

    let target = Url::parse(url);
    let source = Url::parse(rf);
    match (target, source) {
        (Ok(target), Ok(source)) => {
            let same_scheme = target.scheme() == source.scheme();
            let same_host = target.host_str() == source.host_str();
            let same_port = target.port_or_known_default() == source.port_or_known_default();
            if same_scheme && same_host && same_port {
                "same-origin"
            } else {
                "cross-site"
            }
        }
        _ => "none",
    }
}
