//! Small deterministic helpers used across Canic.
//!
//! This crate intentionally stays lightweight: helpers are designed to be
//! replica-friendly and predictable, and avoid pulling in higher-level Canic
//! orchestration concerns.
//!
//! Modules:
//! - [`case`] – optional string casing helpers when the `case` feature is enabled.
//! - [`format`] – small formatting helpers for logs/UI.
//! - [`hash`] – fast xxHash3 hashing (non-cryptographic).
//! - [`instructions`] – formatting helpers for instruction counts.
//! - [`rand`] – optional ChaCha20 PRNG seeded externally when the `rand` feature is enabled.

#[cfg(feature = "case")]
pub mod case;
pub mod format;
pub mod hash;
pub mod instructions;
#[cfg(feature = "rand")]
pub mod rand;
