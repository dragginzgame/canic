//! Module: operational_readiness
//!
//! Responsibility: own the test-only executable 0.94 recovery protocol.
//! Does not own: production backup or restore behavior.
//! Boundary: binds frozen crash-case identities to focused executable proofs.

pub mod manifest;
