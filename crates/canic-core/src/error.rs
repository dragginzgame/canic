use crate::access::AccessError;
use crate::domain::policy::pure::{
    component_allocation::ComponentAllocationPolicyError,
    component_child_allocation::ComponentChildAllocationPolicyError,
    topology::{TopologyPolicyError, registry::RegistryPolicyError},
};
use crate::dto::error::{Error as PublicError, ErrorCode as PublicErrorCode};
use std::fmt;
use thiserror::Error as ThisError;

///
/// InternalError
///
/// Internal, structured error type.
///
/// This error:
/// - is NOT Candid-exposed
/// - is NOT stable across versions
/// - may evolve freely
///
/// All canister endpoints must convert this into a public error envelope
/// defined in dto/.
///

#[derive(Debug, ThisError)]
#[error("{message}")]
pub struct InternalError {
    class: InternalErrorClass,
    origin: InternalErrorOrigin,
    message: String,
    public_error: Option<PublicError>,
}

impl InternalError {
    pub fn new(
        class: InternalErrorClass,
        origin: InternalErrorOrigin,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            origin,
            message: message.into(),
            public_error: None,
        }
    }

    #[must_use]
    pub fn public(err: PublicError) -> Self {
        Self {
            class: InternalErrorClass::Domain,
            origin: InternalErrorOrigin::Domain,
            message: err.message.clone(),
            public_error: Some(err),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::public(PublicError::forbidden(message))
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::public(PublicError::invalid(message))
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::public(PublicError::conflict(message))
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::public(PublicError::unavailable(message))
    }

    pub fn resource_exhausted(message: impl Into<String>) -> Self {
        Self::public(PublicError::exhausted(message))
    }

    pub fn auth_material_stale(message: impl Into<String>) -> Self {
        Self::public(PublicError::new(
            PublicErrorCode::AuthMaterialStale,
            message.into(),
        ))
    }

    pub fn auth_proof_expired(message: impl Into<String>) -> Self {
        Self::public(PublicError::new(
            PublicErrorCode::AuthProofExpired,
            message.into(),
        ))
    }

    pub fn auth_token_expired(message: impl Into<String>) -> Self {
        Self::public(PublicError::auth_token_expired(message))
    }

    pub fn auth_proof_pending(message: impl Into<String>) -> Self {
        Self::public(PublicError::auth_proof_pending(message))
    }

    #[must_use]
    pub fn operation_id_required() -> Self {
        Self::public(PublicError::operation_id_required())
    }

    #[must_use]
    pub fn root_data_certificate_unavailable() -> Self {
        Self::public(PublicError::root_data_certificate_unavailable())
    }

    pub fn domain(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Domain, origin, message)
    }

    pub fn invariant(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Invariant, origin, message)
    }

    pub fn infra(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Infra, origin, message)
    }

    pub fn ops(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Ops, origin, message)
    }

    pub fn workflow(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Workflow, origin, message)
    }

    /// Append internal diagnostic context without changing the error's typed
    /// classification or public projection.
    #[must_use]
    pub(crate) fn with_diagnostic_context(mut self, context: impl Into<String>) -> Self {
        self.message = format!("{}; {}", self.message, context.into());
        self
    }

    #[must_use]
    pub const fn class(&self) -> InternalErrorClass {
        self.class
    }

    #[must_use]
    pub const fn origin(&self) -> InternalErrorOrigin {
        self.origin
    }

    #[must_use]
    pub const fn log_fields(&self) -> (InternalErrorClass, InternalErrorOrigin) {
        (self.class, self.origin)
    }

    #[must_use]
    pub const fn public_error(&self) -> Option<&PublicError> {
        self.public_error.as_ref()
    }

    #[must_use]
    pub fn is_public_resource_exhausted(&self) -> bool {
        self.public_error
            .as_ref()
            .is_some_and(|err| err.code == PublicErrorCode::ResourceExhausted)
    }
}

impl From<AccessError> for InternalError {
    fn from(err: AccessError) -> Self {
        let kind = err.kind();
        let message = err.to_string();
        match kind {
            crate::access::AccessErrorKind::DelegatedAuthCertExpired => {
                Self::auth_proof_expired(message)
            }
            crate::access::AccessErrorKind::DelegatedAuthTokenExpired => {
                Self::auth_token_expired(message)
            }
            crate::access::AccessErrorKind::Denied => Self::new(
                InternalErrorClass::Access,
                InternalErrorOrigin::Access,
                message,
            ),
        }
    }
}

impl From<TopologyPolicyError> for InternalError {
    fn from(err: TopologyPolicyError) -> Self {
        let message = err.to_string();
        let public_code = match &err {
            TopologyPolicyError::RegistryPolicy(err) => registry_policy_error_code(err),
            TopologyPolicyError::DuplicateDirectoryRole(_)
            | TopologyPolicyError::ImmediateParentMismatch { .. }
            | TopologyPolicyError::DirectoryRoleMismatch { .. }
            | TopologyPolicyError::ParentNotFound(_)
            | TopologyPolicyError::RegistryEntryMissing(_) => None,
        };

        match public_code {
            Some(code) => Self::public(PublicError::policy(code, message)),
            None => Self::domain(InternalErrorOrigin::Domain, message),
        }
    }
}

impl From<ComponentAllocationPolicyError> for InternalError {
    fn from(err: ComponentAllocationPolicyError) -> Self {
        let message = err.to_string();
        match err {
            ComponentAllocationPolicyError::EmptyOperationId
            | ComponentAllocationPolicyError::ComponentSpecNotAdmitted(_)
            | ComponentAllocationPolicyError::ComponentSpecUnknown(_) => {
                Self::invalid_input(message)
            }
            ComponentAllocationPolicyError::AllocationSequenceExhausted
            | ComponentAllocationPolicyError::ComponentCountOverflow
            | ComponentAllocationPolicyError::ComponentCapacityExhausted
            | ComponentAllocationPolicyError::ComponentSpecCountOverflow(_)
            | ComponentAllocationPolicyError::ComponentSpecCapacityExhausted(_)
            | ComponentAllocationPolicyError::ManagedCanisterCountOverflow
            | ComponentAllocationPolicyError::ManagedCanisterCapacityExhausted
            | ComponentAllocationPolicyError::PeerProvisioningCountOverflow
            | ComponentAllocationPolicyError::PeerProvisioningCapacityExhausted { .. } => {
                Self::resource_exhausted(message)
            }
            ComponentAllocationPolicyError::InvalidRootTopologyProjection
            | ComponentAllocationPolicyError::RootTopologyDigestMismatch
            | ComponentAllocationPolicyError::ComponentSpecHashMismatch(_)
            | ComponentAllocationPolicyError::InvalidPeerRequesterBinding => {
                Self::invariant(InternalErrorOrigin::Domain, message)
            }
            ComponentAllocationPolicyError::PeerRootRuntimeInactive => Self::unavailable(message),
            ComponentAllocationPolicyError::PeerRequesterRegistryMemberInactive
            | ComponentAllocationPolicyError::PeerProvisioningGrantMissing { .. } => {
                Self::forbidden(message)
            }
        }
    }
}

impl From<ComponentChildAllocationPolicyError> for InternalError {
    fn from(err: ComponentChildAllocationPolicyError) -> Self {
        let message = err.to_string();
        match err {
            ComponentChildAllocationPolicyError::EmptyOperationId
            | ComponentChildAllocationPolicyError::ChildRoleNotAdmitted { .. } => {
                Self::invalid_input(message)
            }
            ComponentChildAllocationPolicyError::ParentComponentMismatch
            | ComponentChildAllocationPolicyError::ParentCallerMismatch
            | ComponentChildAllocationPolicyError::ParentRegistryMemberNotActive
            | ComponentChildAllocationPolicyError::SpawnGrantMissing { .. } => {
                Self::forbidden(message)
            }
            ComponentChildAllocationPolicyError::FleetRegistryRootNotActive
            | ComponentChildAllocationPolicyError::RootRuntimeNotActive
            | ComponentChildAllocationPolicyError::ComponentRegistryNotActive => {
                Self::unavailable(message)
            }
            ComponentChildAllocationPolicyError::ComponentRegistryAuthorityMismatch => {
                Self::conflict(message)
            }
            ComponentChildAllocationPolicyError::ParentRoleCountOverflow
            | ComponentChildAllocationPolicyError::ParentRoleCapacityExhausted { .. }
            | ComponentChildAllocationPolicyError::ComponentDescendantCapacityExhausted
            | ComponentChildAllocationPolicyError::ComponentCountOverflow
            | ComponentChildAllocationPolicyError::ManagedCanisterCountOverflow
            | ComponentChildAllocationPolicyError::ManagedCanisterCapacityExhausted => {
                Self::resource_exhausted(message)
            }
            ComponentChildAllocationPolicyError::InvalidComponentBinding
            | ComponentChildAllocationPolicyError::InvalidParentBinding
            | ComponentChildAllocationPolicyError::ComponentSpecUnknown(_) => {
                Self::invariant(InternalErrorOrigin::Domain, message)
            }
        }
    }
}

const fn registry_policy_error_code(err: &RegistryPolicyError) -> Option<PublicErrorCode> {
    match err {
        RegistryPolicyError::RoleAlreadyRegistered { .. } => {
            Some(PublicErrorCode::PolicyRoleAlreadyRegistered)
        }
        RegistryPolicyError::SingletonAlreadyRegisteredUnderParent { .. } => {
            Some(PublicErrorCode::PolicySingletonAlreadyRegisteredUnderParent)
        }
        RegistryPolicyError::ReplicaRequiresServiceWithScaling { .. } => {
            Some(PublicErrorCode::PolicyReplicaRequiresServiceWithScaling)
        }
        RegistryPolicyError::ShardRequiresServiceWithSharding { .. } => {
            Some(PublicErrorCode::PolicyShardRequiresServiceWithSharding)
        }
        RegistryPolicyError::InstanceRequiresServiceWithIndex { .. } => {
            Some(PublicErrorCode::PolicyInstanceRequiresServiceWithIndex)
        }
        RegistryPolicyError::ServiceRequiresRootParent { .. } => None,
    }
}

///
/// InternalErrorClass
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalErrorClass {
    Access,
    Domain,
    Infra,
    Ops,
    Workflow,
    Invariant,
}

///
/// InternalErrorOrigin
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalErrorOrigin {
    Access,
    Config,
    Domain,
    Infra,
    Ops,
    Storage,
    Workflow,
}

impl fmt::Display for InternalErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Access => "Access",
            Self::Domain => "Domain",
            Self::Infra => "Infra",
            Self::Ops => "Ops",
            Self::Workflow => "Workflow",
            Self::Invariant => "Invariant",
        };

        f.write_str(label)
    }
}

impl fmt::Display for InternalErrorOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Access => "Access",
            Self::Config => "Config",
            Self::Domain => "Domain",
            Self::Infra => "Infra",
            Self::Ops => "Ops",
            Self::Storage => "Storage",
            Self::Workflow => "Workflow",
        };

        f.write_str(label)
    }
}
