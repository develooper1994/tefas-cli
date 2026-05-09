//! Typed output structures for the TEFAS fund page parser.
//!
//! These types provide a structured alternative to the raw `(Value, Value)` tuple returned
//! by [`parse_document`][super::parse_document].  They add zero runtime overhead — they are
//! thin wrappers that name each position and document what it contains.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Metadata associated with a TEFAS fund page (the second element of the raw tuple).
///
/// Contains extra fields extracted during parsing that are not part of the main fund data
/// groups — for example Next.js RSC fast-path fields, page-level identifiers, and any
/// additional key/value pairs that do not belong to a structured section.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FundPageMeta {
    /// Raw JSON object.  Keys and values vary per page version.
    pub fields: Value,
}

/// Typed output of the TEFAS fund page parser.
///
/// Wraps the `(grouped, meta)` tuple from [`parse_document`][super::parse_document] and
/// gives each component a descriptive name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FundPageOutput {
    /// Grouped fund data with sections: `indicator`, `profile`, `return`, `history`.
    pub data: Value,
    /// Extra page-level metadata fields.
    pub meta: FundPageMeta,
}

impl FundPageOutput {
    /// Construct a `FundPageOutput` from the raw `(data, meta_fields)` tuple returned by
    /// [`parse_document`][super::parse_document].
    pub fn from_raw(data: Value, meta_fields: Value) -> Self {
        Self {
            data,
            meta: FundPageMeta {
                fields: meta_fields,
            },
        }
    }

    /// Consume the typed output and return the underlying `(data, meta_fields)` tuple,
    /// matching the shape of [`parse_document`][super::parse_document].
    pub fn into_raw(self) -> (Value, Value) {
        (self.data, self.meta.fields)
    }
}
