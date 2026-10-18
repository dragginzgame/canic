//! Module: domain::policy::pure::auth
//!
//! Responsibility: pure auth issuance policy decisions.
//! Does not own: proof verification, storage access, replay, or signing.
//! Boundary: called by workflow before auth ops prepare Root delegation proofs.

pub mod application_authorization;
mod root_provisioning;

use crate::domain::value::Principal;
use canic_contracts::ids::CanisterRole;
use thiserror::Error as ThisError;

pub use root_provisioning::{
    RootDelegationProofPreparePolicyInput, validate_root_delegation_proof_prepare_policy,
    validate_root_issuer_policy_fleet_binding, validate_root_issuer_policy_upsert,
    validate_root_issuer_renewal_template_fleet_binding,
    validate_root_issuer_renewal_template_upsert,
};

///
/// AuthPolicyError
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum AuthPolicyError {
    #[error("root issuer audience is not allowed for issuer {issuer_pid}")]
    RootIssuerAudienceNotAllowed { issuer_pid: Principal },

    #[error("root issuer audience must match the protected Fleet")]
    RootIssuerFleetMismatch,

    #[error("enabled root issuer policy must allow at least one audience")]
    RootIssuerAudienceRequired,

    #[error("root issuer certificate TTL must be greater than zero")]
    RootIssuerCertTtlZero,

    #[error(
        "root issuer certificate TTL {cert_ttl_ns} exceeds max certificate TTL {max_cert_ttl_ns}"
    )]
    RootIssuerCertTtlExceedsMax {
        cert_ttl_ns: u64,
        max_cert_ttl_ns: u64,
    },

    #[error("root issuer {issuer_pid} is disabled")]
    RootIssuerDisabled { issuer_pid: Principal },

    #[error("root issuer grant scope '{scope}' is not allowed for role {role}")]
    RootIssuerGrantNotAllowed { role: CanisterRole, scope: String },

    #[error("enabled root issuer policy must allow at least one grant")]
    RootIssuerGrantRequired,

    #[error("root issuer max certificate TTL must be greater than zero")]
    RootIssuerMaxCertTtlZero,

    #[error("root issuer policy is for {expected}, but request named issuer {found}")]
    RootIssuerPolicyMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("root issuer refresh-after offset must be within the certificate TTL")]
    RootIssuerRefreshAfterInvalid,

    #[error("root issuer refresh-after timestamp overflows nanoseconds")]
    RootIssuerRefreshAfterOverflow,

    #[error("root issuer refresh ratio must be between 1 and 9999 basis points")]
    RootIssuerRefreshRatioInvalid { refresh_after_ratio_bps: u16 },

    #[error("root issuer is not registered")]
    RootIssuerUnregistered,

    #[error("enabled root issuer renewal template must include at least one grant")]
    RootIssuerRenewalGrantRequired,
}
