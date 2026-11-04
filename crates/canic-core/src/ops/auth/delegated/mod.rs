//! Module: ops::auth::delegated
//!
//! Responsibility: group Root delegation certificate projection, signing and proof helpers.
//! Does not own: endpoint authorization, stable auth records, or runtime config.
//! Boundary: private auth-ops support for Root delegation and issuer proof flows.

pub(super) mod active_proof;
pub(super) mod audience;
pub(super) mod canonical;
pub(super) mod cert_rules;
pub(super) mod chain_key;
pub(super) mod chain_key_signing;
pub(super) mod delegation_cert;
pub(super) mod protocol;
pub(super) mod root_proof;
#[cfg(test)]
pub mod test_fixtures;
