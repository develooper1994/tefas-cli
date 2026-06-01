# SemVer Policy

This crate follows Semantic Versioning for the public Rust API exposed by `tefas`.

## Versioning Rules

- `MAJOR`: breaking changes in public API surface or behavior.
- `MINOR`: backward-compatible new APIs or capabilities.
- `PATCH`: backward-compatible bug fixes and documentation updates.

## What is Considered Public API

- Public items re-exported from `crates/lib/src/lib.rs`.
- Public methods on `TefasClient` and runtime wrappers.
- Public request/plan structs intended for integrators.

## Breaking Change Examples

- Removing/renaming a public type, field, enum variant, or method.
- Changing required method arguments.
- Changing result shape in a way that breaks existing consumers.

## Non-breaking Change Examples

- Adding new methods or optional fields.
- Improving retry/backoff internals without API changes.
- Adding examples and documentation.

## Compatibility Notes

- Network responses from TEFAS endpoints are external contracts and may evolve.
- The library aims to keep parser/query/fetch wrappers stable even when endpoint payloads evolve.
