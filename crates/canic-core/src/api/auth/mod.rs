//! Module: api::auth
//!
//! Responsibility: expose auth endpoint helpers and auth boundary adapters.
//! Does not own: stable auth records, proof verification internals, or runtime policy.
//! Boundary: endpoint layer maps public DTOs into ops/workflow auth calls.

use crate::{dto::error::Error, ops::config::ConfigOps};

#[cfg(any(
    feature = "auth-root-canister-sig-create",
    feature = "auth-issuer-canister-sig-create"
))]
pub use ic_auth::signature_store::SignatureSibling;

// Internal auth pipeline:
// - `attestation` owns role-attestation endpoint adapters.
// - `root` owns root-only issuer policy, renewal, and chain-key proof adapters.
// - `issuer_proof` owns issuer-local Root proof installation and reporting.
mod attestation;
mod issuer_proof;
#[cfg(feature = "auth-portable-issuer")]
pub mod portable_issuer;
mod root;

///
/// AuthApi
///
/// Owns Root proof support, role attestation and selected portable receiver adapters.
/// Owned by the API layer and called by generated endpoint wrappers.
///

pub struct AuthApi;

impl AuthApi {
    /// Publish trusted application sibling roots alongside Canic's one /sig tree.
    /// Call synchronously after restoration and whenever an application branch changes.
    /// Siblings are innermost first and must reserve /sig for Canic's shared store.
    #[cfg(any(
        feature = "auth-root-canister-sig-create",
        feature = "auth-issuer-canister-sig-create"
    ))]
    pub fn set_canister_signature_siblings(siblings: Vec<SignatureSibling>) -> Result<(), Error> {
        crate::ops::auth::signature_store::set_siblings(siblings).map_err(Self::map_auth_error)
    }

    // Map internal auth failures onto public endpoint errors.
    fn map_auth_error(err: crate::InternalError) -> Error {
        Error::from(err)
    }

    fn require_issuer_proof_support() -> Result<(), Error> {
        let delegated_tokens_cfg =
            ConfigOps::delegated_tokens_config().map_err(Self::map_auth_error)?;
        if !delegated_tokens_cfg.enabled {
            return Err(Error::from_registered(
                crate::diagnostics::codes::REQUEST_INVALID,
            ));
        }

        let canister_cfg = ConfigOps::current_canister().map_err(Self::map_auth_error)?;
        if !canister_cfg.auth.delegated_token_issuer {
            return Err(Error::from_registered(
                crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
            ));
        }

        Ok(())
    }
}
#[cfg(feature = "auth-local-application-authorization")]
pub mod portable;
