use anyhow::Context;
use std::process::Command;

pub(crate) fn have_cmd(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        return Command::new("where")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

pub(crate) fn run_command(cmd_str: &str, dry_run: bool) -> anyhow::Result<()> {
    if dry_run {
        println!("[dry-run] {}", cmd_str);
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    let status = Command::new("cmd")
        .arg("/C")
        .arg(cmd_str)
        .status()
        .with_context(|| format!("failed to execute command: {}", cmd_str))?;

    #[cfg(not(target_os = "windows"))]
    let status = Command::new("bash")
        .arg("-c")
        .arg(cmd_str)
        .status()
        .with_context(|| format!("failed to execute command: {}", cmd_str))?;
    if !status.success() {
        anyhow::bail!("command failed: {}", cmd_str);
    }
    Ok(())
}
