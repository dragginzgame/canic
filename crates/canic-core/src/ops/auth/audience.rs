use crate::{
    cdk::types::Principal,
    dto::auth::{DelegationAudience, DelegationCert},
    ids::CanisterRole,
};

use super::{DelegationScopeError, TokenAudience, TokenGrant};

// Return true when the local verifier role is allowed by the audience.
pub fn role_allowed(target: &CanisterRole, allowed: &DelegationAudience) -> bool {
    match allowed {
        DelegationAudience::Any => true,
        DelegationAudience::Roles(roles) => roles.iter().any(|role| role == target),
    }
}

// Return true when a token audience stays within the delegation cert audience.
pub fn roles_subset(subset: &DelegationAudience, superset: &DelegationAudience) -> bool {
    match (subset, superset) {
        (DelegationAudience::Any, DelegationAudience::Roles(_)) => false,
        (DelegationAudience::Any | DelegationAudience::Roles(_), DelegationAudience::Any) => true,
        (DelegationAudience::Roles(subset), DelegationAudience::Roles(superset)) => subset
            .iter()
            .all(|item| superset.iter().any(|allowed| allowed == item)),
    }
}

// Return true when a scoped audience has no roles.
pub const fn has_empty_roles(audience: &DelegationAudience) -> bool {
    matches!(audience, DelegationAudience::Roles(roles) if roles.is_empty())
}

// Return the number of role entries in a scoped audience.
pub const fn role_count(audience: &DelegationAudience) -> usize {
    match audience {
        DelegationAudience::Any => 0,
        DelegationAudience::Roles(roles) => roles.len(),
    }
}

// Return true when every string in `subset` is present in `superset`.
pub fn strings_subset(subset: &[String], superset: &[String]) -> bool {
    subset.iter().all(|item| superset.contains(item))
}

// Verify that this canister is explicitly included in the delegated audience.
pub fn verify_self_audience(
    audience: TokenAudience<'_>,
    self_pid: Principal,
    self_role: &CanisterRole,
    self_is_verifier: bool,
) -> Result<(), DelegationScopeError> {
    if !self_is_verifier {
        return Err(DelegationScopeError::SelfVerifierUnavailable { self_pid });
    }

    match audience.aud {
        DelegationAudience::Any => Ok(()),
        DelegationAudience::Roles(roles) if roles.is_empty() => {
            Err(DelegationScopeError::AudienceRoleListEmpty)
        }
        DelegationAudience::Roles(roles) if roles.iter().any(|role| role == self_role) => Ok(()),
        DelegationAudience::Roles(_) => Err(DelegationScopeError::SelfRoleAudienceMissing {
            self_pid,
            role: self_role.clone(),
        }),
    }
}

// Validate token claims against the bounds encoded in the delegation cert.
pub fn validate_claims_against_cert(
    grant: TokenGrant<'_>,
    cert: &DelegationCert,
) -> Result<(), DelegationScopeError> {
    if grant.shard_pid != cert.shard_pid {
        return Err(DelegationScopeError::ShardPidMismatch {
            expected: cert.shard_pid,
            found: grant.shard_pid,
        });
    }

    if has_empty_roles(grant.aud) || has_empty_roles(&cert.aud) {
        return Err(DelegationScopeError::AudienceRoleListEmpty);
    }

    if !roles_subset(grant.aud, &cert.aud) {
        match (grant.aud, &cert.aud) {
            (DelegationAudience::Any, DelegationAudience::Roles(_)) => {
                return Err(DelegationScopeError::AudienceAnyNotAllowed);
            }
            (DelegationAudience::Roles(grant_roles), DelegationAudience::Roles(cert_roles)) => {
                for role in grant_roles {
                    if !cert_roles.iter().any(|allowed| allowed == role) {
                        return Err(DelegationScopeError::AudienceRoleNotAllowed {
                            role: role.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if !strings_subset(grant.scopes, &cert.scopes) {
        for scope in grant.scopes {
            if !cert.scopes.iter().any(|allowed| allowed == scope) {
                return Err(DelegationScopeError::ScopeNotAllowed {
                    scope: scope.clone(),
                });
            }
        }
    }

    Ok(())
}
