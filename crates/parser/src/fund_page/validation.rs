pub(crate) fn contains_failureconfig(html: &str) -> bool {
    // TEFAS always sends "failureConfig" (camelCase); exact search is faster.
    html.contains("failureConfig") || html.contains("failureconfig")
}
