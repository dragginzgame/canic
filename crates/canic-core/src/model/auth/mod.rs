//! Module: model::auth
//!
//! Responsibility: own authoritative delegated-auth runtime state shapes.
//! Does not own: policy decisions, stable-record conversion, or storage access.
//! Boundary: workflow and policy inspect model values; ops persists and projects them.

mod chain_key_root_delegation;
mod root_issuer;

pub use chain_key_root_delegation::ChainKeyRootDelegationInstallFailure;
pub use root_issuer::{
    RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
    RootIssuerRenewalAttempt, RootIssuerRenewalAttemptStatus, RootIssuerRenewalOutcome,
    RootIssuerRenewalProofRef, RootIssuerRenewalState, RootIssuerRenewalTemplate,
};
