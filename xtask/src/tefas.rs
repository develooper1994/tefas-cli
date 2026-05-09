use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::common::{
    command_exists, latest_directory, resolve_local_profile_bin, run_checked, sh_quote,
    timestamp_string,
};

#[derive(Debug, Clone)]
struct RemotePipelineOpts {
    remote_alias: String,
    remote_repo: String,
    bench_filter: String,
    measurement_time: String,
    warmup_time: String,
    run_samply: bool,
    run_flamegraph: bool,
    profile_name: String,
    cainfo: String,
}

#[derive(Debug, Clone)]
struct RemoteSamplyOpts {
    remote_alias: String,
    remote_repo: String,
    profile_name: String,
    workload: String,
    output: Option<PathBuf>,
}

impl RemoteSamplyOpts {
    fn from_env() -> Self {
        Self {
            remote_alias: env::var("REMOTE_ALIAS").unwrap_or_else(|_| "tefas-vm-ip".to_string()),
            remote_repo: env::var("REMOTE_REPO")
                .unwrap_or_else(|_| "/home/developer/Projects/github/TefasKapRequests".to_string()),
            profile_name: env::var("PROFILE_NAME").unwrap_or_else(|_| "profileDebug".to_string()),
            workload: env::var("WORKLOAD").unwrap_or_else(|_| "parse".to_string()),
            output: None,
        }
    }
}

impl RemotePipelineOpts {
    fn from_env() -> Self {
        Self {
            remote_alias: env::var("REMOTE_ALIAS").unwrap_or_else(|_| "tefas-vm-ip".to_string()),
            remote_repo: env::var("REMOTE_REPO")
                .unwrap_or_else(|_| "/home/developer/Projects/github/TefasKapRequests".to_string()),
            bench_filter: env::var("BENCH_FILTER")
                .unwrap_or_else(|_| "parse_document_batch/ALL_DATASET_PAGES".to_string()),
            measurement_time: env::var("MEASUREMENT_TIME").unwrap_or_else(|_| "3".to_string()),
            warmup_time: env::var("WARMUP_TIME").unwrap_or_else(|_| "1".to_string()),
            run_samply: env::var("RUN_SAMPLY").map(|v| v != "0").unwrap_or(true),
            run_flamegraph: env::var("RUN_FLAMEGRAPH")
                .map(|v| v == "1")
                .unwrap_or(false),
            profile_name: env::var("PROFILE_NAME").unwrap_or_else(|_| "profileDebug".to_string()),
            cainfo: env::var("CARGO_HTTP_CAINFO").unwrap_or_default(),
        }
    }

    fn profile_bucket(&self) -> &str {
        match self.profile_name.as_str() {
            "profileDebug" => "profileDebug",
            "profileRelease" => "profileRelease",
            other => other,
        }
    }
}

pub fn run(workspace_root: &Path, task: &str, args: Vec<String>) -> Result<()> {
    match task {
        "pgo" => run_pgo(workspace_root, args),
        "lint" => run_lint(workspace_root),
        "probe" => run_probe(workspace_root, args),
        "bench" => {
            let opts = parse_remote_opts(args, None)?;
            run_remote_pipeline(workspace_root, &opts)
        }
        "profile" => {
            let opts = parse_remote_opts(args, Some(true))?;
            run_remote_pipeline(workspace_root, &opts)
        }
        "perf-remote" => {
            let opts = parse_remote_opts(args, None)?;
            run_remote_pipeline(workspace_root, &opts)
        }
        "samply-remote" => {
            let opts = parse_remote_samply_opts(args)?;
            run_remote_samply(workspace_root, &opts)
        }
        "concurrency-sweep" => run_concurrency_sweep(workspace_root, args),
        "ssh-setup" => run_ssh_setup(args),
        "samply-summary" => run_samply_summary(args),
        "fuzz" => run_fuzz(workspace_root, args),
        "test-manual" => run_test_manual(workspace_root, args),
        "ffi-header" => run_ffi_header(workspace_root),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => bail!("unknown tefas xtask command: {other}"),
    }
}

fn parse_remote_opts(args: Vec<String>, force_samply: Option<bool>) -> Result<RemotePipelineOpts> {
    let mut opts = RemotePipelineOpts::from_env();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--host requires a value");
                };
                opts.remote_alias = v.clone();
                i += 2;
            }
            "--repo" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--repo requires a value");
                };
                opts.remote_repo = v.clone();
                i += 2;
            }
            "--filter" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--filter requires a value");
                };
                opts.bench_filter = v.clone();
                i += 2;
            }
            "--warmup" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--warmup requires a value");
                };
                opts.warmup_time = v.clone();
                i += 2;
            }
            "--measure" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--measure requires a value");
                };
                opts.measurement_time = v.clone();
                i += 2;
            }
            "--cargo-profile" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--cargo-profile requires a value");
                };
                opts.profile_name = v.clone();
                i += 2;
            }
            "--samply" => {
                opts.run_samply = true;
                i += 1;
            }
            "--no-samply" => {
                opts.run_samply = false;
                i += 1;
            }
            "--flamegraph" => {
                opts.run_flamegraph = true;
                i += 1;
            }
            other => bail!("Unknown option: {other}"),
        }
    }

    if let Some(force) = force_samply {
        opts.run_samply = force;
    }

    Ok(opts)
}

fn parse_remote_samply_opts(args: Vec<String>) -> Result<RemoteSamplyOpts> {
    let mut opts = RemoteSamplyOpts::from_env();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--host requires a value");
                };
                opts.remote_alias = v.clone();
                i += 2;
            }
            "--repo" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--repo requires a value");
                };
                opts.remote_repo = v.clone();
                i += 2;
            }
            "--cargo-profile" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--cargo-profile requires a value");
                };
                opts.profile_name = v.clone();
                i += 2;
            }
            "--workload" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--workload requires a value");
                };
                opts.workload = v.clone();
                i += 2;
            }
            "--output" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--output requires a value");
                };
                opts.output = Some(PathBuf::from(v));
                i += 2;
            }
            other => bail!("Unknown option: {other}"),
        }
    }
    Ok(opts)
}

fn run_remote_pipeline(workspace_root: &Path, opts: &RemotePipelineOpts) -> Result<()> {
    if !command_exists("ssh") {
        bail!("ssh is required for remote pipeline");
    }
    if !command_exists("rsync") {
        bail!("rsync is required for remote pipeline");
    }

    if opts.run_flamegraph {
        eprintln!("WARN: --flamegraph is not implemented in native remote mode yet and is ignored");
    }

    let repo_root = workspace_root
        .parent()
        .context("failed to resolve repository root from tefas workspace")?;
    let local_artifact_dir = workspace_root
        .join("target")
        .join("perf-artifacts-remote")
        .join(opts.profile_bucket());
    fs::create_dir_all(&local_artifact_dir)
        .with_context(|| format!("failed to create {}", local_artifact_dir.display()))?;

    println!("Syncing to {}:{} ...", opts.remote_alias, opts.remote_repo);
    run_checked(
        Command::new("rsync")
            .arg("-az")
            .arg("--delete")
            .arg("--exclude")
            .arg(".git")
            .arg("--exclude")
            .arg("tefas/target")
            .arg("--exclude")
            .arg("target")
            .arg("--exclude")
            .arg(".venv")
            .arg(format!("{}/", repo_root.display()))
            .arg(format!("{}:{}/", opts.remote_alias, opts.remote_repo)),
        "rsync repository to remote",
    )?;

    let local_profile_bin = resolve_local_profile_bin(workspace_root, &opts.profile_name);
    if !local_profile_bin.exists() {
        println!("Building local binary: {}", opts.profile_name);
        run_checked(
            Command::new("cargo")
                .arg("build")
                .arg("--profile")
                .arg(&opts.profile_name)
                .arg("-p")
                .arg("tefas-cli")
                .current_dir(workspace_root),
            "cargo build local profile binary",
        )?;
    }
    if !local_profile_bin.exists() {
        bail!(
            "local profile binary not found after build: {}",
            local_profile_bin.display()
        );
    }

    let remote_profile_bin = format!(
        "{}/target/perf-bin/{}/tefas-cli",
        opts.remote_repo, opts.profile_name
    );
    run_checked(
        Command::new("ssh").arg(&opts.remote_alias).arg(format!(
            "mkdir -p {}",
            sh_quote(&format!(
                "{}/target/perf-bin/{}",
                opts.remote_repo, opts.profile_name
            ))
        )),
        "prepare remote perf-bin directory",
    )?;

    println!(
        "Deploying binary to {}:{}",
        opts.remote_alias, remote_profile_bin
    );
    run_checked(
        Command::new("rsync")
            .arg("-az")
            .arg(&local_profile_bin)
            .arg(format!("{}:{}", opts.remote_alias, remote_profile_bin)),
        "rsync binary to remote",
    )?;

    let ts = timestamp_string();
    let out_dir = format!(
        "{}/target/perf-artifacts/{}/{}",
        opts.remote_repo,
        opts.profile_bucket(),
        ts
    );

    let remote_script = format!(
        "set -euo pipefail\ncd {repo}\n\nif [[ ! -d datasets/fundpage/html ]] || [[ -z \"$(find datasets/fundpage/html -maxdepth 1 -type f -name '*.html' | head -n 1)\" ]]; then\n  echo \"ERROR: datasets/fundpage/html is missing on remote host.\" >&2\n  exit 2\nfi\n\nif [[ ! -x {bin} ]]; then\n  echo \"ERROR: remote prebuilt binary not found: {bin}\" >&2\n  exit 2\nfi\n\nOUT_DIR={out_dir}\nmkdir -p \"$OUT_DIR/parsed\"\n\nmapfile -t HTML_FILES < <(find datasets/fundpage/html -maxdepth 1 -type f -name '*.html' | sort)\nHTML_COUNT=\"${{#HTML_FILES[@]}}\"\nif [[ \"$HTML_COUNT\" -eq 0 ]]; then\n  echo \"ERROR: no HTML files found in datasets/fundpage/html\" >&2\n  exit 2\nfi\n\nWORKLOAD_CMD='for f in datasets/fundpage/html/*.html; do {bin} parse \"$f\" >/dev/null; done'\n{{\n  echo \"profile={profile}\"\n  echo \"bucket={bucket}\"\n  echo \"workload=all_dataset_pages\"\n  echo \"html_count=$HTML_COUNT\"\n  echo \"filter={filter}\"\n  echo \"measure={measure}\"\n  echo \"warmup={warmup}\"\n}} > \"$OUT_DIR/run_meta.txt\"\n\nfor f in datasets/fundpage/html/*.html; do\n  b=\"$(basename \"$f\" .html)\"\n  {bin} parse \"$f\" > \"$OUT_DIR/parsed/${{b}}.txt\"\ndone\n\nif command -v perf >/dev/null 2>&1; then\n  perf stat -o \"$OUT_DIR/perf_stat.txt\" -- /bin/bash -lc \"$WORKLOAD_CMD\" > /dev/null 2>&1\n  perf record -F 199 --call-graph dwarf --output \"$OUT_DIR/perf.data\" -- /bin/bash -lc \"$WORKLOAD_CMD\"\n  perf report --stdio --no-children --percent-limit 0.05 --sort overhead,symbol -i \"$OUT_DIR/perf.data\" > \"$OUT_DIR/perf_report.txt\"\nelse\n  echo \"WARN: perf not found on remote host\" > \"$OUT_DIR/perf_report.txt\"\nfi\n\nif [[ {run_samply} == '1' ]] && command -v samply >/dev/null 2>&1; then\n  samply record --save-only -o \"$OUT_DIR/profile.json.gz\" -- /bin/bash -lc \"$WORKLOAD_CMD\" > /dev/null 2>&1 || true\nfi\n\necho \"remote perf complete: $OUT_DIR\"\n",
        repo = sh_quote(&opts.remote_repo),
        bin = sh_quote(&remote_profile_bin),
        out_dir = sh_quote(&out_dir),
        profile = opts.profile_name,
        bucket = opts.profile_bucket(),
        filter = opts.bench_filter,
        measure = opts.measurement_time,
        warmup = opts.warmup_time,
        run_samply = if opts.run_samply { "1" } else { "0" },
    );

    let mut remote_cmd = Command::new("ssh");
    remote_cmd
        .arg(&opts.remote_alias)
        .arg("bash")
        .arg("-lc")
        .arg(remote_script);
    if !opts.cainfo.is_empty() {
        remote_cmd.env("CARGO_HTTP_CAINFO", &opts.cainfo);
    }
    run_checked(&mut remote_cmd, "remote benchmark execution")?;

    println!("Syncing artifacts from remote...");
    let sync_status = Command::new("rsync")
        .arg("-az")
        .arg(format!(
            "{}:{}/target/perf-artifacts/{}/",
            opts.remote_alias,
            opts.remote_repo,
            opts.profile_bucket()
        ))
        .arg(format!("{}/", local_artifact_dir.display()))
        .status();

    match sync_status {
        Ok(status) if status.success() => {}
        Ok(_) | Err(_) => {
            eprintln!(
                "WARN: could not sync remote artifacts bucket {}",
                opts.profile_bucket()
            );
        }
    }

    if let Some(latest) = latest_directory(&local_artifact_dir)? {
        println!("Remote perf done. Latest artifacts: {}", latest.display());
    }

    Ok(())
}

fn run_remote_samply(workspace_root: &Path, opts: &RemoteSamplyOpts) -> Result<()> {
    if !command_exists("ssh") {
        bail!("ssh is required for remote samply");
    }
    if !command_exists("rsync") {
        bail!("rsync is required for remote samply");
    }

    let local_bin = resolve_local_profile_bin(workspace_root, &opts.profile_name);
    if !local_bin.exists() {
        println!("Building local binary: {}", opts.profile_name);
        run_checked(
            Command::new("cargo")
                .arg("build")
                .arg("--profile")
                .arg(&opts.profile_name)
                .arg("-p")
                .arg("tefas-cli")
                .current_dir(workspace_root),
            "cargo build local profile binary",
        )?;
    }
    if !local_bin.exists() {
        bail!("local profile binary not found: {}", local_bin.display());
    }

    let remote_bin = format!(
        "{}/target/perf-bin/{}/tefas-cli",
        opts.remote_repo, opts.profile_name
    );
    run_checked(
        Command::new("ssh").arg(&opts.remote_alias).arg(format!(
            "mkdir -p {}",
            sh_quote(&format!(
                "{}/target/perf-bin/{}",
                opts.remote_repo, opts.profile_name
            ))
        )),
        "prepare remote perf-bin directory",
    )?;

    println!("Deploying binary to {}:{}", opts.remote_alias, remote_bin);
    run_checked(
        Command::new("rsync")
            .arg("-az")
            .arg(&local_bin)
            .arg(format!("{}:{}", opts.remote_alias, remote_bin)),
        "rsync binary to remote",
    )?;

    let ts = timestamp_string();
    let remote_out_dir = format!(
        "{}/target/perf-artifacts/samply/{}_{}",
        opts.remote_repo, opts.workload, ts
    );
    let remote_profile = format!("{}/profile.json.gz", remote_out_dir);
    let remote_meta = format!("{}/run_meta.txt", remote_out_dir);

    let remote_cmd_for_workload = match opts.workload.as_str() {
        "parse" => format!(
            "{bin} --quiet --parse-concurrency 1 parse datasets/fundpage/html/*.html --output /tmp/tefas_samply_parse.json",
            bin = sh_quote(&remote_bin)
        ),
        "fundpage" => format!(
            "{bin} --quiet --network-concurrency 4 fundpage AC5 TLY TAR ZFB --output /tmp/tefas_samply_fundpage.json",
            bin = sh_quote(&remote_bin)
        ),
        "query" => format!(
            "{bin} --quiet --network-concurrency 2 query fonBilgiGetir fonFiyatBilgiGetir fonGetiriBazliBilgiGetir fonBuyuklukBazliBilgiGetir",
            bin = sh_quote(&remote_bin)
        ),
        "fetch" => format!(
            "{bin} --quiet --network-concurrency 2 fetch https://www.tefas.gov.tr/tr/fon-detayli-analiz/AC5 https://www.tefas.gov.tr/tr/fon-detayli-analiz/TLY --output /tmp/tefas_samply_fetch",
            bin = sh_quote(&remote_bin)
        ),
        other => bail!("unknown workload: {other}. expected parse|fundpage|query|fetch"),
    };

    let remote_script = format!(
        "set -euo pipefail\ncd {repo}\nif ! command -v samply >/dev/null 2>&1; then\n  echo \"ERROR: samply not found on remote host\" >&2\n  exit 2\nfi\nmkdir -p {out_dir}\n\n{{\n  echo \"timestamp=$(date +%Y%m%d_%H%M%S)\"\n  echo \"workload={workload}\"\n  echo \"binary={binary}\"\n  echo \"command={cmd}\"\n}} > {meta}\n\nsamply record --save-only -o {profile} -- /bin/bash -lc {cmd_q} >/dev/null 2>&1\n\necho \"remote samply complete: {profile}\"\n",
        repo = sh_quote(&opts.remote_repo),
        out_dir = sh_quote(&remote_out_dir),
        workload = opts.workload,
        binary = remote_bin,
        cmd = remote_cmd_for_workload,
        meta = sh_quote(&remote_meta),
        profile = sh_quote(&remote_profile),
        cmd_q = sh_quote(&remote_cmd_for_workload),
    );

    run_checked(
        Command::new("ssh")
            .arg(&opts.remote_alias)
            .arg("bash")
            .arg("-lc")
            .arg(remote_script),
        "remote samply execution",
    )?;

    let local_out = if let Some(path) = &opts.output {
        path.clone()
    } else {
        workspace_root
            .join("target")
            .join("perf-artifacts-remote")
            .join("samply")
            .join(format!("samply_{}_{}.json.gz", opts.workload, ts))
    };
    if let Some(parent) = local_out.parent() {
        fs::create_dir_all(parent)?;
    }
    run_checked(
        Command::new("rsync")
            .arg("-az")
            .arg(format!("{}:{}", opts.remote_alias, remote_profile))
            .arg(&local_out),
        "sync remote samply profile",
    )?;

    println!("Saved local samply profile: {}", local_out.display());
    Ok(())
}

fn run_pgo(workspace_root: &Path, args: Vec<String>) -> Result<()> {
    let action = args
        .first()
        .map(String::as_str)
        .unwrap_or("help")
        .to_string();
    let pgo_data_dir = env::var("TEFAS_PGO_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::temp_dir().join("tefas-pgo-data"));
    let profdata_file = pgo_data_dir.join("merged.profdata");

    match action.as_str() {
        "generate" => {
            println!("Building with profile generation...");
            if pgo_data_dir.exists() {
                fs::remove_dir_all(&pgo_data_dir).with_context(|| {
                    format!("failed to clean PGO directory: {}", pgo_data_dir.display())
                })?;
            }
            fs::create_dir_all(&pgo_data_dir).with_context(|| {
                format!("failed to create PGO directory: {}", pgo_data_dir.display())
            })?;

            run_checked(
                Command::new("cargo")
                    .arg("build")
                    .arg("-p")
                    .arg("tefas-cli")
                    .arg("--release")
                    .env(
                        "RUSTFLAGS",
                        format!("-Cprofile-generate={}", pgo_data_dir.display()),
                    )
                    .current_dir(workspace_root),
                "cargo build (PGO generate)",
            )?;

            println!("Running workload to gather profile data...");
            let fixture = workspace_root.join("datasets/fundpage/html/ADE.html");
            let workload_output = pgo_data_dir.join("parse_output.json");
            run_checked(
                Command::new(workspace_root.join("target/release/tefas-cli"))
                    .arg("parse")
                    .arg(&fixture)
                    .arg("--output")
                    .arg(&workload_output)
                    .current_dir(workspace_root),
                "tefas-cli parse workload",
            )?;

            println!("Merging profile data...");
            if !command_exists("llvm-profdata") {
                bail!("llvm-profdata not found. Please install llvm/clang.");
            }

            run_checked(
                Command::new("llvm-profdata")
                    .arg("merge")
                    .arg("-o")
                    .arg(&profdata_file)
                    .arg(&pgo_data_dir)
                    .current_dir(workspace_root),
                "llvm-profdata merge",
            )?;
            println!("Profile data generated at {}", profdata_file.display());
        }
        "use" => {
            if !profdata_file.exists() {
                bail!(
                    "Profile data not found at {}. Run 'cargo xtask pgo generate' first.",
                    profdata_file.display()
                );
            }

            println!("Building with profile-guided optimizations...");
            run_checked(
                Command::new("cargo")
                    .arg("build")
                    .arg("-p")
                    .arg("tefas-cli")
                    .arg("--release")
                    .env(
                        "RUSTFLAGS",
                        format!("-Cprofile-use={}", profdata_file.display()),
                    )
                    .current_dir(workspace_root),
                "cargo build (PGO use)",
            )?;
            println!("PGO-optimized binary built at ./target/release/tefas-cli");
        }
        "clean" => {
            if pgo_data_dir.exists() {
                fs::remove_dir_all(&pgo_data_dir).with_context(|| {
                    format!("failed to remove PGO directory: {}", pgo_data_dir.display())
                })?;
            }
            run_checked(
                Command::new("cargo")
                    .arg("clean")
                    .current_dir(workspace_root),
                "cargo clean",
            )?;
        }
        _ => {
            println!("Usage: cargo xtask pgo [generate|use|clean]");
        }
    }

    Ok(())
}

fn run_lint(workspace_root: &Path) -> Result<()> {
    if !command_exists("shellcheck") {
        bail!("shellcheck not found; please install shellcheck");
    }

    let mut files = Vec::new();
    collect_shell_scripts(workspace_root, &mut files)?;
    files.sort();

    if files.is_empty() {
        println!("No bash scripts found under '*/bash/*.sh' pattern; nothing to lint.");
        return Ok(());
    }

    let mut cmd = Command::new("shellcheck");
    cmd.arg("-x")
        .arg("--exclude=SC1091")
        .arg("--exclude=SC2317")
        .current_dir(workspace_root);
    for f in files {
        cmd.arg(f);
    }
    run_checked(&mut cmd, "shellcheck")
}

fn collect_shell_scripts(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(dir).with_context(|| format!("failed to read dir: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_shell_scripts(&path, out)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let is_sh = path.extension().and_then(OsStr::to_str) == Some("sh");
        let in_bash_tree = path
            .components()
            .any(|c| c.as_os_str() == OsStr::new("bash"));
        if is_sh && in_bash_tree {
            out.push(path);
        }
    }
    Ok(())
}

fn run_probe(workspace_root: &Path, args: Vec<String>) -> Result<()> {
    let input = args.first().map(String::as_str).unwrap_or("AC5");
    let target_url = if input.starts_with("http://") || input.starts_with("https://") {
        input.to_string()
    } else {
        format!(
            "https://www.tefas.gov.tr/tr/fon-detayli-analiz/{}",
            input.to_ascii_uppercase()
        )
    };

    let work_dir = env::temp_dir().join("tefas-probe");
    fs::create_dir_all(&work_dir)
        .with_context(|| format!("failed to create probe dir: {}", work_dir.display()))?;

    let out_cli = work_dir.join("cli_fetch.html");
    let out_curl = work_dir.join("curl_fetch.html");
    let out_imp = work_dir.join("impcurl_fetch.html");
    let log_cli = work_dir.join("cli_fetch.log");

    println!("[probe] target={target_url}");

    println!("[probe] 1/3 tefas-cli (reqwest+rustls default)");
    let cli_output = Command::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("tefas-cli")
        .arg("--")
        .arg("fetch")
        .arg(&target_url)
        .env("TEFAS_DEBUG_HTTP", "1")
        .current_dir(workspace_root)
        .output()
        .context("failed to execute tefas-cli probe run")?;
    let mut log = Vec::new();
    log.extend_from_slice(&cli_output.stdout);
    log.extend_from_slice(&cli_output.stderr);
    fs::write(&log_cli, log)
        .with_context(|| format!("failed to write probe log: {}", log_cli.display()))?;

    let fetched = workspace_root.join("tefas_fetched.html");
    if fetched.exists() {
        let _ = fs::copy(&fetched, &out_cli);
    }
    print_markers(&out_cli, "tefas-cli")?;

    println!("[probe] 2/3 system curl + cookie jar");
    if command_exists("curl") {
        let cookie_jar = work_dir.join("curl.cookies");
        let _ = Command::new("curl")
            .arg("-sS")
            .arg("-L")
            .arg("-c")
            .arg(&cookie_jar)
            .arg("-b")
            .arg(&cookie_jar)
            .arg("https://www.tefas.gov.tr/tr/fon-getirileri")
            .arg("-o")
            .arg(work_dir.join("curl_home.html"))
            .status();
        let _ = Command::new("curl")
            .arg("-sS")
            .arg("-L")
            .arg("-c")
            .arg(&cookie_jar)
            .arg("-b")
            .arg(&cookie_jar)
            .arg(&target_url)
            .arg("-o")
            .arg(&out_curl)
            .status();
        print_markers(&out_curl, "curl")?;
    } else {
        println!("[curl] curl not found in PATH");
    }

    println!("[probe] 3/3 curl-impersonate (if available)");
    if let Some(imp_bin) = resolve_impersonate_binary() {
        let imp_cookie = work_dir.join("impcurl.cookies");
        let _ = Command::new(&imp_bin)
            .arg("-sS")
            .arg("-L")
            .arg("-c")
            .arg(&imp_cookie)
            .arg("-b")
            .arg(&imp_cookie)
            .arg("https://www.tefas.gov.tr/tr/fon-getirileri")
            .arg("-o")
            .arg(work_dir.join("impcurl_home.html"))
            .status();
        let _ = Command::new(&imp_bin)
            .arg("-sS")
            .arg("-L")
            .arg("-c")
            .arg(&imp_cookie)
            .arg("-b")
            .arg(&imp_cookie)
            .arg(&target_url)
            .arg("-o")
            .arg(&out_imp)
            .status();
        print_markers(&out_imp, "impcurl")?;
    } else {
        println!("[impcurl] no impersonation binary found in PATH or ~/.local/bin");
    }

    println!("[probe] done. Working directory: {}", work_dir.display());
    Ok(())
}

fn print_markers(path: &Path, label: &str) -> Result<()> {
    if !path.exists() {
        println!("[{label}] no output file");
        return Ok(());
    }

    let contents = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let failure_config = count_substring(&contents, b"failureConfig");
    let js_gate = count_substring(&contents, b"Please enable JavaScript");
    let title_match = count_substring(&contents, b"Fon Detaylari ve Analizi - TEFAS")
        + count_substring(&contents, b"Fon Detaylar\xC4\xB1 ve Analizi - TEFAS");

    println!(
        "[{label}] failureConfig={failure_config} pleaseEnableJS={js_gate} titleMatch={title_match} file={}",
        path.display()
    );
    Ok(())
}

fn count_substring(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

fn resolve_impersonate_binary() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(home) = env::var("HOME") {
        let base = PathBuf::from(home).join(".local/bin");
        candidates.push(base.join("curl_chrome136"));
        candidates.push(base.join("curl_chrome116"));
        candidates.push(base.join("curl-impersonate-chrome"));
    }

    for p in candidates {
        if p.exists() {
            return Some(p);
        }
    }

    for cmd in [
        "curl_chrome136",
        "curl_chrome116",
        "curl-impersonate-chrome",
    ] {
        if command_exists(cmd) {
            return Some(PathBuf::from(cmd));
        }
    }
    None
}

pub fn print_help() {
    println!(
        "tefas xtask commands:\n  cargo xtask tefas pgo [generate|use|clean]\n  cargo xtask tefas lint\n  cargo xtask tefas probe [FUND_CODE|URL]\n  cargo xtask tefas bench [args...]\n  cargo xtask tefas profile [args...]\n  cargo xtask tefas perf-remote [args...]\n  cargo xtask tefas samply-remote [--host ALIAS --repo PATH --cargo-profile NAME --workload parse|fundpage|query|fetch --output FILE]\n  cargo xtask tefas concurrency-sweep [--host ALIAS --repo PATH --cargo-profile NAME --runs N --levels 1,2,4 --workloads fundpage,query,fetch,parse --query-ops a,b,c --output FILE]\n  cargo xtask tefas ssh-setup [--host USER@HOST --key PATH --alias ALIAS]\n  cargo xtask tefas samply-summary <profile.json|profile.json.gz> [--top N]\n  cargo xtask tefas fuzz [--dry-run] [--binary PATH]\n  cargo xtask tefas test-manual [fpl-toplam|fpl-fonturu]\n  cargo xtask tefas ffi-header"
    );
}

fn run_ffi_header(workspace_root: &Path) -> Result<()> {
    let ffi_dir = workspace_root.join("crates/ffi");
    if !ffi_dir.exists() {
        bail!("tefas-ffi crate directory not found");
    }
    if !command_exists("cbindgen") {
        bail!("cbindgen not found. Install with: cargo install cbindgen");
    }
    run_checked(
        Command::new("cbindgen")
            .arg("--config")
            .arg("cbindgen.toml")
            .arg("--crate")
            .arg("tefas-ffi")
            .arg("--output")
            .arg("include/tefas_ffi.h")
            .current_dir(&ffi_dir),
        "cbindgen generate ffi header",
    )
}

fn run_fuzz(workspace_root: &Path, args: Vec<String>) -> Result<()> {
    let mut dry_run = false;
    let mut binary = workspace_root.join("target/release/tefas-cli");
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--binary" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--binary requires a value");
                };
                binary = PathBuf::from(v);
                i += 2;
            }
            other => bail!("Unknown option: {other}"),
        }
    }

    let operations = [
        "dagilimSiraliGetirT",
        "fonBilgiGetir",
        "fonBuyuklukBazliBilgiGetir",
        "fonDetayGetir",
        "fonFiyatBilgiGetir",
        "fonGetiriBazliBilgiGetir",
        "fonGnlBlgSiraliGetir",
        "fonKurucuGetir",
        "fonProfilDtyGetir",
        "fonTefasDuyuruGetir",
        "fonTipiGetir",
        "fonTurDnmGetiriGetir",
        "fonUnvanAra",
        "fonUnvanGetir",
        "fonYonetimBazliBilgiGetir",
        "getBanners",
        "getBefasFonTurBazliIslemHacmi",
        "getBefasFonTuruBazindaFonSayisi",
        "getBefasToplamIslemHacmi",
        "getBefasUyeBazliIslemHacmi",
        "getFplDovizList",
        "getFplFonBazliIslemHacmi",
        "getFplFonList",
        "getFplFonTuruBazindaFonSayisi",
        "getFplHaftaList",
        "getFplIslemYapanKurumAdet",
        "getFplMkkStokBakiye",
        "getFplToplamIslemHacmi",
        "getFplUyeBazliIslemHacmi",
        "getLogo",
        "isShowedPopup",
        "validate",
    ];
    let old_ops = [
        "bindChartData",
        "bindComparisonFundReturns",
        "bindComparisonFundSizes",
        "bindComparisonManagementFees",
        "bindHistoryAllocation",
        "bindHistoryInfo",
        "getAllFundAnalyzeData",
        "getAllFunds",
    ];
    let backend_tls = [
        ("reqwest", "rustls"),
        ("reqwest", "nativetls"),
        ("reqwest", "insecure"),
        ("hyper", "rustls"),
        ("hyper", "nativetls"),
    ];

    let mut total = 0usize;
    let mut crash = 0usize;
    for op in operations {
        for (backend, tls) in backend_tls {
            total += 1;
            let cmd = vec![
                binary.display().to_string(),
                "--backend".into(),
                backend.into(),
                "--tls".into(),
                tls.into(),
                "query".into(),
                op.into(),
            ];
            if dry_run {
                println!("[dry-run] {}", cmd.join(" "));
            } else if run_and_check_crash(&cmd)? {
                crash += 1;
                break;
            }
        }
        total += 1;
        let cmd = vec![
            binary.display().to_string(),
            "--backend".into(),
            "impcurl".into(),
            "query".into(),
            op.into(),
        ];
        if dry_run {
            println!("[dry-run] {}", cmd.join(" "));
        } else if run_and_check_crash(&cmd)? {
            crash += 1;
        }
    }
    for op in old_ops {
        total += 1;
        let cmd = vec![
            binary.display().to_string(),
            "query".into(),
            "--old".into(),
            op.into(),
        ];
        if dry_run {
            println!("[dry-run] {}", cmd.join(" "));
        } else if run_and_check_crash(&cmd)? {
            crash += 1;
        }
    }
    println!("Fuzz smoke completed: total={total} crash={crash}");
    if crash > 0 {
        bail!("fuzz smoke detected crash exits")
    }
    Ok(())
}

fn run_and_check_crash(cmd: &[String]) -> Result<bool> {
    let Some((exe, rest)) = cmd.split_first() else {
        return Ok(false);
    };
    let out = Command::new(exe).args(rest).output()?;
    if out.status.code() == Some(101) {
        eprintln!("[CRASH] {}", cmd.join(" "));
        return Ok(true);
    }
    Ok(false)
}

fn run_test_manual(workspace_root: &Path, args: Vec<String>) -> Result<()> {
    let case = args.first().map(String::as_str).unwrap_or("fpl-toplam");
    let cli = workspace_root.join("target/debug/tefas-cli");
    if !cli.exists() {
        bail!(
            "Binary not found at {} — run 'cargo build -p tefas-cli' first",
            cli.display()
        );
    }
    match case {
        "fpl-toplam" => {
            run_checked(
                Command::new(&cli)
                    .arg("query")
                    .arg("getFplToplamIslemHacmi")
                    .arg("--set")
                    .arg("basYil=2026")
                    .arg("--set")
                    .arg("basHafta=01")
                    .arg("--set")
                    .arg("bitYil=2026")
                    .arg("--set")
                    .arg("bitHafta=02"),
                "manual test fpl-toplam #1",
            )?;
            run_checked(
                Command::new(&cli)
                    .arg("query")
                    .arg("getFplToplamIslemHacmi")
                    .arg("--set")
                    .arg("basYil=2026")
                    .arg("--set")
                    .arg("basHafta=01")
                    .arg("--set")
                    .arg("bitYil=2026")
                    .arg("--set")
                    .arg("bitHafta=02")
                    .arg("--set")
                    .arg("paraBirimi=USD"),
                "manual test fpl-toplam #2",
            )?;
            run_checked(
                Command::new(&cli)
                    .arg("query")
                    .arg("getFplToplamIslemHacmi")
                    .arg("--set")
                    .arg("basYil=2026")
                    .arg("--set")
                    .arg("basHafta=01")
                    .arg("--set")
                    .arg("bitYil=2026")
                    .arg("--set")
                    .arg("bitHafta=02")
                    .arg("--set")
                    .arg("paraBirimi=EUR"),
                "manual test fpl-toplam #3",
            )?;
        }
        "fpl-fonturu" => {
            run_checked(
                Command::new(&cli)
                    .arg("query")
                    .arg("getFplFonTuruBazindaFonSayisi")
                    .arg("--set")
                    .arg("yil=2025"),
                "manual test fpl-fonturu #1",
            )?;
            run_checked(
                Command::new(&cli)
                    .arg("query")
                    .arg("getFplFonTuruBazindaFonSayisi")
                    .arg("--set")
                    .arg("yil=2025")
                    .arg("--set")
                    .arg("ay=01"),
                "manual test fpl-fonturu #2",
            )?;
            run_checked(
                Command::new(&cli)
                    .arg("query")
                    .arg("getFplFonTuruBazindaFonSayisi")
                    .arg("--set")
                    .arg("yil=2025")
                    .arg("--set")
                    .arg("hafta=07"),
                "manual test fpl-fonturu #3",
            )?;
        }
        other => bail!("unknown test-manual case: {other}. expected fpl-toplam|fpl-fonturu"),
    }
    Ok(())
}

fn run_ssh_setup(args: Vec<String>) -> Result<()> {
    let mut remote_host = env::var("REMOTE_HOST").unwrap_or_else(|_| "192.168.238.128".to_string());
    let mut remote_user = env::var("REMOTE_USER").unwrap_or_else(|_| "developer".to_string());
    let mut ssh_key = env::var("SSH_KEY")
        .unwrap_or_else(|_| format!("{}/.ssh/id_ed25519", env::var("HOME").unwrap_or_default()));
    let mut ssh_alias = env::var("SSH_ALIAS").unwrap_or_else(|_| "tefas-vm-ip".to_string());

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                let Some(v) = args.get(i + 1) else {
                    bail!("--host requires user@host")
                };
                if let Some((u, h)) = v.split_once('@') {
                    remote_user = u.to_string();
                    remote_host = h.to_string();
                } else {
                    bail!("--host requires user@host");
                }
                i += 2;
            }
            "--key" => {
                ssh_key = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("--key requires value"))?;
                i += 2;
            }
            "--alias" => {
                ssh_alias = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("--alias requires value"))?;
                i += 2;
            }
            other => bail!("Unknown option: {other}"),
        }
    }

    let key_path = PathBuf::from(&ssh_key);
    if !key_path.exists() {
        run_checked(
            Command::new("ssh-keygen")
                .arg("-t")
                .arg("ed25519")
                .arg("-f")
                .arg(&ssh_key)
                .arg("-N")
                .arg(""),
            "ssh-keygen",
        )?;
    }
    run_checked(
        Command::new("ssh-copy-id")
            .arg("-i")
            .arg(format!("{}.pub", ssh_key))
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg(format!("{}@{}", remote_user, remote_host)),
        "ssh-copy-id",
    )?;

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let ssh_dir = PathBuf::from(home).join(".ssh");
    fs::create_dir_all(&ssh_dir)?;
    let cfg = ssh_dir.join("config");
    let mut content = fs::read_to_string(&cfg).unwrap_or_default();
    if !content.contains(&format!("Host {ssh_alias}")) {
        content.push_str(&format!("\nHost {alias}\n    HostName {host}\n    User {user}\n    IdentityFile {key}\n    ServerAliveInterval 60\n", alias=ssh_alias, host=remote_host, user=remote_user, key=ssh_key));
        fs::write(&cfg, content)?;
    }
    run_checked(
        Command::new("ssh")
            .arg("-o")
            .arg("BatchMode=yes")
            .arg(&ssh_alias)
            .arg("echo OK"),
        "verify passwordless ssh",
    )
}

fn run_concurrency_sweep(workspace_root: &Path, args: Vec<String>) -> Result<()> {
    let mut remote_alias = env::var("REMOTE_ALIAS").unwrap_or_else(|_| "tefas-vm-ip".to_string());
    let mut remote_repo = env::var("REMOTE_REPO")
        .unwrap_or_else(|_| "/home/developer/Projects/github/TefasKapRequests".to_string());
    let mut profile_name = env::var("PROFILE_NAME").unwrap_or_else(|_| "profileDebug".to_string());
    let mut runs: usize = env::var("RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    let mut levels_csv = env::var("LEVELS_CSV").unwrap_or_else(|_| "1,2,4,8".to_string());
    let mut workloads_csv =
        env::var("WORKLOADS_CSV").unwrap_or_else(|_| "fundpage,query,fetch,parse".to_string());
    let mut query_ops_csv = env::var("QUERY_OPS_CSV").unwrap_or_else(|_| {
        "fonBilgiGetir,fonFiyatBilgiGetir,fonGetiriBazliBilgiGetir,fonBuyuklukBazliBilgiGetir"
            .to_string()
    });
    let mut output: Option<PathBuf> = None;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                remote_alias = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("--host requires value"))?;
                i += 2;
            }
            "--repo" => {
                remote_repo = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("--repo requires value"))?;
                i += 2;
            }
            "--cargo-profile" => {
                profile_name = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("--cargo-profile requires value"))?;
                i += 2;
            }
            "--runs" => {
                runs = args
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| anyhow::anyhow!("--runs requires int"))?;
                i += 2;
            }
            "--levels" => {
                levels_csv = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("--levels requires value"))?;
                i += 2;
            }
            "--workloads" => {
                workloads_csv = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("--workloads requires value"))?;
                i += 2;
            }
            "--query-ops" => {
                query_ops_csv = args
                    .get(i + 1)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("--query-ops requires value"))?;
                i += 2;
            }
            "--output" => {
                output = Some(PathBuf::from(
                    args.get(i + 1)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("--output requires value"))?,
                ));
                i += 2;
            }
            other => bail!("Unknown option: {other}"),
        }
    }

    let local_bin = resolve_local_profile_bin(workspace_root, &profile_name);
    if !local_bin.exists() {
        run_checked(
            Command::new("cargo")
                .arg("build")
                .arg("--profile")
                .arg(&profile_name)
                .arg("-p")
                .arg("tefas-cli")
                .current_dir(workspace_root),
            "build local tefas-cli",
        )?;
    }
    let remote_bin = format!("{}/target/perf-bin/{}/tefas-cli", remote_repo, profile_name);
    run_checked(
        Command::new("ssh").arg(&remote_alias).arg(format!(
            "mkdir -p {}",
            sh_quote(&format!("{}/target/perf-bin/{}", remote_repo, profile_name))
        )),
        "prepare remote perf-bin dir",
    )?;
    run_checked(
        Command::new("rsync")
            .arg("-az")
            .arg(&local_bin)
            .arg(format!("{}:{}", remote_alias, remote_bin)),
        "upload binary",
    )?;

    let ts = timestamp_string();
    let out_file = output.unwrap_or_else(|| {
        workspace_root
            .join("target/perf-artifacts-remote/concurrency-sweep")
            .join(format!("sweep_{ts}.tsv"))
    });
    if let Some(parent) = out_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &out_file,
        "timestamp\tworkload\tconcurrency\trun\tseconds\n",
    )?;

    let remote_script = format!(
        "set -euo pipefail\ncd {repo}\nBIN={bin}\nRUNS={runs}\nLEVELS='{levels}'\nWORKLOADS='{workloads}'\nQUERY_OPS='{qops}'\nTS_REMOTE=$(date +%Y%m%d_%H%M%S)\ncontains_workload() {{ local name=\"$1\"; [[ \",$WORKLOADS,\" == *\",$name,\"* ]]; }}\nrun_timed() {{ local cmd=\"$1\"; {{ /usr/bin/time -f \"%e\" bash -lc \"$cmd\" >/dev/null; }} 2>&1; }}\nIFS=',' read -r -a LEVEL_ARR <<< \"$LEVELS\"\nIFS=',' read -r -a QUERY_OPS_ARR <<< \"$QUERY_OPS\"\nfor c in \"${{LEVEL_ARR[@]}}\"; do for ((i=1;i<=RUNS;i++)); do if contains_workload fundpage; then sec=$(run_timed \"$BIN --quiet --network-concurrency $c fundpage AC5 TLY TAR ZFB --output /tmp/fundpage_sweep.json\"); echo -e \"$TS_REMOTE\tfundpage\t$c\t$i\t$sec\"; fi; if contains_workload query; then query_cmd=\"$BIN --quiet --network-concurrency $c query\"; for op in \"${{QUERY_OPS_ARR[@]}}\"; do query_cmd+=\" $op\"; done; sec=$(run_timed \"$query_cmd > /tmp/query_sweep.json\"); echo -e \"$TS_REMOTE\tquery\t$c\t$i\t$sec\"; fi; if contains_workload fetch; then sec=$(run_timed \"$BIN --quiet --network-concurrency $c fetch https://www.tefas.gov.tr/tr/fon-detayli-analiz/AC5 https://www.tefas.gov.tr/tr/fon-detayli-analiz/TLY --output /tmp/fetch_sweep\"); echo -e \"$TS_REMOTE\tfetch\t$c\t$i\t$sec\"; fi; if contains_workload parse; then sec=$(run_timed \"$BIN --quiet --parse-concurrency $c parse datasets/fundpage/html/*.html --output /tmp/parse_sweep.json\"); echo -e \"$TS_REMOTE\tparse\t$c\t$i\t$sec\"; fi; done; done",
        repo = sh_quote(&remote_repo),
        bin = sh_quote(&remote_bin),
        runs = runs,
        levels = levels_csv,
        workloads = workloads_csv,
        qops = query_ops_csv,
    );

    let output_remote = Command::new("ssh")
        .arg(&remote_alias)
        .arg("bash")
        .arg("-lc")
        .arg(remote_script)
        .output()?;
    if !output_remote.status.success() {
        bail!("remote concurrency sweep failed");
    }
    let sweep_body = String::from_utf8_lossy(&output_remote.stdout);
    let mut lines: Vec<String> = sweep_body.lines().map(|s| s.to_string()).collect();
    lines.retain(|l| !l.trim().is_empty());
    let mut content = fs::read_to_string(&out_file)?;
    for l in &lines {
        content.push_str(l);
        content.push('\n');
    }
    fs::write(&out_file, content)?;

    // Median summary in Rust
    let median_file = out_file.with_file_name(format!(
        "{}_median.tsv",
        out_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sweep")
    ));
    let raw = fs::read_to_string(&out_file)?;
    let mut groups: std::collections::BTreeMap<(String, u32), Vec<f64>> =
        std::collections::BTreeMap::new();
    for (idx, line) in raw.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 {
            continue;
        }
        let workload = cols[1].to_string();
        let conc: u32 = match cols[2].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let secs: f64 = match cols[4].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        groups.entry((workload, conc)).or_default().push(secs);
    }
    let mut out = String::from("workload\tconcurrency\tmedian_seconds\n");
    for ((w, c), vals) in &mut groups {
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let m = if vals.len() % 2 == 1 {
            vals[vals.len() / 2]
        } else {
            (vals[vals.len() / 2 - 1] + vals[vals.len() / 2]) / 2.0
        };
        out.push_str(&format!("{}\t{}\t{:.6}\n", w, c, m));
    }
    fs::write(&median_file, out)?;
    println!("Saved sweep results: {}", out_file.display());
    println!("Saved median summary: {}", median_file.display());
    Ok(())
}

fn run_samply_summary(args: Vec<String>) -> Result<()> {
    let Some(input) = args.first() else {
        bail!("Usage: cargo xtask tefas samply-summary <profile.json|profile.json.gz> [--top N]");
    };
    let mut top: usize = 15;
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--top" => {
                top = args
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| anyhow::anyhow!("--top requires integer"))?;
                i += 2;
            }
            other => bail!("Unknown argument: {other}"),
        }
    }
    let path = PathBuf::from(input);
    let mut json_text = String::new();
    if input.ends_with(".gz") {
        let file = fs::File::open(&path)?;
        let mut gz = GzDecoder::new(file);
        gz.read_to_string(&mut json_text)?;
    } else {
        json_text = fs::read_to_string(&path)?;
    }
    let v: serde_json::Value = serde_json::from_str(&json_text)?;
    let threads = v
        .get("threads")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow::anyhow!("invalid profile: missing threads"))?;
    let mut counts: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for thread in threads {
        let string_array: Vec<String> = thread
            .get("stringArray")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| x.as_str().unwrap_or("<unknown>").to_string())
                    .collect()
            })
            .unwrap_or_default();
        let frame_table = thread.get("frameTable").and_then(|x| x.as_object());
        let func_table = thread.get("funcTable").and_then(|x| x.as_object());
        let samples = thread.get("samples").and_then(|x| x.as_object());
        let Some(samples) = samples else { continue };
        let stack_idx = samples
            .get("stack")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let weights = samples
            .get("weight")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let stack_table = thread.get("stackTable").and_then(|x| x.as_object());
        let Some(stack_table) = stack_table else {
            continue;
        };
        let frame_col = stack_table
            .get("frame")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let func_col = frame_table
            .and_then(|ft| ft.get("func"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let func_names = func_table
            .and_then(|ft| ft.get("name"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        for (idx, si) in stack_idx.iter().enumerate() {
            let Some(si) = si.as_u64() else { continue };
            let si = si as usize;
            let fi = frame_col.get(si).and_then(|x| x.as_u64()).unwrap_or(0) as usize;
            let fu = func_col.get(fi).and_then(|x| x.as_u64()).unwrap_or(0) as usize;
            let name_idx = func_names.get(fu).and_then(|x| x.as_u64()).unwrap_or(0) as usize;
            let sym = string_array
                .get(name_idx)
                .cloned()
                .unwrap_or_else(|| "<unknown>".to_string());
            let w = weights.get(idx).and_then(|x| x.as_f64()).unwrap_or(1.0);
            *counts.entry(sym).or_insert(0.0) += w;
        }
    }
    let total: f64 = counts.values().sum();
    let mut items: Vec<(String, f64)> = counts.into_iter().collect();
    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    println!("Top {} hotspots (total weight: {:.0})", top, total);
    for (i, (sym, cnt)) in items.into_iter().take(top).enumerate() {
        let pct = if total > 0.0 {
            cnt * 100.0 / total
        } else {
            0.0
        };
        println!("{:>3}. {:>10.0} {:>5.1}% {}", i + 1, cnt, pct, sym);
    }
    Ok(())
}
