use anyhow::Context;
use reqwest::StatusCode;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub(crate) async fn download_to_path(
    url: &str,
    target: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    if dry_run {
        println!("[dry-run] download {} -> {}", url, target.display());
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating parent dir for {}", target.display()))?;
    }

    let response = reqwest::get(url)
        .await
        .with_context(|| format!("download request failed: {}", url))?;
    if response.status() != StatusCode::OK {
        anyhow::bail!(
            "download failed with status {} for {}",
            response.status(),
            url
        );
    }

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed reading response body for {}", url))?;

    let mut file = fs::File::create(target)
        .with_context(|| format!("failed creating file {}", target.display()))?;
    file.write_all(&bytes)
        .with_context(|| format!("failed writing file {}", target.display()))?;
    Ok(())
}

pub(crate) fn install_binary_like(
    downloaded: &Path,
    install_target: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    if dry_run {
        println!(
            "[dry-run] install binary {} -> {}",
            downloaded.display(),
            install_target.display()
        );
        return Ok(());
    }

    let install_dir = install_target
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid install target: {}", install_target.display()))?;
    fs::create_dir_all(install_dir)
        .with_context(|| format!("failed creating install dir {}", install_dir.display()))?;

    fs::copy(downloaded, install_target).with_context(|| {
        format!(
            "failed copying {} to {}",
            downloaded.display(),
            install_target.display()
        )
    })?;

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(install_target)
            .with_context(|| format!("failed reading metadata {}", install_target.display()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(install_target, perms).with_context(|| {
            format!(
                "failed setting executable permissions on {}",
                install_target.display()
            )
        })?;
    }

    Ok(())
}

pub(crate) fn build_download_temp_path(work_dir: &Path, install_target: &Path) -> PathBuf {
    let file_name = install_target
        .file_name()
        .map(|s| s.to_os_string())
        .unwrap_or_else(|| "downloaded-artifact".into());
    work_dir.join(file_name)
}
