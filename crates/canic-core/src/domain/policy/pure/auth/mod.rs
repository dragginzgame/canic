//! Module: domain::policy::pure::auth
//!
//! Responsibility: pure auth issuance policy decisions.
//! Does not own: proof verification, storage access, replay, or signing.
//! Boundary: called by workflow before auth ops prepare delegated-token proofs.

use crate::{
    domain::value::Principal,
    ids::{CanisterRole, cap},
    model::auth::application_authorization::ApplicationScopeRef,
};
use thiserror::Error as ThisError;

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "B2 pure decisions are consumed by the sequenced B3-B5 runtime batches"
    )
)]
pub mod application_authorization;
mod root_provisioning;

pub use root_provisioning::{
    RootDelegationProofPreparePolicyInput, validate_root_delegation_proof_prepare_policy,
    validate_root_issuer_policy_fleet_binding, validate_root_issuer_policy_upsert,
    validate_root_issuer_renewal_template_fleet_binding,
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

/// Canonical application scopes explicitly declared for one target role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredApplicationRoleScopes {
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

/// Validate the public delegated-token prepare surface.
///
/// Open issuance is only safe for login/session scopes. Privileged grants need
/// an issuer-authorized path that computes grants instead of trusting request
/// payloads supplied by the caller.
pub fn validate_public_delegated_token_prepare(
    grants: &[DelegatedRoleGrantPolicy],
    declared_application_scopes: &[DeclaredApplicationRoleScopes],
) -> Result<(), AuthPolicyError> {
    for grant in grants {
        for scope in &grant.scopes {
            if !public_delegated_token_prepare_scope(
                &grant.target,
                scope,
                declared_application_scopes,
            ) {
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
fn public_delegated_token_prepare_scope(
    role: &CanisterRole,
    scope: &str,
    declared_application_scopes: &[DeclaredApplicationRoleScopes],
) -> bool {
    if scope == cap::SESSION || scope == cap::VERIFY {
        return true;
    }
    if ApplicationScopeRef::parse(scope).is_err() {
        return false;
    }
    declared_application_scopes.iter().any(|declared| {
        declared.target == *role
            && declared
                .scopes
                .binary_search_by(|candidate| candidate.as_str().cmp(scope))
                .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrantPolicy {
        DelegatedRoleGrantPolicy {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn declared(role: &str, scopes: &[&str]) -> DeclaredApplicationRoleScopes {
        DeclaredApplicationRoleScopes {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    #[test]
    fn public_prepare_allows_login_scopes_for_fleet_wide_tokens() {
        validate_public_delegated_token_prepare(
            &[
                grant("user_shard", &[cap::SESSION]),
                grant("project_instance", &[cap::VERIFY]),
            ],
            &[],
        )
        .expect("login scopes should be public-issuable");
    }

    #[test]
    fn public_prepare_allows_only_role_declared_canonical_application_scopes() {
        let declared = [
            declared("project_instance", &["demo:read", "demo:write"]),
            declared("user_shard", &["demo:read"]),
        ];

        validate_public_delegated_token_prepare(
            &[grant("project_instance", &["demo:read", "demo:write"])],
            &declared,
        )
        .expect("declared application scopes should be issuable");

        for (role, scope) in [
            ("project_instance", "demo:admin"),
            ("user_shard", "demo:write"),
            ("project_instance", "Demo:read"),
        ] {
            let err = validate_public_delegated_token_prepare(&[grant(role, &[scope])], &declared)
                .expect_err("undeclared or noncanonical application scope must fail");
            assert_eq!(
                err,
                AuthPolicyError::PublicPrepareScopeNotSelfGrantable {
                    role: CanisterRole::owned(role.to_string()),
                    scope: scope.to_string(),
                }
            );
        }
    }

    #[test]
    fn public_prepare_rejects_privileged_or_custom_scopes() {
        for denied in [cap::READ, cap::WRITE, cap::ADMIN, "toko.admin"] {
            let err = validate_public_delegated_token_prepare(
                &[grant("project_instance", &[denied])],
                &[],
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
