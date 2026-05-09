//! curl-impersonate helper facade.
//!
//! This module keeps the old `impcurl::*` call sites stable while the internals
//! are split into focused files under `src/impcurl/`.

#[path = "impcurl/discovery.rs"]
mod discovery;
#[path = "impcurl/process_fallback.rs"]
mod process_fallback;
#[path = "impcurl/transport.rs"]
mod transport;

pub(crate) use discovery::{find_binary, lib_dir_for};
pub(crate) use process_fallback::run;
pub(crate) use transport::{
    IMPCURL_IMPERSONATE_FALLBACK_WARNED, IMPCURL_PROCESS_FALLBACK_WARNED, unknown_impersonate,
};
