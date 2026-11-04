//! Module: model::auth
//!
//! Responsibility: own authoritative delegated-auth runtime state shapes.
//! Does not own: policy decisions, stable-record conversion, or storage access.
//! Boundary: workflow and policy inspect model values; ops persists and projects them.

pub mod application_authorization;
mod chain_key_root_delegation;
#[cfg(any(test, feature = "auth-portable-issuer"))]
pub mod portable_issuer;
#[cfg(any(test, feature = "auth-root-delegation-state"))]
pub mod portable_registration;
mod root_issuer;

pub use chain_key_root_delegation::ChainKeyRootDelegationInstallFailure;
pub use root_issuer::{
    RootDelegatedRoleGrantPolicy, RootIssuerPolicy, RootIssuerRenewalState,
    RootIssuerRenewalTemplate,
};
