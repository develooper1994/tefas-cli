use anyhow::{Result, bail};
use std::env;

mod common;
mod tefas;
mod tools;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        print_help();
        return Ok(());
    };

    let workspace_root = common::workspace_root()?;

    match first.as_str() {
        "tefas" => {
            let task = args.next().unwrap_or_else(|| "help".to_string());
            tefas::run(&workspace_root, &task, args.collect())
        }
        "tools" => {
            let task = args.next().unwrap_or_else(|| "help".to_string());
            tools::run(&workspace_root, &task, args.collect())
        }
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        // Backward compatibility: existing top-level commands map to tefas namespace.
        legacy => {
            let mut passthrough: Vec<String> = args.collect();
            match legacy {
                "pgo" | "lint" | "probe" | "bench" | "profile" | "perf-remote"
                | "samply-remote" => tefas::run(&workspace_root, legacy, std::mem::take(&mut passthrough)),
                _ => bail!("unknown xtask command: {legacy}"),
            }
        }
    }
}

fn print_help() {
    println!(
        "xtask namespaces:\n  cargo xtask tefas <command>\n  cargo xtask tools <command>\n\nBackward compatible shortcuts:\n  cargo xtask pgo|lint|probe|bench|profile|perf-remote|samply-remote\n"
    );
    tefas::print_help();
    tools::print_help();
}
