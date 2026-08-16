use crate::{InternalError, dto::error::Error};

const fn internal_error_to_public(err: &InternalError) -> Error {
    err.public_error()
}

impl From<&InternalError> for Error {
    fn from(err: &InternalError) -> Self {
        internal_error_to_public(err)
    }
}

impl From<InternalError> for Error {
    fn from(err: InternalError) -> Self {
        internal_error_to_public(&err)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::AccessError,
        domain::policy::pure::{
            component_allocation::ComponentAllocationPolicyError,
            component_child_allocation::ComponentChildAllocationPolicyError,
        },
        ids::CanisterRole,
    };

    #[test]
    fn internal_error_mapping_uses_registered_semantic_causes() {
        let access: Error = InternalError::from(AccessError::ControllerRequired).into();
        assert_eq!(
            access.code(),
            crate::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
        );

        let domain_config: Error = InternalError::projected(
            crate::diagnostics::codes::CONFIGURATION_INVALID,
            crate::diagnostics::codes::REQUEST_INVALID,
        )
        .into();
        assert_eq!(
            domain_config.code(),
            crate::diagnostics::codes::REQUEST_INVALID.raw_code()
        );

        let domain_other: Error = InternalError::conflict().into();
        assert_eq!(
            domain_other.code(),
            crate::diagnostics::codes::STATE_CONFLICT.raw_code()
        );

        let invariant: Error = InternalError::invariant().into();
        assert_eq!(
            invariant.code(),
            crate::diagnostics::codes::STATE_INVALID.raw_code()
        );

        let infra: Error = InternalError::platform_failure().into();
        assert_eq!(
            infra.code(),
            crate::diagnostics::codes::STATE_FAILED.raw_code()
        );

        let ops: Error = InternalError::state_failure().into();
        assert_eq!(
            ops.code(),
            crate::diagnostics::codes::STATE_FAILED.raw_code()
        );

        let workflow: Error = InternalError::lifecycle_failure().into();
        assert_eq!(
            workflow.code(),
            crate::diagnostics::codes::STATE_FAILED.raw_code()
        );

        let invalid_allocation: Error =
            InternalError::from(ComponentAllocationPolicyError::EmptyOperationId).into();
        assert_eq!(
            invalid_allocation.code(),
            crate::diagnostics::codes::REQUEST_INCOMPLETE.raw_code()
        );

        let exhausted_allocation: Error =
            InternalError::from(ComponentAllocationPolicyError::ComponentCapacityExhausted).into();
        assert_eq!(
            exhausted_allocation.code(),
            crate::diagnostics::codes::CAPACITY_LIMIT.raw_code()
        );

        let invalid_authority: Error =
            InternalError::from(ComponentAllocationPolicyError::RootTopologyDigestMismatch).into();
        assert_eq!(
            invalid_authority.code(),
            crate::diagnostics::codes::DIGEST_CONFLICT.raw_code()
        );

        let forbidden_child: Error =
            InternalError::from(ComponentChildAllocationPolicyError::SpawnGrantMissing {
                parent_role: CanisterRole::new("project_hub"),
                child_role: CanisterRole::new("project_ledger"),
            })
            .into();
        assert_eq!(
            forbidden_child.code(),
            crate::diagnostics::codes::CONFIGURATION_UNAVAILABLE.raw_code()
        );

        let stale_child: Error = InternalError::from(
            ComponentChildAllocationPolicyError::ComponentRegistryAuthorityMismatch,
        )
        .into();
        assert_eq!(
            stale_child.code(),
            crate::diagnostics::codes::AUTHORITY_CONFLICT.raw_code()
        );

        let exhausted_child: Error = InternalError::from(
            ComponentChildAllocationPolicyError::ComponentDescendantCapacityExhausted,
        )
        .into();
        assert_eq!(
            exhausted_child.code(),
            crate::diagnostics::codes::CAPACITY_LIMIT.raw_code()
        );

        let token_expired: Error = AccessError::DelegatedAuthTokenExpired.into();
        assert_eq!(
            token_expired.code(),
            crate::diagnostics::codes::AUTH_TOKEN_EXPIRED.raw_code()
        );

        let cert_expired: Error = AccessError::DelegatedAuthCertExpired.into();
        assert_eq!(
            cert_expired.code(),
            crate::diagnostics::codes::AUTH_CERT_EXPIRED.raw_code()
        );
    }

    #[test]
    fn public_error_is_preserved_without_remap() {
        let public = Error::from_registered(crate::diagnostics::codes::COLLECTION_UNAVAILABLE);
        let remapped: Error =
            InternalError::public(crate::diagnostics::codes::COLLECTION_UNAVAILABLE).into();
        assert_eq!(remapped, public);
    }
}
