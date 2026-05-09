//! Compatibility adapter crate.
//!
//! Network/TLS backend implementation now lives in the shared [http-client] crate.
//! This crate re-exports the public API for backward compatibility.

pub use http_client::*;
