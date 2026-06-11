use super::canonical::{CanonicalAuthError, role_hash, validate_scope_label};
use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedRoleGrant, DelegationAudience},
    ids::CanisterRole,
};
use thiserror::Error;

pub const MAX_DELEGATED_ROLE_GRANTS: usize = 16;
pub const MAX_SCOPES_PER_ROLE_GRANT: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudienceAcceptanceContext<'a> {
    pub local_canister: Principal,
    pub local_canic_subnet: Option<Principal>,
    pub local_project: Option<&'a str>,
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum AudienceError {
    #[error("delegated auth project audience is empty")]
    EmptyProject,
    #[error("delegated auth project audience contains invalid characters: {project}")]
    InvalidProject { project: String },
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

pub fn validate_audience_shape(audience: &DelegationAudience) -> Result<(), AudienceError> {
    match audience {
        DelegationAudience::Canister(_) | DelegationAudience::CanicSubnet(_) => Ok(()),
        DelegationAudience::Project(project) => validate_project(project),
    }
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
    match (child, parent) {
        (DelegationAudience::Canister(child), DelegationAudience::Canister(parent))
        | (DelegationAudience::CanicSubnet(child), DelegationAudience::CanicSubnet(parent)) => {
            child == parent
        }
        (DelegationAudience::Project(child), DelegationAudience::Project(parent)) => {
            child == parent
        }
        _ => false,
    }
}

pub fn audience_accepted(
    ctx: AudienceAcceptanceContext<'_>,
    audience: &DelegationAudience,
) -> bool {
    match audience {
        DelegationAudience::Canister(canister) => *canister == ctx.local_canister,
        DelegationAudience::CanicSubnet(subnet) => ctx.local_canic_subnet == Some(*subnet),
        DelegationAudience::Project(project) => ctx.local_project == Some(project.as_str()),
    }
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

fn validate_project(project: &str) -> Result<(), AudienceError> {
    if project.is_empty() {
        return Err(AudienceError::EmptyProject);
    }
    if !project.bytes().all(is_canonical_label_byte) {
        return Err(AudienceError::InvalidProject {
            project: project.to_string(),
        });
    }
    Ok(())
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

const fn is_canonical_label_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b':' | b'-' | b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn audience_subset_requires_matching_kind_and_value() {
        assert!(audience_subset(
            &DelegationAudience::Project("demo".to_string()),
            &DelegationAudience::Project("demo".to_string())
        ));
        assert!(audience_subset(
            &DelegationAudience::Canister(p(1)),
            &DelegationAudience::Canister(p(1))
        ));
        assert!(audience_subset(
            &DelegationAudience::CanicSubnet(p(2)),
            &DelegationAudience::CanicSubnet(p(2))
        ));
        assert!(!audience_subset(
            &DelegationAudience::Project("demo".to_string()),
            &DelegationAudience::CanicSubnet(p(2))
        ));
        assert!(!audience_subset(
            &DelegationAudience::Canister(p(1)),
            &DelegationAudience::Project("demo".to_string())
        ));
    }

    #[test]
    fn audience_acceptance_requires_matching_local_context() {
        let ctx = AudienceAcceptanceContext {
            local_canister: p(1),
            local_canic_subnet: Some(p(2)),
            local_project: Some("demo"),
        };

        assert!(audience_accepted(ctx, &DelegationAudience::Canister(p(1))));
        assert!(audience_accepted(
            ctx,
            &DelegationAudience::CanicSubnet(p(2))
        ));
        assert!(audience_accepted(
            ctx,
            &DelegationAudience::Project("demo".to_string())
        ));
        assert!(!audience_accepted(ctx, &DelegationAudience::Canister(p(9))));
        assert!(!audience_accepted(
            ctx,
            &DelegationAudience::CanicSubnet(p(9))
        ));
        let wrong_project = AudienceAcceptanceContext {
            local_project: Some("other"),
            ..ctx
        };
        assert!(!audience_accepted(
            wrong_project,
            &DelegationAudience::Project("demo".to_string())
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
