//! Developer tooling utilities: dependency installation with multi-strategy fallback.

mod command;
mod download;
mod installer;

pub use installer::{InstallOptions, install_with_fallback};

pub const TOOLS_VERSION: &str = env!("CARGO_PKG_VERSION");
