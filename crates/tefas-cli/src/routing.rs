use clap::ValueEnum;
use std::path::{Path, PathBuf};
use tefas::{Operation, OperationOld};

fn dedup_push(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if !candidates.iter().any(|candidate| candidate == &path) {
        candidates.push(path);
    }
}

fn bundled_fixture_candidates(file_name: &Path) -> Vec<PathBuf> {
    let mut search_roots = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        dedup_push(&mut search_roots, cwd.clone());
        for ancestor in cwd.ancestors() {
            dedup_push(&mut search_roots, ancestor.to_path_buf());
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dedup_push(&mut search_roots, manifest_dir.clone());
    for ancestor in manifest_dir.ancestors() {
        dedup_push(&mut search_roots, ancestor.to_path_buf());
    }

    let mut candidates = Vec::new();
    for root in search_roots {
        dedup_push(
            &mut candidates,
            root.join("etl")
                .join("shared")
                .join("datasets")
                .join("tefas")
                .join("fundpage")
                .join("html")
                .join(file_name),
        );
        dedup_push(
            &mut candidates,
            root.join("shared")
                .join("datasets")
                .join("tefas")
                .join("fundpage")
                .join("html")
                .join(file_name),
        );
        dedup_push(
            &mut candidates,
            root.join("datasets")
                .join("fundpage")
                .join("html")
                .join(file_name),
        );
    }

    candidates
}

/// How `fundpage` should dispatch its JSON outputs.
pub(crate) enum FundpageOutputPlan {
    /// Single code, no --output flag -> write to stdout.
    Stdout,
    /// 1-path --output -> merge all codes into one file.
    MergedFile(PathBuf),
    /// Per-code files: (uppercase_code, dest_path) in original code order.
    PerCode(Vec<(String, PathBuf)>),
}

pub(crate) fn resolve_fundpage_outputs(
    codes: &[String],
    output: Option<Vec<String>>,
) -> anyhow::Result<FundpageOutputPlan> {
    if output.is_none() && codes.len() == 1 {
        return Ok(FundpageOutputPlan::Stdout);
    }

    let paths = output.unwrap_or_default();
    let code_count = codes.len();
    match paths.len() {
        0 => {
            let per = codes
                .iter()
                .map(|c| {
                    let uc = c.to_uppercase();
                    let path = PathBuf::from(format!("{}.json", uc));
                    (uc, path)
                })
                .collect();
            Ok(FundpageOutputPlan::PerCode(per))
        }
        1 => Ok(FundpageOutputPlan::MergedFile(PathBuf::from(&paths[0]))),
        n if n == code_count => Ok(FundpageOutputPlan::PerCode(
            codes
                .iter()
                .zip(paths.iter())
                .map(|(c, p)| (c.to_uppercase(), PathBuf::from(p)))
                .collect(),
        )),
        n => Err(anyhow::anyhow!(
            "--output: {} path(s) given but {} code(s) specified; expected 0, 1, or {} path(s)",
            n,
            code_count,
            code_count
        )),
    }
}

pub(crate) fn resolve_save_html_paths(
    codes: &[String],
    save_html: Option<Vec<String>>,
) -> anyhow::Result<Option<Vec<PathBuf>>> {
    let paths = match save_html {
        None => return Ok(None),
        Some(p) => p,
    };
    let resolved = match paths.len() {
        0 => codes
            .iter()
            .map(|c| PathBuf::from(format!("{}.html", c.to_uppercase())))
            .collect(),
        1 => {
            let dir = PathBuf::from(&paths[0]);
            codes
                .iter()
                .map(|c| dir.join(format!("{}.html", c.to_uppercase())))
                .collect()
        }
        n if n == codes.len() => paths.iter().map(PathBuf::from).collect(),
        n => {
            return Err(anyhow::anyhow!(
                "--save-html: {} path(s) given but {} code(s) specified; expected 0, 1, or {} path(s)",
                n,
                codes.len(),
                codes.len()
            ));
        }
    };
    Ok(Some(resolved))
}

pub(crate) fn url_to_filename(url: &str, index: usize) -> String {
    let segment = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");
    let clean: String = segment
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let clean = clean.trim_matches('_').to_string();
    if clean.is_empty() {
        format!("fetched_{}.html", index)
    } else if clean.ends_with(".html") || clean.ends_with(".htm") {
        clean
    } else {
        format!("{}.html", clean)
    }
}

/// How `fetch` should dispatch its outputs.
pub(crate) enum FetchOutputPlan {
    DefaultSingle,
    PerUrl(Vec<(String, PathBuf)>),
}

pub(crate) fn resolve_fetch_outputs(
    urls: &[String],
    output: Option<Vec<String>>,
) -> anyhow::Result<FetchOutputPlan> {
    let paths = output.unwrap_or_default();
    match paths.len() {
        0 => {
            if urls.len() == 1 {
                Ok(FetchOutputPlan::DefaultSingle)
            } else {
                let per = urls
                    .iter()
                    .enumerate()
                    .map(|(i, u)| (u.clone(), PathBuf::from(url_to_filename(u, i))))
                    .collect();
                Ok(FetchOutputPlan::PerUrl(per))
            }
        }
        1 => {
            let dir = PathBuf::from(&paths[0]);
            let per = urls
                .iter()
                .enumerate()
                .map(|(i, u)| (u.clone(), dir.join(url_to_filename(u, i))))
                .collect();
            Ok(FetchOutputPlan::PerUrl(per))
        }
        n if n == urls.len() => {
            let per = urls
                .iter()
                .zip(paths.iter())
                .map(|(u, p)| (u.clone(), PathBuf::from(p)))
                .collect();
            Ok(FetchOutputPlan::PerUrl(per))
        }
        n => Err(anyhow::anyhow!(
            "--output: {} path(s) given but {} URL(s) specified; expected 0, 1, or {} path(s)",
            n,
            urls.len(),
            urls.len()
        )),
    }
}

pub(crate) fn resolve_parse_input_path(input: &str) -> anyhow::Result<PathBuf> {
    let direct = PathBuf::from(input);
    if direct.exists() {
        return Ok(direct);
    }

    if let Some(file_name) = direct.file_name() {
        let candidates = bundled_fixture_candidates(Path::new(file_name));
        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }

        let hint = candidates
            .first()
            .map(|candidate| candidate.display().to_string())
            .unwrap_or_else(|| "datasets/tefas/fundpage/html/<FON>.html".to_string());

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        anyhow::bail!(
            "Input file not found: {} (cwd: {}). If you meant a bundled fixture, try: {}",
            input,
            cwd.display(),
            hint
        );
    }

    anyhow::bail!(
        "Input file not found: {}. Provide a direct path or a fixture file name like AC5.html.",
        input
    );
}

pub(crate) enum AnyOperation {
    New(Operation),
    Old(OperationOld),
}

pub(crate) fn find_operation(name: &str) -> Option<AnyOperation> {
    for &op in Operation::value_variants() {
        if let Some(pv) = op.to_possible_value()
            && pv.matches(name, true)
        {
            return Some(AnyOperation::New(op));
        }
    }
    for &op in OperationOld::value_variants() {
        if let Some(pv) = op.to_possible_value()
            && pv.matches(name, true)
        {
            return Some(AnyOperation::Old(op));
        }
    }
    None
}
