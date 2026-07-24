//! Module: ops::auth::delegated::audience
//!
//! Responsibility: validate delegated auth audience and role-grant shapes.
//! Does not own: token verification, storage, or endpoint authorization.
//! Boundary: pure delegated auth helper used by cert and token validation.

use super::canonical::{CanonicalAuthError, role_hash, validate_scope_label};
use crate::{
    dto::auth::{DelegatedRoleGrant, DelegationAudience},
    ids::{CanisterRole, FleetKey},
};
use thiserror::Error;

pub const MAX_DELEGATED_ROLE_GRANTS: usize = 16;
pub const MAX_SCOPES_PER_ROLE_GRANT: usize = 32;

///
/// AudienceAcceptanceContext
///
/// Local verifier context used when deciding whether an audience applies here.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudienceAcceptanceContext {
    pub local_fleet: FleetKey,
}

///
/// AudienceError
///
/// Typed failure surface for delegated audience and role-grant validation.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum AudienceError {
    #[error("delegated auth role grants must not be empty")]
    GrantsEmpty,
    #[error("delegated auth role grants exceed max {max}: {found}")]
    TooManyGrants { found: usize, max: usize },
    #[error("delegated auth role grant for {role} has no scopes")]
    EmptyGrantScopes { role: CanisterRole },
    #[error("delegated auth role grant for {role} exceeds max scopes {max}: {found}")]
    TooManyGrantScopes {
        role: CanisterRole,
        found: usize,
        max: usize,
    },
    #[error("delegated auth role grants must be strictly sorted and unique")]
    NonCanonicalGrants,
    #[error("delegated auth grant scope rejected: {scope}")]
    GrantScopeRejected { scope: String },
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
}

pub fn validate_role_grants(grants: &[DelegatedRoleGrant]) -> Result<(), AudienceError> {
    if grants.is_empty() {
        return Err(AudienceError::GrantsEmpty);
    }
    if grants.len() > MAX_DELEGATED_ROLE_GRANTS {
        return Err(AudienceError::TooManyGrants {
            found: grants.len(),
            max: MAX_DELEGATED_ROLE_GRANTS,
        });
    }

    let mut previous = None;
    for grant in grants {
        role_hash(&grant.target)?;
        let current = grant.target.as_str().as_bytes();
        if previous.is_some_and(|previous| previous >= current) {
            return Err(AudienceError::NonCanonicalGrants);
        }
        previous = Some(current);

        if grant.scopes.is_empty() {
            return Err(AudienceError::EmptyGrantScopes {
                role: grant.target.clone(),
            });
        }
        if grant.scopes.len() > MAX_SCOPES_PER_ROLE_GRANT {
            return Err(AudienceError::TooManyGrantScopes {
                role: grant.target.clone(),
                found: grant.scopes.len(),
                max: MAX_SCOPES_PER_ROLE_GRANT,
            });
        }
        validate_grant_scopes(&grant.scopes)?;
    }

    Ok(())
}

pub fn audience_subset(child: &DelegationAudience, parent: &DelegationAudience) -> bool {
    child == parent
}

pub fn audience_accepted(ctx: AudienceAcceptanceContext, audience: &DelegationAudience) -> bool {
    matches!(audience, DelegationAudience::Fleet(fleet) if *fleet == ctx.local_fleet)
}

pub fn role_grants_subset(child: &[DelegatedRoleGrant], parent: &[DelegatedRoleGrant]) -> bool {
    child.iter().all(|child_grant| {
        parent
            .iter()
            .find(|parent_grant| parent_grant.target == child_grant.target)
            .is_some_and(|parent_grant| scopes_subset(&child_grant.scopes, &parent_grant.scopes))
    })
}

pub fn scopes_for_role(
    grants: &[DelegatedRoleGrant],
    local_role: &CanisterRole,
) -> Option<Vec<String>> {
    grants
        .iter()
        .find(|grant| &grant.target == local_role)
        .map(|grant| grant.scopes.clone())
}

fn validate_grant_scopes(scopes: &[String]) -> Result<(), AudienceError> {
    let mut previous = None;
    for scope in scopes {
        validate_scope_label(scope).map_err(|err| match err {
            CanonicalAuthError::InvalidScope { scope } => {
                AudienceError::GrantScopeRejected { scope }
            }
            other => AudienceError::Canonical(other),
        })?;
        let current = scope.as_bytes();
        if previous.is_some_and(|previous| previous >= current) {
            return Err(AudienceError::Canonical(
                CanonicalAuthError::NonCanonicalScopes,
            ));
        }
        previous = Some(current);
    }
    Ok(())
}

fn scopes_subset(child: &[String], parent: &[String]) -> bool {
    child.iter().all(|scope| parent.contains(scope))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::support::fleet_key;

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    #[test]
    fn audience_subset_requires_matching_fleet() {
        assert!(audience_subset(
            &DelegationAudience::Fleet(fleet_key(1)),
            &DelegationAudience::Fleet(fleet_key(1))
        ));
        assert!(!audience_subset(
            &DelegationAudience::Fleet(fleet_key(1)),
            &DelegationAudience::Fleet(fleet_key(2))
        ));
    }

    #[test]
    fn audience_acceptance_requires_the_protected_local_fleet() {
        let ctx = AudienceAcceptanceContext {
            local_fleet: fleet_key(1),
        };

        assert!(audience_accepted(
            ctx,
            &DelegationAudience::Fleet(fleet_key(1))
        ));
        assert!(!audience_accepted(
            ctx,
            &DelegationAudience::Fleet(fleet_key(2))
        ));
    }

    #[test]
    fn role_grants_require_canonical_order() {
        assert_eq!(
            validate_role_grants(&[
                grant("project_instance", &["upload"]),
                grant("project_hub", &["upload"])
            ]),
            Err(AudienceError::NonCanonicalGrants)
        );
    }

    #[test]
    fn role_grants_subset_checks_scopes_per_role() {
        let parent = [
            grant("project_hub", &["session", "upload"]),
            grant("project_instance", &["upload"]),
        ];
        let child = [
            grant("project_hub", &["upload"]),
            grant("project_instance", &["upload"]),
        ];

        assert!(role_grants_subset(&child, &parent));
        assert!(!role_grants_subset(
            &[grant("project_instance", &["admin"])],
            &parent
        ));
    }
}
