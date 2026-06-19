//! Module: ops::auth::delegated
//!
//! Responsibility: group delegated auth canonicalization, proof, token, and cache helpers.
//! Does not own: endpoint authorization, stable auth records, or runtime config.
//! Boundary: private auth-ops support for delegated-token and delegation-proof flows.

pub(super) mod active_proof;
mod audience;
pub(super) mod cache;
pub(super) mod canonical;
pub(super) mod cert_rules;
pub(super) mod delegation_cert;
pub(super) mod prepare;
pub(super) mod verify;
