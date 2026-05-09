use anyhow::Context;

/// Spawn `curl-impersonate` with the given arguments and return its raw output.
pub(crate) async fn run(
    binary: &str,
    lib_dir: Option<&String>,
    args: &[String],
) -> anyhow::Result<std::process::Output> {
    use tokio::process::Command;

    let mut cmd = Command::new(binary);
    if let Some(ld) = lib_dir {
        cmd.env("LD_LIBRARY_PATH", ld);
    }
    cmd.args(args)
        .output()
        .await
        .context("failed to spawn curl-impersonate")
}
