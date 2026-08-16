use crate::access::AccessError;
use crate::domain::policy::pure::{
    component_allocation::ComponentAllocationPolicyError,
    component_child_allocation::ComponentChildAllocationPolicyError,
};
use crate::{
    diagnostics::{RegisteredDiagnosticCode, codes},
    dto::error::Error as PublicError,
};
use std::fmt;

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

#[derive(Debug)]
pub struct InternalError {
    code: RegisteredDiagnosticCode,
    projection: PublicProjection,
}

#[derive(Clone, Copy, Debug)]
enum PublicProjection {
    Registered(RegisteredDiagnosticCode),
    Forwarded(PublicError),
}

impl InternalError {
    const fn new(code: RegisteredDiagnosticCode, public_code: RegisteredDiagnosticCode) -> Self {
        Self {
            code,
            projection: PublicProjection::Registered(public_code),
        }
    }

    #[must_use]
    pub const fn public(code: RegisteredDiagnosticCode) -> Self {
        Self::projected(code, code)
    }

    #[must_use]
    pub const fn projected(
        code: RegisteredDiagnosticCode,
        public_code: RegisteredDiagnosticCode,
    ) -> Self {
        Self {
            code,
            projection: PublicProjection::Registered(public_code),
        }
    }

    /// Preserve a decoded remote rejection while assigning its local transport
    /// failure a registered exact identity.
    #[must_use]
    pub const fn observed_public(err: PublicError) -> Self {
        Self {
            code: codes::PLATFORM_FAILED,
            projection: PublicProjection::Forwarded(err),
        }
    }

    #[must_use]
    pub const fn forbidden() -> Self {
        Self::public(crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED)
    }

    #[must_use]
    pub const fn invalid_input() -> Self {
        Self::public(crate::diagnostics::codes::REQUEST_INVALID)
    }

    #[must_use]
    pub const fn conflict() -> Self {
        Self::public(crate::diagnostics::codes::STATE_CONFLICT)
    }

    #[must_use]
    pub const fn unavailable() -> Self {
        Self::public(crate::diagnostics::codes::STATE_UNAVAILABLE)
    }

    #[must_use]
    pub const fn resource_exhausted() -> Self {
        Self::public(crate::diagnostics::codes::CAPACITY_LIMIT)
    }

    #[must_use]
    pub const fn auth_material_stale() -> Self {
        Self::public(codes::SECURITY_CONFLICT)
    }

    #[must_use]
    pub const fn auth_proof_expired() -> Self {
        Self::public(codes::AUTH_CERT_EXPIRED)
    }

    #[must_use]
    pub const fn auth_token_expired() -> Self {
        Self::public(crate::diagnostics::codes::AUTH_TOKEN_EXPIRED)
    }

    #[must_use]
    pub const fn auth_proof_pending() -> Self {
        Self::public(crate::diagnostics::codes::SECURITY_UNAVAILABLE)
    }

    #[must_use]
    pub const fn operation_id_required() -> Self {
        Self::public(crate::diagnostics::codes::AUTHORITY_UNAVAILABLE)
    }

    #[must_use]
    pub const fn root_data_certificate_unavailable() -> Self {
        Self::public(crate::diagnostics::codes::SECURITY_UNAVAILABLE)
    }

    #[must_use]
    pub const fn invariant() -> Self {
        Self::new(codes::STATE_INVALID, codes::STATE_INVALID)
    }

    #[must_use]
    pub const fn platform_failure() -> Self {
        Self::new(codes::PLATFORM_FAILED, codes::STATE_FAILED)
    }

    #[must_use]
    pub const fn state_failure() -> Self {
        Self::new(codes::STATE_FAILED, codes::STATE_FAILED)
    }

    #[must_use]
    pub const fn lifecycle_failure() -> Self {
        Self::new(codes::LIFECYCLE_FAILED, codes::STATE_FAILED)
    }

    /// Return the exact registered diagnostic identity.
    #[must_use]
    pub const fn code(&self) -> RegisteredDiagnosticCode {
        self.code
    }

    /// Return the reviewed safe public projection.
    #[must_use]
    pub const fn public_code(&self) -> Option<RegisteredDiagnosticCode> {
        match self.projection {
            PublicProjection::Registered(code) => Some(code),
            PublicProjection::Forwarded(_) => None,
        }
    }

    /// Construct the reviewed public projection or preserve a decoded remote
    /// rejection unchanged.
    #[must_use]
    pub const fn public_error(&self) -> PublicError {
        match self.projection {
            PublicProjection::Registered(code) => PublicError::from_registered(code),
            PublicProjection::Forwarded(error) => error,
        }
    }

    #[must_use]
    pub fn is_public_resource_exhausted(&self) -> bool {
        self.public_error().code() == codes::CAPACITY_LIMIT.raw_code()
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.code, f)
    }
}

impl std::error::Error for InternalError {}

impl From<AccessError> for InternalError {
    fn from(err: AccessError) -> Self {
        match err {
            AccessError::Internal(error) => error,
            error => {
                let diagnostic = error
                    .diagnostic_codes()
                    .expect("non-internal access errors have registered reasons");
                Self::projected(diagnostic.exact, diagnostic.public)
            }
        }
    }
}

impl From<ComponentAllocationPolicyError> for InternalError {
    fn from(err: ComponentAllocationPolicyError) -> Self {
        use crate::diagnostics::codes;

        let code = match err {
            ComponentAllocationPolicyError::EmptyOperationId => codes::REQUEST_INCOMPLETE,
            ComponentAllocationPolicyError::AllocationSequenceExhausted => codes::VERSION_CAPACITY,
            ComponentAllocationPolicyError::InvalidRootTopologyProjection => {
                codes::COLLECTION_INVALID
            }
            ComponentAllocationPolicyError::RootTopologyDigestMismatch
            | ComponentAllocationPolicyError::ComponentSpecHashMismatch(_) => {
                codes::DIGEST_CONFLICT
            }
            ComponentAllocationPolicyError::ComponentSpecNotAdmitted(_) => {
                codes::AUTHORITY_INVALID_STATE
            }
            ComponentAllocationPolicyError::ComponentSpecUnknown(_)
            | ComponentAllocationPolicyError::PeerProvisioningGrantMissing { .. } => {
                codes::CONFIGURATION_UNAVAILABLE
            }
            ComponentAllocationPolicyError::ComponentCountOverflow
            | ComponentAllocationPolicyError::ComponentCapacityExhausted
            | ComponentAllocationPolicyError::ComponentSpecCountOverflow(_)
            | ComponentAllocationPolicyError::ComponentSpecCapacityExhausted(_)
            | ComponentAllocationPolicyError::PeerProvisioningCountOverflow
            | ComponentAllocationPolicyError::PeerProvisioningCapacityExhausted { .. } => {
                codes::CAPACITY_LIMIT
            }
            ComponentAllocationPolicyError::InvalidPeerRequesterBinding => codes::AUTHORITY_INVALID,
            ComponentAllocationPolicyError::PeerRootRuntimeInactive
            | ComponentAllocationPolicyError::PeerRequesterRegistryMemberInactive => {
                codes::AUTHORITY_INACTIVE
            }
        };
        Self::public(code)
    }
}

impl From<ComponentChildAllocationPolicyError> for InternalError {
    fn from(err: ComponentChildAllocationPolicyError) -> Self {
        use crate::diagnostics::codes;

        let code = match err {
            ComponentChildAllocationPolicyError::EmptyOperationId => codes::REQUEST_INCOMPLETE,
            ComponentChildAllocationPolicyError::InvalidComponentBinding
            | ComponentChildAllocationPolicyError::InvalidParentBinding => codes::AUTHORITY_INVALID,
            ComponentChildAllocationPolicyError::ParentComponentMismatch
            | ComponentChildAllocationPolicyError::ParentCallerMismatch
            | ComponentChildAllocationPolicyError::ComponentRegistryAuthorityMismatch => {
                codes::AUTHORITY_CONFLICT
            }
            ComponentChildAllocationPolicyError::FleetRegistryRootNotActive
            | ComponentChildAllocationPolicyError::RootRuntimeNotActive
            | ComponentChildAllocationPolicyError::ComponentRegistryNotActive
            | ComponentChildAllocationPolicyError::ParentRegistryMemberNotActive => {
                codes::AUTHORITY_INACTIVE
            }
            ComponentChildAllocationPolicyError::ComponentSpecUnknown(_)
            | ComponentChildAllocationPolicyError::SpawnGrantMissing { .. } => {
                codes::CONFIGURATION_UNAVAILABLE
            }
            ComponentChildAllocationPolicyError::ChildRoleNotAdmitted { .. } => {
                codes::AUTHORITY_INVALID_STATE
            }
            ComponentChildAllocationPolicyError::ParentRoleCountOverflow
            | ComponentChildAllocationPolicyError::ParentRoleCapacityExhausted { .. }
            | ComponentChildAllocationPolicyError::ComponentDescendantCapacityExhausted
            | ComponentChildAllocationPolicyError::ComponentCountOverflow => codes::CAPACITY_LIMIT,
            ComponentChildAllocationPolicyError::InvalidDeploymentLimits => {
                codes::CONFIGURATION_INVALID
            }
        };
        Self::public(code)
    }
}
