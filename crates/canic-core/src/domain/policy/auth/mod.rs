//! Module: domain::policy::auth
//!
//! Responsibility: pure auth issuance policy decisions.
//! Does not own: proof verification, storage access, replay, or signing.
//! Boundary: called by workflow before auth ops prepare delegated-token proofs.

use crate::{
    cdk::types::Principal,
    dto::auth::DelegatedRoleGrant,
    ids::{CanisterRole, cap},
};
use thiserror::Error as ThisError;

///
/// AuthPolicyError
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum AuthPolicyError {
    #[error(
        "delegated token prepare public issuance scope '{scope}' is not self-grantable for role {role}"
    )]
    PublicPrepareScopeNotSelfGrantable { role: CanisterRole, scope: String },

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
    grants: &[DelegatedRoleGrant],
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

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
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
