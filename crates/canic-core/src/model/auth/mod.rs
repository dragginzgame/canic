//! Module: model::auth
//!
//! Responsibility: own authoritative delegated-auth runtime state shapes.
//! Does not own: policy decisions, stable-record conversion, or storage access.
//! Boundary: workflow and policy inspect model values; ops persists and projects them.

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "B2 model values are consumed by the sequenced B3-B5 runtime batches"
    )
)]
pub mod application_authorization;
mod chain_key_root_delegation;
mod root_issuer;

pub use chain_key_root_delegation::ChainKeyRootDelegationInstallFailure;
pub use root_issuer::{
    RootDelegatedRoleGrantPolicy, RootIssuerPolicy, RootIssuerRenewalState,
    RootIssuerRenewalTemplate,
};
