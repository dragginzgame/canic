use super::canonical::{CanonicalAuthError, role_hash};
use crate::{cdk::types::Principal, dto::auth::DelegationAudience, ids::CanisterRole};
use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum AudienceError {
    #[error("delegated auth role hash mismatch")]
    RoleHashMismatch,
    #[error("delegated auth principal audience is anonymous principal")]
    AnonymousPrincipal,
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
}

pub fn validate_audience_shape(audience: &DelegationAudience) -> Result<(), AudienceError> {
    match audience {
        DelegationAudience::Role(role) => {
            role_hash(role)?;
            Ok(())
        }
        DelegationAudience::Principal(principal) => validate_principal(*principal),
    }
}

pub fn expected_role_hash_for_cert_audience(
    audience: &DelegationAudience,
) -> Result<Option<[u8; 32]>, AudienceError> {
    validate_audience_shape(audience)?;

    match audience {
        DelegationAudience::Principal(_) => Ok(None),
        DelegationAudience::Role(role) => Ok(Some(role_hash(role)?)),
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
    matches!(audience, DelegationAudience::Role(_))
}

pub fn verifier_is_in_audience(
    local_principal: Principal,
    local_role: Option<&CanisterRole>,
    audience: &DelegationAudience,
) -> bool {
    match audience {
        DelegationAudience::Role(role) => local_role.is_some_and(|local| local == role),
        DelegationAudience::Principal(principal) => local_principal == *principal,
    }
}

pub fn audience_subset(child: &DelegationAudience, parent: &DelegationAudience) -> bool {
    match (child, parent) {
        (DelegationAudience::Role(child), DelegationAudience::Role(parent)) => child == parent,
        (DelegationAudience::Principal(child), DelegationAudience::Principal(parent)) => {
            child == parent
        }
        (DelegationAudience::Role(_), DelegationAudience::Principal(_))
        | (DelegationAudience::Principal(_), DelegationAudience::Role(_)) => false,
    }
}

fn validate_principal(principal: Principal) -> Result<(), AudienceError> {
    if principal == Principal::anonymous() {
        return Err(AudienceError::AnonymousPrincipal);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn matching_roles_are_audience_subset() {
        let cert = DelegationAudience::Role(CanisterRole::new("project_instance"));
        let claims = DelegationAudience::Role(CanisterRole::new("project_instance"));

        assert!(audience_subset(&claims, &cert));
    }

    #[test]
    fn role_and_principal_do_not_cross_match() {
        let cert = DelegationAudience::Role(CanisterRole::new("project_instance"));
        let claims = DelegationAudience::Principal(p(1));

        assert!(!audience_subset(&claims, &cert));
    }

    #[test]
    fn different_roles_are_not_audience_subset() {
        let cert = DelegationAudience::Role(CanisterRole::new("project_instance"));
        let claims = DelegationAudience::Role(CanisterRole::new("project_hub"));

        assert!(!audience_subset(&claims, &cert));
    }

    #[test]
    fn verifier_membership_accepts_matching_role() {
        let audience = DelegationAudience::Role(CanisterRole::new("project_instance"));

        assert!(verifier_is_in_audience(
            p(9),
            Some(&CanisterRole::new("project_instance")),
            &audience
        ));
        assert!(!verifier_is_in_audience(
            p(9),
            Some(&CanisterRole::new("project_hub")),
            &audience
        ));
    }

    #[test]
    fn cert_role_hash_requires_exact_single_role_hash() {
        let role = CanisterRole::new("project_instance");
        let audience = DelegationAudience::Role(role.clone());
        let expected = role_hash(&role).unwrap();

        validate_cert_role_hash(&audience, Some(expected)).unwrap();
        assert_eq!(
            validate_cert_role_hash(&audience, None),
            Err(AudienceError::RoleHashMismatch)
        );
    }

    #[test]
    fn principal_only_cert_requires_absent_role_hash() {
        let audience = DelegationAudience::Principal(p(1));

        validate_cert_role_hash(&audience, None).unwrap();
        assert_eq!(
            validate_cert_role_hash(&audience, Some([1; 32])),
            Err(AudienceError::RoleHashMismatch)
        );
    }

    #[test]
    fn audience_shape_rejects_anonymous_principal_audience() {
        assert_eq!(
            validate_audience_shape(&DelegationAudience::Principal(Principal::anonymous())),
            Err(AudienceError::AnonymousPrincipal)
        );
    }
}
