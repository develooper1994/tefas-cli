use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use tefas_tools::{InstallOptions, install_with_fallback};

use crate::common::run_checked;

pub fn run(_workspace_root: &Path, task: &str, args: Vec<String>) -> Result<()> {
    match task {
        "install-fallback" => run_install_fallback(args),
        "protect" => run_protect(args),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => bail!("unknown tools xtask command: {other}"),
    }
}

fn run_install_fallback(args: Vec<String>) -> Result<()> {
    let mut opts = InstallOptions::default();
    opts.work_dir = PathBuf::from("/tmp/install-with-fallback");

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--name" => {
                opts.name = args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--name requires value"))?;
                i += 2;
            }
            "--check-cmd" => {
                opts.check_cmd = args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--check-cmd requires value"))?;
                i += 2;
            }
            "--apt" => { opts.apt.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--apt requires value"))?); i += 2; }
            "--dnf" => { opts.dnf.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--dnf requires value"))?); i += 2; }
            "--yum" => { opts.yum.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--yum requires value"))?); i += 2; }
            "--pacman" => { opts.pacman.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--pacman requires value"))?); i += 2; }
            "--apk" => { opts.apk.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--apk requires value"))?); i += 2; }
            "--brew" => { opts.brew.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--brew requires value"))?); i += 2; }
            "--winget" => { opts.winget.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--winget requires value"))?); i += 2; }
            "--scoop" => { opts.scoop.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--scoop requires value"))?); i += 2; }
            "--choco" => { opts.choco.push(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--choco requires value"))?); i += 2; }
            "--prebuilt-url" => { opts.prebuilt_url = Some(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--prebuilt-url requires value"))?); i += 2; }
            "--prebuilt-target" => { opts.prebuilt_target = Some(PathBuf::from(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--prebuilt-target requires value"))?)); i += 2; }
            "--source-url" => { opts.source_url = Some(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--source-url requires value"))?); i += 2; }
            "--source-build" => { opts.source_build = Some(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--source-build requires value"))?); i += 2; }
            "--source-install" => { opts.source_install = Some(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--source-install requires value"))?); i += 2; }
            "--work-dir" => { opts.work_dir = PathBuf::from(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--work-dir requires value"))?); i += 2; }
            "--dry-run" => { opts.dry_run = true; i += 1; }
            other => bail!("Unknown option: {other}"),
        }
    }

    if opts.name.is_empty() || opts.check_cmd.is_empty() {
        bail!("--name and --check-cmd are required")
    }

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(install_with_fallback(opts))
}

fn run_protect(args: Vec<String>) -> Result<()> {
    let mut input = String::new();
    let mut output: Option<String> = None;
    let mut tool = "auto".to_string();
    let mut strip = false;
    let mut compress = false;
    let mut list = false;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-i" | "--input" => { input = args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--input requires value"))?; i += 2; }
            "-o" | "--output" => { output = Some(args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--output requires value"))?); i += 2; }
            "-t" | "--tool" => { tool = args.get(i + 1).cloned().ok_or_else(|| anyhow::anyhow!("--tool requires value"))?; i += 2; }
            "-s" | "--strip" => { strip = true; i += 1; }
            "-c" | "--compress" => { compress = true; i += 1; }
            "-l" | "--list" => { list = true; i += 1; }
            other => bail!("Unknown option: {other}"),
        }
    }

    if list {
        println!("Detected tools:");
        for c in ["shc", "bunster", "bash-obfuscate", "npx", "upx", "gcc"] {
            if command_exists(c) {
                println!("- {c}");
            }
        }
        return Ok(());
    }

    if input.is_empty() {
        bail!("--input is required unless --list is used");
    }

    let selected_tool = if tool == "auto" {
        if command_exists("shc") {
            "shc".to_string()
        } else if command_exists("bunster") {
            "bunster".to_string()
        } else if command_exists("bash-obfuscate") || command_exists("npx") {
            "bash-obfuscate".to_string()
        } else {
            bail!("No suitable protection tool found (shc, bunster, bash-obfuscate)");
        }
    } else {
        tool
    };

    let out = output.unwrap_or_else(|| {
        if selected_tool == "bash-obfuscate" {
            format!("{}.obf.sh", input.trim_end_matches(".sh"))
        } else {
            input.trim_end_matches(".sh").to_string()
        }
    });

    match selected_tool.as_str() {
        "shc" => {
            run_checked(Command::new("shc").arg("-f").arg(&input).arg("-o").arg(&out).arg("-r"), "shc protect")?;
        }
        "bunster" => {
            if command_exists("bunster") {
                run_checked(Command::new("bunster").arg("build").arg(&input).arg("-o").arg(&out), "bunster protect")?;
            } else {
                run_checked(Command::new("npx").arg("--yes").arg("bunster").arg("build").arg(&input).arg("-o").arg(&out), "npx bunster protect")?;
            }
        }
        "bash-obfuscate" => {
            if command_exists("bash-obfuscate") {
                run_checked(Command::new("bash-obfuscate").arg(&input).arg("-o").arg(&out), "bash-obfuscate protect")?;
            } else {
                run_checked(Command::new("npx").arg("--yes").arg("bash-obfuscate").arg(&input).arg("-o").arg(&out), "npx bash-obfuscate protect")?;
            }
        }
        other => bail!("Unknown tool: {other}"),
    }

    if compress && command_exists("upx") {
        let _ = Command::new("upx").arg("-9").arg(&out).status();
    }
    if strip {
        let _ = std::fs::remove_file(&input);
    }

    println!("Protection completed: {out}");
    Ok(())
}

fn command_exists(command: &str) -> bool {
    let status = if cfg!(windows) {
        Command::new("where").arg(command).status()
    } else {
        Command::new("which").arg(command).status()
    };
    matches!(status, Ok(s) if s.success())
}

pub fn print_help() {
    println!(
        "tools xtask commands:\n  cargo xtask tools install-fallback [OPTIONS]\n  cargo xtask tools protect [OPTIONS]"
    );
}
