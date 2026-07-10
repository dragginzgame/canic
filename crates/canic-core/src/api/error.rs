use crate::{InternalError, InternalErrorClass, InternalErrorOrigin, dto::error::Error};

fn internal_error_to_public(err: &InternalError) -> Error {
    if let Some(public) = err.public_error() {
        return public.clone();
    }

    let message = err.to_string();

    match err.class() {
        InternalErrorClass::Access => Error::unauthorized(message),

        InternalErrorClass::Domain => match err.origin() {
            InternalErrorOrigin::Config => Error::invalid(message),
            _ => Error::conflict(message),
        },

        InternalErrorClass::Invariant => Error::invariant(message),

        InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
            Error::internal(message)
        }
    }
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
        cdk::types::Principal,
        domain::policy::pure::topology::{TopologyPolicyError, registry::RegistryPolicyError},
        dto::error::ErrorCode,
        ids::CanisterRole,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn internal_error_mapping_matches_class_contract() {
        let access: Error = InternalError::from(AccessError::Denied("denied".to_string())).into();
        assert_eq!(access.code, ErrorCode::Unauthorized);

        let domain_config: Error =
            InternalError::domain(InternalErrorOrigin::Config, "bad config").into();
        assert_eq!(domain_config.code, ErrorCode::InvalidInput);

        let domain_other: Error =
            InternalError::domain(InternalErrorOrigin::Domain, "conflict").into();
        assert_eq!(domain_other.code, ErrorCode::Conflict);

        let invariant: Error =
            InternalError::invariant(InternalErrorOrigin::Ops, "broken invariant").into();
        assert_eq!(invariant.code, ErrorCode::InvariantViolation);

        let infra: Error = InternalError::infra(InternalErrorOrigin::Infra, "infra fail").into();
        assert_eq!(infra.code, ErrorCode::Internal);

        let ops: Error = InternalError::ops(InternalErrorOrigin::Ops, "ops fail").into();
        assert_eq!(ops.code, ErrorCode::Internal);

        let workflow: Error =
            InternalError::workflow(InternalErrorOrigin::Workflow, "workflow fail").into();
        assert_eq!(workflow.code, ErrorCode::Internal);

        let token_expired: Error = AccessError::DelegatedAuthTokenExpired.into();
        assert_eq!(token_expired.code, ErrorCode::AuthTokenExpired);

        let cert_expired: Error = AccessError::DelegatedAuthCertExpired.into();
        assert_eq!(cert_expired.code, ErrorCode::AuthProofExpired);
    }

    #[test]
    fn public_error_is_preserved_without_remap() {
        let public = Error::not_found("missing");
        let remapped: Error = InternalError::public(public.clone()).into();
        assert_eq!(remapped, public);
    }

    #[test]
    fn registry_policy_errors_map_to_stable_public_policy_codes() {
        let err = RegistryPolicyError::RoleAlreadyRegistered {
            role: CanisterRole::new("app"),
            pid: p(7),
        };
        let internal: InternalError = TopologyPolicyError::from(err).into();
        let public: Error = internal.into();
        assert_eq!(public.code, ErrorCode::PolicyRoleAlreadyRegistered);
    }

    #[test]
    fn registry_policy_service_parent_errors_use_service_owned_public_codes() {
        let cases = [
            (
                RegistryPolicyError::ReplicaRequiresServiceWithScaling {
                    role: CanisterRole::new("replica"),
                    parent_role: CanisterRole::new("plain_parent"),
                },
                ErrorCode::PolicyReplicaRequiresServiceWithScaling,
            ),
            (
                RegistryPolicyError::ShardRequiresServiceWithSharding {
                    role: CanisterRole::new("shard"),
                    parent_role: CanisterRole::new("plain_parent"),
                },
                ErrorCode::PolicyShardRequiresServiceWithSharding,
            ),
            (
                RegistryPolicyError::InstanceRequiresServiceWithDirectory {
                    role: CanisterRole::new("instance"),
                    parent_role: CanisterRole::new("plain_parent"),
                },
                ErrorCode::PolicyInstanceRequiresServiceWithDirectory,
            ),
        ];

        for (err, expected) in cases {
            let internal: InternalError = TopologyPolicyError::from(err).into();
            let public: Error = internal.into();

            assert_eq!(public.code, expected);
        }
    }
}
