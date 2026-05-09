use std::sync::atomic::AtomicBool;

pub(crate) static IMPCURL_IMPERSONATE_FALLBACK_WARNED: AtomicBool = AtomicBool::new(false);
pub(crate) static IMPCURL_PROCESS_FALLBACK_WARNED: AtomicBool = AtomicBool::new(false);

/// Returns `true` when the curl-impersonate stderr indicates the `--impersonate`
/// flag is not recognized by this binary.
pub(crate) fn unknown_impersonate(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    s.contains("option --impersonate: is unknown")
        || s.contains("unrecognized option '--impersonate'")
        || s.contains("unknown option --impersonate")
}
