//! Module: domain::policy::pure::auth
//!
//! Responsibility: pure auth issuance policy decisions.
//! Does not own: proof verification, storage access, replay, or signing.
//! Boundary: called by workflow before auth ops prepare delegated-token proofs.

use crate::{
    domain::value::Principal,
    ids::{CanisterRole, cap},
};
use thiserror::Error as ThisError;

mod root_provisioning;

pub use root_provisioning::{
    RootDelegationProofPreparePolicyDecision, RootDelegationProofPreparePolicyInput,
    validate_root_delegation_proof_prepare_policy, validate_root_issuer_policy_upsert,
    validate_root_issuer_renewal_template_upsert,
};

///
/// DelegatedRoleGrantPolicy
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegatedRoleGrantPolicy {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}

///
/// AuthPolicyError
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum AuthPolicyError {
    #[error(
        "delegated token prepare public issuance scope '{scope}' is not self-grantable for role {role}"
    )]
    PublicPrepareScopeNotSelfGrantable { role: CanisterRole, scope: String },

    #[error("root issuer audience is not allowed for issuer {issuer_pid}")]
    RootIssuerAudienceNotAllowed { issuer_pid: Principal },

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

    #[error("delegated token prepare subject must match caller")]
    SubjectCallerMismatch,
}

/// Validate the public delegated-token prepare surface.
///
/// Open issuance is only safe for login/session scopes. Privileged grants need
/// an issuer-authorized path that computes grants instead of trusting request
/// payloads supplied by the caller.
pub fn validate_public_delegated_token_prepare(
    caller: Principal,
    subject: Principal,
    grants: &[DelegatedRoleGrantPolicy],
) -> Result<(), AuthPolicyError> {
    if subject != caller {
        return Err(AuthPolicyError::SubjectCallerMismatch);
    }

    for grant in grants {
        for scope in &grant.scopes {
            if !public_delegated_token_prepare_scope(scope) {
                return Err(AuthPolicyError::PublicPrepareScopeNotSelfGrantable {
                    role: grant.target.clone(),
                    scope: scope.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Return whether a scope is safe to issue from the open prepare endpoint.
#[must_use]
pub fn public_delegated_token_prepare_scope(scope: &str) -> bool {
    scope == cap::SESSION || scope == cap::VERIFY
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrantPolicy {
        DelegatedRoleGrantPolicy {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    #[test]
    fn public_prepare_allows_login_scopes_for_subnet_wide_tokens() {
        validate_public_delegated_token_prepare(
            p(7),
            p(7),
            &[
                grant("user_shard", &[cap::SESSION]),
                grant("project_instance", &[cap::VERIFY]),
            ],
        )
        .expect("login scopes should be public-issuable");
    }

    #[test]
    fn public_prepare_rejects_subject_mismatch() {
        let err = validate_public_delegated_token_prepare(
            p(7),
            p(8),
            &[grant("project_instance", &[cap::SESSION])],
        )
        .expect_err("subject must bind to caller");

        assert_eq!(err, AuthPolicyError::SubjectCallerMismatch);
    }

    #[test]
    fn public_prepare_rejects_privileged_or_custom_scopes() {
        for denied in [cap::READ, cap::WRITE, cap::ADMIN, "toko.admin"] {
            let err = validate_public_delegated_token_prepare(
                p(7),
                p(7),
                &[grant("project_instance", &[denied])],
            )
            .expect_err("privileged scope must not be self-grantable");

            assert_eq!(
                err,
                AuthPolicyError::PublicPrepareScopeNotSelfGrantable {
                    role: CanisterRole::owned("project_instance".to_string()),
                    scope: denied.to_string(),
                }
            );
        }
    }
}
