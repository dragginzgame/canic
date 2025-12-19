//! Small deterministic helpers used across Canic.
//!
//! This crate intentionally stays lightweight: helpers are designed to be
//! replica-friendly and predictable, and avoid pulling in higher-level Canic
//! orchestration concerns.
//!
//! Modules:
//! - [`case`] – string casing helpers.
//! - [`format`] – small formatting helpers for logs/UI.
//! - [`hash`] – fast xxHash3 hashing (non-cryptographic).
//! - [`instructions`] – formatting helpers for instruction counts.
//! - [`rand`] – ChaCha20 PRNG seeded via the management canister `raw_rand`.

pub mod case;
pub mod format;
pub mod hash;
pub mod instructions;
pub mod rand;
