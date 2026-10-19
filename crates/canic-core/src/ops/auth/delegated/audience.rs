//! Module: ops::auth::delegated::audience
//!
//! Responsibility: validate delegated auth audience and role-grant shapes.
//! Does not own: token verification, storage, or endpoint authorization.
//! Boundary: pure delegated auth helper used by cert and token validation.

use super::{canonical::CanonicalAuthError, protocol};
use crate::dto::auth::DelegatedRoleGrant;
#[cfg(test)]
use crate::dto::auth::DelegationAudience;
use canic_contracts::ids::CanisterRole;
#[cfg(test)]
use canic_contracts::ids::FleetKey;
use ic_auth::certificate_issuance::{GrantValidationError, validate_grants};
use thiserror::Error;

pub const MAX_DELEGATED_ROLE_GRANTS: usize = 16;
pub const MAX_SCOPES_PER_ROLE_GRANT: usize = 32;

///
/// AudienceAcceptanceContext
///
/// Local verifier context used when deciding whether an audience applies here.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(test)]
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
    // Bound unchecked DTO projection before allocating canonical Auth values.
    if grants.is_empty() {
        return Err(AudienceError::GrantsEmpty);
    }
    if grants.len() > MAX_DELEGATED_ROLE_GRANTS {
        return Err(AudienceError::TooManyGrants {
            found: grants.len(),
            max: MAX_DELEGATED_ROLE_GRANTS,
        });
    }

    let validate = |grants: &[ic_auth_protocol_types::DelegatedRoleGrant]| {
        validate_grants(grants, MAX_DELEGATED_ROLE_GRANTS, MAX_SCOPES_PER_ROLE_GRANT)
            .map_err(map_grant_validation_error)
    };
    // DTO roles are unchecked strings. Preserve failures in the preceding grants
    // before returning a later role-conversion error; Auth owns all grant rules.
    for (index, grant) in grants.iter().enumerate() {
        if let Err(cause) = protocol::role(&grant.target) {
            if index != 0 {
                validate(&protocol::grants(&grants[..index])?)?;
            }
            return Err(AudienceError::Canonical(cause));
        }
    }
    validate(&protocol::grants(grants)?)
}

fn map_grant_validation_error(error: GrantValidationError) -> AudienceError {
    use ic_auth::canonical::CanonicalAuthError as Canonical;
    match error {
        GrantValidationError::Empty => AudienceError::GrantsEmpty,
        GrantValidationError::TooMany { found, max } => AudienceError::TooManyGrants { found, max },
        GrantValidationError::EmptyScopes { role } => AudienceError::EmptyGrantScopes {
            role: CanisterRole::owned(role.as_str().to_owned()),
        },
        GrantValidationError::TooManyScopes { role, found, max } => {
            AudienceError::TooManyGrantScopes {
                role: CanisterRole::owned(role.as_str().to_owned()),
                found,
                max,
            }
        }
        GrantValidationError::Canonical(Canonical::NonCanonicalRoles) => {
            AudienceError::NonCanonicalGrants
        }
        GrantValidationError::Canonical(Canonical::InvalidScope { scope }) => {
            AudienceError::GrantScopeRejected { scope }
        }
        GrantValidationError::Canonical(cause) => AudienceError::Canonical(cause.into()),
    }
}

#[cfg(test)]
pub fn audience_subset(child: &DelegationAudience, parent: &DelegationAudience) -> bool {
    child == parent
}

#[cfg(test)]
pub fn audience_accepted(ctx: AudienceAcceptanceContext, audience: &DelegationAudience) -> bool {
    matches!(audience, DelegationAudience::Fleet(fleet) if *fleet == ctx.local_fleet)
}

#[cfg(test)]
pub fn role_grants_subset(child: &[DelegatedRoleGrant], parent: &[DelegatedRoleGrant]) -> bool {
    child.iter().all(|child_grant| {
        parent
            .iter()
            .find(|parent_grant| parent_grant.target == child_grant.target)
            .is_some_and(|parent_grant| scopes_subset(&child_grant.scopes, &parent_grant.scopes))
    })
}

#[cfg(test)]
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
    fn grant_roles_use_the_shared_identifier_syntax() {
        validate_role_grants(&[grant("project-instance:v1_2", &["upload"])])
            .expect("shared role syntax");
        for role in ["", "Upper", "bad role", "é"] {
            let expected = if role.is_empty() {
                CanonicalAuthError::EmptyRole
            } else {
                CanonicalAuthError::InvalidRole {
                    role: role.to_string(),
                }
            };
            assert_eq!(
                validate_role_grants(&[grant(role, &["upload"])]),
                Err(AudienceError::Canonical(expected))
            );
        }
    }

    #[test]
    fn invalid_grant_role_precedes_empty_scope_refusal() {
        assert_eq!(
            validate_role_grants(&[grant("Upper", &[])]),
            Err(AudienceError::Canonical(CanonicalAuthError::InvalidRole {
                role: "Upper".to_string(),
            }))
        );
    }

    #[test]
    fn grant_limits_and_scope_failures_keep_typed_causes() {
        let scopes: Vec<_> = (0..32).map(|index| format!("s{index:02}")).collect();
        let maximum: Vec<_> = (0..16)
            .map(|index| DelegatedRoleGrant {
                target: CanisterRole::owned(format!("r{index:02}")),
                scopes: scopes.clone(),
            })
            .collect();
        assert_eq!(validate_role_grants(&maximum), Ok(()));
        assert_eq!(validate_role_grants(&[]), Err(AudienceError::GrantsEmpty));
        assert_eq!(
            validate_role_grants(&vec![grant("Upper", &[]); 17]),
            Err(AudienceError::TooManyGrants { found: 17, max: 16 })
        );
        for (scopes, expected) in [
            (
                vec![],
                AudienceError::EmptyGrantScopes {
                    role: CanisterRole::new("app"),
                },
            ),
            (
                vec!["read"; 33],
                AudienceError::TooManyGrantScopes {
                    role: CanisterRole::new("app"),
                    found: 33,
                    max: 32,
                },
            ),
            (
                vec!["Read"],
                AudienceError::GrantScopeRejected {
                    scope: "Read".into(),
                },
            ),
            (
                vec![""],
                AudienceError::Canonical(CanonicalAuthError::EmptyScope),
            ),
            (
                vec!["read", "read"],
                AudienceError::Canonical(CanonicalAuthError::NonCanonicalScopes),
            ),
            (
                vec!["write", "read"],
                AudienceError::Canonical(CanonicalAuthError::NonCanonicalScopes),
            ),
        ] {
            assert_eq!(
                validate_role_grants(&[grant("app", &scopes)]),
                Err(expected)
            );
        }
    }

    #[test]
    fn earlier_grant_errors_precede_later_unrepresentable_roles() {
        for (first, expected) in [
            (
                grant("app", &[]),
                AudienceError::EmptyGrantScopes {
                    role: CanisterRole::new("app"),
                },
            ),
            (
                grant("app", &["Read"]),
                AudienceError::GrantScopeRejected {
                    scope: "Read".into(),
                },
            ),
            (
                grant("app", &["read", "read"]),
                AudienceError::Canonical(CanonicalAuthError::NonCanonicalScopes),
            ),
        ] {
            assert_eq!(
                validate_role_grants(&[first, grant("Upper", &[])]),
                Err(expected)
            );
        }
        assert_eq!(
            validate_role_grants(&[
                grant("z", &["read"]),
                grant("a", &["read"]),
                grant("Upper", &[])
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
