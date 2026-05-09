use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn workspace_root() -> Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("xtask root does not have a workspace parent")?
        .to_path_buf())
}

pub fn command_exists(command: &str) -> bool {
    let status = if cfg!(windows) {
        Command::new("where").arg(command).status()
    } else {
        Command::new("which").arg(command).status()
    };
    matches!(status, Ok(s) if s.success())
}

pub fn run_checked(cmd: &mut Command, op: &str) -> Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("failed to execute {op}"))?;
    if !status.success() {
        bail!("{op} failed with status {status}");
    }
    Ok(())
}

pub fn resolve_local_profile_bin(workspace_root: &Path, profile_name: &str) -> PathBuf {
    let candidate = workspace_root
        .join("target")
        .join(profile_name)
        .join("tefas-cli");
    if candidate.exists() {
        return candidate;
    }

    match profile_name {
        "profileDebug" => {
            let alt = workspace_root
                .join("target")
                .join("profileProfileDebug")
                .join("tefas-cli");
            if alt.exists() {
                return alt;
            }
        }
        "profileRelease" => {
            let alt = workspace_root.join("target").join("release").join("tefas-cli");
            if alt.exists() {
                return alt;
            }
        }
        _ => {}
    }
    candidate
}

pub fn latest_directory(base: &Path) -> Result<Option<PathBuf>> {
    if !base.exists() {
        return Ok(None);
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() {
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    Ok(dirs.pop())
}

pub fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn timestamp_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}
