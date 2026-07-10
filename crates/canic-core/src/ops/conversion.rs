//! Module: ops::conversion
//!
//! Responsibility: preserve typed domain failure classifications at the internal/public boundary.
//! Does not own: policy decisions, public rendering, or endpoint orchestration.
//! Boundary: converts owner-defined domain errors into structured internal errors.

use crate::{
    InternalError, InternalErrorOrigin,
    domain::policy::pure::topology::{TopologyPolicyError, registry::RegistryPolicyError},
    dto::error::{Error, ErrorCode},
};

impl From<TopologyPolicyError> for InternalError {
    fn from(err: TopologyPolicyError) -> Self {
        let message = err.to_string();
        let public_code = match &err {
            TopologyPolicyError::RegistryPolicy(err) => registry_policy_error_code(err),
            TopologyPolicyError::DuplicateIndexRole(_)
            | TopologyPolicyError::ImmediateParentMismatch { .. }
            | TopologyPolicyError::IndexRoleMismatch { .. }
            | TopologyPolicyError::ModuleHashMismatch(_)
            | TopologyPolicyError::ParentNotFound(_)
            | TopologyPolicyError::RegistryEntryMissing(_) => None,
        };

        match public_code {
            Some(code) => Self::public(Error::policy(code, message)),
            None => Self::domain(InternalErrorOrigin::Domain, message),
        }
    }
}

const fn registry_policy_error_code(err: &RegistryPolicyError) -> Option<ErrorCode> {
    match err {
        RegistryPolicyError::RoleAlreadyRegistered { .. } => {
            Some(ErrorCode::PolicyRoleAlreadyRegistered)
        }
        RegistryPolicyError::SingletonAlreadyRegisteredUnderParent { .. } => {
            Some(ErrorCode::PolicySingletonAlreadyRegisteredUnderParent)
        }
        RegistryPolicyError::ReplicaRequiresServiceWithScaling { .. } => {
            Some(ErrorCode::PolicyReplicaRequiresServiceWithScaling)
        }
        RegistryPolicyError::ShardRequiresServiceWithSharding { .. } => {
            Some(ErrorCode::PolicyShardRequiresServiceWithSharding)
        }
        RegistryPolicyError::InstanceRequiresServiceWithDirectory { .. } => {
            Some(ErrorCode::PolicyInstanceRequiresServiceWithDirectory)
        }
        RegistryPolicyError::ServiceRequiresRootParent { .. } => None,
    }
}
