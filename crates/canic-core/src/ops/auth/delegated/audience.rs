use super::canonical::{CanonicalAuthError, role_hash};
use crate::{cdk::types::Principal, dto::auth::DelegationAudience, ids::CanisterRole};
use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum AudienceError {
    #[error("delegated auth audience must not be empty")]
    EmptyAudience,
    #[error("delegated auth role audience must contain exactly one role")]
    RoleAudienceMustBeSingular,
    #[error("delegated auth role hash mismatch")]
    RoleHashMismatch,
    #[error("delegated auth principal audience contains anonymous principal")]
    AnonymousPrincipal,
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
}

pub fn validate_audience_shape(audience: &DelegationAudience) -> Result<(), AudienceError> {
    match audience {
        DelegationAudience::Roles(roles) => validate_non_empty_roles(roles),
        DelegationAudience::Principals(principals) => validate_non_empty_principals(principals),
        DelegationAudience::RolesOrPrincipals { roles, principals } => {
            if roles.is_empty() && principals.is_empty() {
                return Err(AudienceError::EmptyAudience);
            }
            validate_roles(roles)?;
            validate_principals(principals)
        }
    }
}

pub fn expected_role_hash_for_cert_audience(
    audience: &DelegationAudience,
) -> Result<Option<[u8; 32]>, AudienceError> {
    validate_audience_shape(audience)?;

    match audience {
        DelegationAudience::Principals(_) => Ok(None),
        DelegationAudience::RolesOrPrincipals { roles, .. } if roles.is_empty() => Ok(None),
        DelegationAudience::Roles(roles) | DelegationAudience::RolesOrPrincipals { roles, .. } => {
            single_role_hash(roles)
        }
    }
}

pub fn validate_cert_role_hash(
    audience: &DelegationAudience,
    verifier_role_hash: Option<[u8; 32]>,
) -> Result<(), AudienceError> {
    let expected = expected_role_hash_for_cert_audience(audience)?;
    if verifier_role_hash != expected {
        return Err(AudienceError::RoleHashMismatch);
    }
    Ok(())
}

pub const fn audience_uses_role(audience: &DelegationAudience) -> bool {
    match audience {
        DelegationAudience::Principals(_) => false,
        DelegationAudience::Roles(roles) | DelegationAudience::RolesOrPrincipals { roles, .. } => {
            !roles.is_empty()
        }
    }
}

pub fn verifier_is_in_audience(
    local_principal: Principal,
    local_role: Option<&CanisterRole>,
    audience: &DelegationAudience,
) -> bool {
    match audience {
        DelegationAudience::Roles(roles) => {
            local_role.is_some_and(|role| role_branch_contains(roles, role))
        }
        DelegationAudience::Principals(principals) => {
            principal_branch_contains(principals, local_principal)
        }
        DelegationAudience::RolesOrPrincipals { roles, principals } => {
            principal_branch_contains(principals, local_principal)
                || local_role.is_some_and(|role| role_branch_contains(roles, role))
        }
    }
}

pub fn audience_subset(child: &DelegationAudience, parent: &DelegationAudience) -> bool {
    match (child, parent) {
        (DelegationAudience::Roles(child), DelegationAudience::Roles(parent)) => {
            roles_subset(child, parent)
        }
        (DelegationAudience::Principals(child), DelegationAudience::Principals(parent)) => {
            principals_subset(child, parent)
        }
        (
            DelegationAudience::RolesOrPrincipals {
                roles: child_roles,
                principals: child_principals,
            },
            DelegationAudience::RolesOrPrincipals {
                roles: parent_roles,
                principals: parent_principals,
            },
        ) => {
            roles_subset(child_roles, parent_roles)
                && principals_subset(child_principals, parent_principals)
        }
        (
            DelegationAudience::Roles(child),
            DelegationAudience::RolesOrPrincipals { roles: parent, .. },
        ) => roles_subset(child, parent),
        (
            DelegationAudience::Principals(child),
            DelegationAudience::RolesOrPrincipals {
                principals: parent, ..
            },
        ) => principals_subset(child, parent),
        (
            DelegationAudience::RolesOrPrincipals { roles, principals },
            DelegationAudience::Roles(parent),
        ) => principals.is_empty() && roles_subset(roles, parent),
        (
            DelegationAudience::RolesOrPrincipals { roles, principals },
            DelegationAudience::Principals(parent),
        ) => roles.is_empty() && principals_subset(principals, parent),
        (DelegationAudience::Roles(_), DelegationAudience::Principals(_))
        | (DelegationAudience::Principals(_), DelegationAudience::Roles(_)) => false,
    }
}

fn single_role_hash(roles: &[CanisterRole]) -> Result<Option<[u8; 32]>, AudienceError> {
    if roles.len() != 1 {
        return Err(AudienceError::RoleAudienceMustBeSingular);
    }
    Ok(Some(role_hash(&roles[0])?))
}

fn validate_non_empty_roles(roles: &[CanisterRole]) -> Result<(), AudienceError> {
    if roles.is_empty() {
        return Err(AudienceError::EmptyAudience);
    }
    validate_roles(roles)
}

fn validate_non_empty_principals(principals: &[Principal]) -> Result<(), AudienceError> {
    if principals.is_empty() {
        return Err(AudienceError::EmptyAudience);
    }
    validate_principals(principals)
}

fn validate_roles(roles: &[CanisterRole]) -> Result<(), AudienceError> {
    for role in roles {
        role_hash(role)?;
    }
    Ok(())
}

fn validate_principals(principals: &[Principal]) -> Result<(), AudienceError> {
    if principals
        .iter()
        .any(|principal| *principal == Principal::anonymous())
    {
        return Err(AudienceError::AnonymousPrincipal);
    }
    Ok(())
}

fn roles_subset(child: &[CanisterRole], parent: &[CanisterRole]) -> bool {
    child
        .iter()
        .all(|role| parent.iter().any(|allowed| allowed == role))
}

fn principals_subset(child: &[Principal], parent: &[Principal]) -> bool {
    child
        .iter()
        .all(|principal| parent.iter().any(|allowed| allowed == principal))
}

fn role_branch_contains(roles: &[CanisterRole], role: &CanisterRole) -> bool {
    roles.iter().any(|allowed| allowed == role)
}

fn principal_branch_contains(principals: &[Principal], principal: Principal) -> bool {
    principals.contains(&principal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn roles_or_principals_allows_role_claim_subset() {
        let cert = DelegationAudience::RolesOrPrincipals {
            roles: vec![CanisterRole::new("project_instance")],
            principals: vec![],
        };
        let claims = DelegationAudience::Roles(vec![CanisterRole::new("project_instance")]);

        assert!(audience_subset(&claims, &cert));
    }

    #[test]
    fn roles_and_principals_do_not_cross_match() {
        let cert = DelegationAudience::Roles(vec![CanisterRole::new("project_instance")]);
        let claims = DelegationAudience::Principals(vec![p(1)]);

        assert!(!audience_subset(&claims, &cert));
    }

    #[test]
    fn mixed_child_cannot_subset_role_parent_when_principals_are_present() {
        let cert = DelegationAudience::Roles(vec![CanisterRole::new("project_instance")]);
        let claims = DelegationAudience::RolesOrPrincipals {
            roles: vec![CanisterRole::new("project_instance")],
            principals: vec![p(1)],
        };

        assert!(!audience_subset(&claims, &cert));
    }

    #[test]
    fn verifier_membership_accepts_local_role_or_principal() {
        let audience = DelegationAudience::RolesOrPrincipals {
            roles: vec![CanisterRole::new("project_instance")],
            principals: vec![p(1)],
        };

        assert!(verifier_is_in_audience(
            p(9),
            Some(&CanisterRole::new("project_instance")),
            &audience
        ));
        assert!(verifier_is_in_audience(p(1), None, &audience));
        assert!(!verifier_is_in_audience(
            p(9),
            Some(&CanisterRole::new("project_hub")),
            &audience
        ));
    }

    #[test]
    fn cert_role_hash_requires_exact_single_role_hash() {
        let role = CanisterRole::new("project_instance");
        let audience = DelegationAudience::Roles(vec![role.clone()]);
        let expected = role_hash(&role).unwrap();

        validate_cert_role_hash(&audience, Some(expected)).unwrap();
        assert_eq!(
            validate_cert_role_hash(&audience, None),
            Err(AudienceError::RoleHashMismatch)
        );
    }

    #[test]
    fn cert_role_hash_rejects_multi_role_cert_audience() {
        let audience = DelegationAudience::Roles(vec![
            CanisterRole::new("project_instance"),
            CanisterRole::new("project_hub"),
        ]);

        assert_eq!(
            expected_role_hash_for_cert_audience(&audience),
            Err(AudienceError::RoleAudienceMustBeSingular)
        );
    }

    #[test]
    fn principal_only_cert_requires_absent_role_hash() {
        let audience = DelegationAudience::Principals(vec![p(1)]);

        validate_cert_role_hash(&audience, None).unwrap();
        assert_eq!(
            validate_cert_role_hash(&audience, Some([1; 32])),
            Err(AudienceError::RoleHashMismatch)
        );
    }

    #[test]
    fn audience_shape_rejects_empty_and_anonymous_principal_audiences() {
        assert_eq!(
            validate_audience_shape(&DelegationAudience::Principals(vec![])),
            Err(AudienceError::EmptyAudience)
        );
        assert_eq!(
            validate_audience_shape(&DelegationAudience::Principals(
                vec![Principal::anonymous()]
            )),
            Err(AudienceError::AnonymousPrincipal)
        );
    }
}
