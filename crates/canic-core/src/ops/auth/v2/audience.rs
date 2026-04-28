use super::canonical::{CanonicalAuthV2Error, role_hash};
use crate::{cdk::types::Principal, dto::auth::DelegationAudienceV2, ids::CanisterRole};
use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum AudienceV2Error {
    #[error("delegated auth v2 audience must not be empty")]
    EmptyAudience,
    #[error("delegated auth v2 role audience must contain exactly one role")]
    RoleAudienceMustBeSingular,
    #[error("delegated auth v2 role hash mismatch")]
    RoleHashMismatch,
    #[error("delegated auth v2 principal audience contains anonymous principal")]
    AnonymousPrincipal,
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthV2Error),
}

pub fn validate_audience_shape(audience: &DelegationAudienceV2) -> Result<(), AudienceV2Error> {
    match audience {
        DelegationAudienceV2::Roles(roles) => validate_non_empty_roles(roles),
        DelegationAudienceV2::Principals(principals) => validate_non_empty_principals(principals),
        DelegationAudienceV2::RolesOrPrincipals { roles, principals } => {
            if roles.is_empty() && principals.is_empty() {
                return Err(AudienceV2Error::EmptyAudience);
            }
            validate_roles(roles)?;
            validate_principals(principals)
        }
    }
}

pub fn expected_role_hash_for_cert_audience(
    audience: &DelegationAudienceV2,
) -> Result<Option<[u8; 32]>, AudienceV2Error> {
    validate_audience_shape(audience)?;

    match audience {
        DelegationAudienceV2::Principals(_) => Ok(None),
        DelegationAudienceV2::RolesOrPrincipals { roles, .. } if roles.is_empty() => Ok(None),
        DelegationAudienceV2::Roles(roles)
        | DelegationAudienceV2::RolesOrPrincipals { roles, .. } => single_role_hash(roles),
    }
}

pub fn validate_cert_role_hash(
    audience: &DelegationAudienceV2,
    verifier_role_hash: Option<[u8; 32]>,
) -> Result<(), AudienceV2Error> {
    let expected = expected_role_hash_for_cert_audience(audience)?;
    if verifier_role_hash != expected {
        return Err(AudienceV2Error::RoleHashMismatch);
    }
    Ok(())
}

pub const fn audience_uses_role(audience: &DelegationAudienceV2) -> bool {
    match audience {
        DelegationAudienceV2::Principals(_) => false,
        DelegationAudienceV2::Roles(roles)
        | DelegationAudienceV2::RolesOrPrincipals { roles, .. } => !roles.is_empty(),
    }
}

pub fn verifier_is_in_audience(
    local_principal: Principal,
    local_role: Option<&CanisterRole>,
    audience: &DelegationAudienceV2,
) -> bool {
    match audience {
        DelegationAudienceV2::Roles(roles) => {
            local_role.is_some_and(|role| role_branch_contains(roles, role))
        }
        DelegationAudienceV2::Principals(principals) => {
            principal_branch_contains(principals, local_principal)
        }
        DelegationAudienceV2::RolesOrPrincipals { roles, principals } => {
            principal_branch_contains(principals, local_principal)
                || local_role.is_some_and(|role| role_branch_contains(roles, role))
        }
    }
}

pub fn audience_subset(child: &DelegationAudienceV2, parent: &DelegationAudienceV2) -> bool {
    match (child, parent) {
        (DelegationAudienceV2::Roles(child), DelegationAudienceV2::Roles(parent)) => {
            roles_subset(child, parent)
        }
        (DelegationAudienceV2::Principals(child), DelegationAudienceV2::Principals(parent)) => {
            principals_subset(child, parent)
        }
        (
            DelegationAudienceV2::RolesOrPrincipals {
                roles: child_roles,
                principals: child_principals,
            },
            DelegationAudienceV2::RolesOrPrincipals {
                roles: parent_roles,
                principals: parent_principals,
            },
        ) => {
            roles_subset(child_roles, parent_roles)
                && principals_subset(child_principals, parent_principals)
        }
        (
            DelegationAudienceV2::Roles(child),
            DelegationAudienceV2::RolesOrPrincipals { roles: parent, .. },
        ) => roles_subset(child, parent),
        (
            DelegationAudienceV2::Principals(child),
            DelegationAudienceV2::RolesOrPrincipals {
                principals: parent, ..
            },
        ) => principals_subset(child, parent),
        (
            DelegationAudienceV2::RolesOrPrincipals { roles, principals },
            DelegationAudienceV2::Roles(parent),
        ) => principals.is_empty() && roles_subset(roles, parent),
        (
            DelegationAudienceV2::RolesOrPrincipals { roles, principals },
            DelegationAudienceV2::Principals(parent),
        ) => roles.is_empty() && principals_subset(principals, parent),
        (DelegationAudienceV2::Roles(_), DelegationAudienceV2::Principals(_))
        | (DelegationAudienceV2::Principals(_), DelegationAudienceV2::Roles(_)) => false,
    }
}

fn single_role_hash(roles: &[CanisterRole]) -> Result<Option<[u8; 32]>, AudienceV2Error> {
    if roles.len() != 1 {
        return Err(AudienceV2Error::RoleAudienceMustBeSingular);
    }
    Ok(Some(role_hash(&roles[0])?))
}

fn validate_non_empty_roles(roles: &[CanisterRole]) -> Result<(), AudienceV2Error> {
    if roles.is_empty() {
        return Err(AudienceV2Error::EmptyAudience);
    }
    validate_roles(roles)
}

fn validate_non_empty_principals(principals: &[Principal]) -> Result<(), AudienceV2Error> {
    if principals.is_empty() {
        return Err(AudienceV2Error::EmptyAudience);
    }
    validate_principals(principals)
}

fn validate_roles(roles: &[CanisterRole]) -> Result<(), AudienceV2Error> {
    for role in roles {
        role_hash(role)?;
    }
    Ok(())
}

fn validate_principals(principals: &[Principal]) -> Result<(), AudienceV2Error> {
    if principals
        .iter()
        .any(|principal| *principal == Principal::anonymous())
    {
        return Err(AudienceV2Error::AnonymousPrincipal);
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
        let cert = DelegationAudienceV2::RolesOrPrincipals {
            roles: vec![CanisterRole::new("project_instance")],
            principals: vec![],
        };
        let claims = DelegationAudienceV2::Roles(vec![CanisterRole::new("project_instance")]);

        assert!(audience_subset(&claims, &cert));
    }

    #[test]
    fn roles_and_principals_do_not_cross_match() {
        let cert = DelegationAudienceV2::Roles(vec![CanisterRole::new("project_instance")]);
        let claims = DelegationAudienceV2::Principals(vec![p(1)]);

        assert!(!audience_subset(&claims, &cert));
    }

    #[test]
    fn mixed_child_cannot_subset_role_parent_when_principals_are_present() {
        let cert = DelegationAudienceV2::Roles(vec![CanisterRole::new("project_instance")]);
        let claims = DelegationAudienceV2::RolesOrPrincipals {
            roles: vec![CanisterRole::new("project_instance")],
            principals: vec![p(1)],
        };

        assert!(!audience_subset(&claims, &cert));
    }

    #[test]
    fn verifier_membership_accepts_local_role_or_principal() {
        let audience = DelegationAudienceV2::RolesOrPrincipals {
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
        let audience = DelegationAudienceV2::Roles(vec![role.clone()]);
        let expected = role_hash(&role).unwrap();

        validate_cert_role_hash(&audience, Some(expected)).unwrap();
        assert_eq!(
            validate_cert_role_hash(&audience, None),
            Err(AudienceV2Error::RoleHashMismatch)
        );
    }

    #[test]
    fn cert_role_hash_rejects_multi_role_cert_audience() {
        let audience = DelegationAudienceV2::Roles(vec![
            CanisterRole::new("project_instance"),
            CanisterRole::new("project_hub"),
        ]);

        assert_eq!(
            expected_role_hash_for_cert_audience(&audience),
            Err(AudienceV2Error::RoleAudienceMustBeSingular)
        );
    }

    #[test]
    fn principal_only_cert_requires_absent_role_hash() {
        let audience = DelegationAudienceV2::Principals(vec![p(1)]);

        validate_cert_role_hash(&audience, None).unwrap();
        assert_eq!(
            validate_cert_role_hash(&audience, Some([1; 32])),
            Err(AudienceV2Error::RoleHashMismatch)
        );
    }

    #[test]
    fn audience_shape_rejects_empty_and_anonymous_principal_audiences() {
        assert_eq!(
            validate_audience_shape(&DelegationAudienceV2::Principals(vec![])),
            Err(AudienceV2Error::EmptyAudience)
        );
        assert_eq!(
            validate_audience_shape(&DelegationAudienceV2::Principals(vec![
                Principal::anonymous()
            ])),
            Err(AudienceV2Error::AnonymousPrincipal)
        );
    }
}
