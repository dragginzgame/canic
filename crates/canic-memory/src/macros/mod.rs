//! Macro modules backing the crate-level exported macros.

/// Stable-memory slot and owner-range declaration macros.
pub mod memory;
/// Eager TLS and eager-init registration macros.
pub mod runtime;
/// `Storable` implementation macros backed by the shared CBOR serializer.
pub mod storable;
