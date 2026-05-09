use std::path::PathBuf;

/// Locate a usable `curl-impersonate` CLI binary. Searches common install paths
/// and finally falls back to PATH lookup.
pub(crate) fn find_binary() -> anyhow::Result<String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Next to the current executable
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        candidates.push(parent.join("lib/impcurl/bin/curl-impersonate-chrome"));
        candidates.push(parent.join("lib/impcurl/bin/curl-impersonate-ff"));
        candidates.push(parent.join("lib/impcurl/bin/curl-impersonate"));
        candidates.push(parent.join("bin/curl-impersonate-chrome"));
        candidates.push(parent.join("bin/curl-impersonate-ff"));
        candidates.push(parent.join("bin/curl-impersonate"));
    }

    // Well-known system/vendor locations
    for prefix in &["/opt/curl-impersonate", "vendor/curl-impersonate"] {
        candidates.push(PathBuf::from(format!(
            "{prefix}/bin/curl-impersonate-chrome"
        )));
        candidates.push(PathBuf::from(format!("{prefix}/bin/curl-impersonate-ff")));
        candidates.push(PathBuf::from(format!("{prefix}/bin/curl-impersonate")));
    }

    // Home dir locations
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("lib/impcurl/bin/curl-impersonate-chrome"));
        candidates.push(home.join(".impcurl/bin/curl-impersonate-chrome"));
    }

    // PATH fallbacks (these succeed only if the binary is actually on PATH)
    candidates.push(PathBuf::from("curl-impersonate-chrome"));
    candidates.push(PathBuf::from("curl-impersonate-ff"));
    candidates.push(PathBuf::from("curl-impersonate"));

    for p in &candidates {
        if p.is_absolute() {
            if p.is_file()
                && let Some(s) = p.to_str()
            {
                return Ok(s.to_string());
            }
        } else if let Some(s) = p.to_str()
            && which_binary(s)
        {
            return Ok(s.to_string());
        }
    }

    Err(anyhow::anyhow!(
        "curl-impersonate binary not found. Install it from https://github.com/lwthiker/curl-impersonate \
         or place it under /opt/curl-impersonate/bin/ or vendor/curl-impersonate/bin/"
    ))
}

/// If `binary` is an absolute path, derive its sibling `lib/` directory for
/// `LD_LIBRARY_PATH`. Returns `None` for bare names resolved via PATH.
pub(crate) fn lib_dir_for(binary: &str) -> Option<String> {
    let p = PathBuf::from(binary);
    if p.is_absolute() {
        p.parent()?
            .parent()
            .map(|d| d.join("lib").to_string_lossy().into_owned())
    } else {
        None
    }
}

/// Returns `true` if `name` resolves to an executable on the system PATH.
fn which_binary(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
