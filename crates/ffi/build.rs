use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn find_cbindgen() -> Option<PathBuf> {
    if let Ok(explicit) = env::var("CBINDGEN") {
        let candidate = PathBuf::from(explicit);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }

    if Command::new("cbindgen")
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
    {
        return Some(PathBuf::from("cbindgen"));
    }

    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));

    cargo_home
        .map(|home| home.join("bin").join("cbindgen"))
        .filter(|candidate| is_executable(candidate))
}

fn main() {
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-env-changed=CBINDGEN");

    let crate_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()));
    let cfg = crate_dir.join("cbindgen.toml");
    let out = crate_dir.join("include/tefas_ffi.h");

    let Some(cbindgen) = find_cbindgen() else {
        println!("cargo:warning=cbindgen not found; skipping header generation");
        return;
    };

    let output = Command::new(&cbindgen)
        .arg("--config")
        .arg(&cfg)
        .arg("--crate")
        .arg("tefas-ffi")
        .arg("--output")
        .arg(&out)
        .current_dir(&crate_dir)
        .output();

    match output {
        Ok(result) if result.status.success() => {}
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            println!(
                "cargo:warning=cbindgen failed while generating include/tefas_ffi.h: {}",
                stderr.trim()
            );
        }
        Err(err) => println!("cargo:warning=failed to execute cbindgen: {err}"),
    }
}
