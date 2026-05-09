use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tefas_tools::{install_with_fallback, InstallOptions};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), nanos));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

#[tokio::test]
async fn returns_ok_when_check_command_exists() {
    let work_dir = unique_temp_dir("tefas_tools_ok");
    let opts = InstallOptions {
        name: "shell".to_string(),
        check_cmd: "sh".to_string(),
        work_dir: work_dir.clone(),
        dry_run: true,
        ..InstallOptions::default()
    };

    let result = install_with_fallback(opts).await;
    assert!(result.is_ok());

    let _ = fs::remove_dir_all(work_dir);
}

#[tokio::test]
async fn fails_when_all_fallback_steps_exhausted() {
    let work_dir = unique_temp_dir("tefas_tools_fail");
    let opts = InstallOptions {
        name: "fake-tool".to_string(),
        check_cmd: "this-command-does-not-exist-xyz".to_string(),
        work_dir: work_dir.clone(),
        dry_run: true,
        ..InstallOptions::default()
    };

    let err = install_with_fallback(opts)
        .await
        .expect_err("must fail when no fallback strategy configured");
    let msg = err.to_string();
    assert!(msg.contains("All fallback steps exhausted"));

    let _ = fs::remove_dir_all(work_dir);
}
