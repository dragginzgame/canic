//! Module: operational_readiness
//!
//! Responsibility: own the maintained persistence and restore proof inventory.
//! Does not own: production backup or restore behavior.
//! Boundary: binds retained crash-case identities to focused executable proofs.

pub mod manifest;
