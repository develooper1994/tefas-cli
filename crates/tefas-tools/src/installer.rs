use crate::command::{have_cmd, run_command};
use crate::download::{build_download_temp_path, download_to_path, install_binary_like};
use anyhow::Context;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    pub name: String,
    pub check_cmd: String,
    pub apt: Vec<String>,
    pub dnf: Vec<String>,
    pub yum: Vec<String>,
    pub pacman: Vec<String>,
    pub apk: Vec<String>,
    pub brew: Vec<String>,
    pub winget: Vec<String>,
    pub scoop: Vec<String>,
    pub choco: Vec<String>,
    pub prebuilt_url: Option<String>,
    pub prebuilt_target: Option<PathBuf>,
    pub source_url: Option<String>,
    pub source_build: Option<String>,
    pub source_install: Option<String>,
    pub work_dir: PathBuf,
    pub dry_run: bool,
}

pub async fn install_with_fallback(opts: InstallOptions) -> anyhow::Result<()> {
    if have_cmd(&opts.check_cmd) {
        println!(
            "{} already installed ({} found in PATH)",
            opts.name, opts.check_cmd
        );
        return Ok(());
    }

    fs::create_dir_all(&opts.work_dir)
        .with_context(|| format!("failed creating work dir {}", opts.work_dir.display()))?;

    println!("Step 1/3: Package manager install attempt");
    let mut managers: Vec<(&str, &Vec<String>, String)> = vec![
        (
            "apt-get",
            &opts.apt,
            "sudo apt-get update && sudo apt-get install -y".to_string(),
        ),
        ("dnf", &opts.dnf, "sudo dnf install -y".to_string()),
        ("yum", &opts.yum, "sudo yum install -y".to_string()),
        (
            "pacman",
            &opts.pacman,
            "sudo pacman -Sy --noconfirm".to_string(),
        ),
        ("apk", &opts.apk, "sudo apk add".to_string()),
        ("brew", &opts.brew, "brew install".to_string()),
    ];

    if cfg!(target_os = "windows") {
        managers.extend([
            (
                "winget",
                &opts.winget,
                "winget install --accept-source-agreements --accept-package-agreements".to_string(),
            ),
            ("scoop", &opts.scoop, "scoop install".to_string()),
            ("choco", &opts.choco, "choco install -y".to_string()),
        ]);
    }

    for (manager_cmd, packages, base_cmd) in managers {
        if !packages.is_empty() && have_cmd(manager_cmd) {
            let full_cmd = format!("{} {}", base_cmd, packages.join(" "));
            if run_command(&full_cmd, opts.dry_run).is_ok() && have_cmd(&opts.check_cmd) {
                println!(
                    "{} installed via package manager ({})",
                    opts.name, manager_cmd
                );
                return Ok(());
            }
        }
    }

    if let (Some(url), Some(target)) = (&opts.prebuilt_url, &opts.prebuilt_target) {
        println!("Step 2/3: Prebuilt artifact install attempt");
        let tmp_path = build_download_temp_path(&opts.work_dir, target);
        if download_to_path(url, &tmp_path, opts.dry_run).await.is_ok()
            && install_binary_like(&tmp_path, target, opts.dry_run).is_ok()
            && have_cmd(&opts.check_cmd)
        {
            println!("{} installed from prebuilt artifact", opts.name);
            return Ok(());
        }
    }

    if let (Some(url), Some(build_cmd)) = (&opts.source_url, &opts.source_build) {
        println!("Step 3/3: Source build attempt");
        let src_dir = opts.work_dir.join("source");
        run_command(&format!("rm -rf '{}'", src_dir.display()), opts.dry_run)?;

        if url.ends_with(".git") || url.contains("github.com") {
            run_command(
                &format!("git clone '{}' '{}'", url, src_dir.display()),
                opts.dry_run,
            )?;
        } else {
            let archive_path = opts.work_dir.join("source-archive");
            download_to_path(url, &archive_path, opts.dry_run).await?;
            fs::create_dir_all(&src_dir)
                .with_context(|| format!("failed creating source dir {}", src_dir.display()))?;
            run_command(
                &format!(
                    "tar -xf '{}' -C '{}' --strip-components=1",
                    archive_path.display(),
                    src_dir.display()
                ),
                opts.dry_run,
            )?;
        }

        let build_full = format!("cd '{}' && {}", src_dir.display(), build_cmd);
        run_command(&build_full, opts.dry_run)?;

        if let Some(install_cmd) = &opts.source_install {
            let install_full = format!("cd '{}' && {}", src_dir.display(), install_cmd);
            run_command(&install_full, opts.dry_run)?;
        }

        if have_cmd(&opts.check_cmd) {
            println!("{} installed from source build", opts.name);
            return Ok(());
        }
    }

    anyhow::bail!(
        "All fallback steps exhausted and {} is still unavailable",
        opts.name
    )
}
