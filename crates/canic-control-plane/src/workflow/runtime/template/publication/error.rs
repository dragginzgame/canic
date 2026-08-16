//! Module: workflow::runtime::template::publication::error
//!
//! Responsibility: classify publication workflow failures before public projection.
//! Does not own: endpoint DTO construction, metrics, or store-side validation.
//! Boundary: publication workflow code raises these causes and converts once to internal errors.

use crate::ids::{CanisterRole, TemplateId, TemplateVersion, WasmStoreBinding, WasmStoreGcMode};
use canic_core::{
    cdk::types::Principal, control_plane_support::error::InternalError, diagnostics::codes, log,
    log::Topic,
};
use thiserror::Error as ThisError;

///
/// PublicationWorkflowError
///
/// Typed causes owned by the root publication workflow.
///

#[derive(Debug, ThisError)]
pub(super) enum PublicationWorkflowError {
    #[error(
        "publication state does not name wasm store binding '{binding}' as its sole active authority"
    )]
    SoleActivePublicationBindingRequired { binding: WasmStoreBinding },

    #[error("publication requires exactly one adopted wasm store, found {observed_count}")]
    SingleAdoptedStoreRequired { observed_count: usize },

    #[error(
        "publication binding '{requested_binding}' does not match adopted wasm store binding '{adopted_binding}'"
    )]
    AdoptedBindingMismatch {
        requested_binding: WasmStoreBinding,
        adopted_binding: WasmStoreBinding,
    },

    #[error("initial publication cannot replace or rotate existing wasm store authority")]
    InitialPublicationAuthorityPresent,

    #[error("initial publication binding did not commit")]
    InitialPublicationBindingCommitFailed,

    #[error(
        "release {release} cannot fit wasm store target {target}: payload_bytes={payload_size_bytes}, remaining_bytes={remaining_store_bytes}"
    )]
    CapacityExceeded {
        release: String,
        target: String,
        payload_size_bytes: u64,
        remaining_store_bytes: u64,
    },

    #[error("template '{template_id}' chunk {chunk_index} hash mismatch for {store_pid}")]
    ChunkHashMismatch {
        template_id: TemplateId,
        chunk_index: u32,
        store_pid: Principal,
    },

    #[error("template '{template_id}' exceeds chunk index bounds")]
    ChunkIndexOverflow { template_id: TemplateId },

    #[error(
        "fleet import missing exact release for role '{role}': expected {template_id}@{version} on {expected_binding}"
    )]
    ExactReleaseMissing {
        role: CanisterRole,
        template_id: TemplateId,
        version: TemplateVersion,
        expected_binding: WasmStoreBinding,
    },

    #[error("publication state invariant failed: {0}")]
    InvalidState(String),

    #[error("wasm store lifecycle operation is already in progress")]
    LifecycleBusy,

    #[error(
        "ws conflict for {template_id}@{version} on {binding}: existing hash/size differ ({existing_payload_hash:?}, {existing_payload_size_bytes})"
    )]
    ReleaseConflict {
        template_id: TemplateId,
        version: TemplateVersion,
        binding: WasmStoreBinding,
        existing_payload_hash: Vec<u8>,
        existing_payload_size_bytes: u64,
    },

    #[error("wasm store binding '{binding}' is write-fenced while gc={mode:?}")]
    GcWriteFenced {
        binding: WasmStoreBinding,
        mode: WasmStoreGcMode,
    },

    #[error("publication transport unavailable at {surface}: {cause}")]
    TransportUnavailable {
        surface: &'static str,
        cause: InternalError,
    },
}

impl From<PublicationWorkflowError> for InternalError {
    fn from(err: PublicationWorkflowError) -> Self {
        let mapped = match err {
            PublicationWorkflowError::SoleActivePublicationBindingRequired { .. } => {
                Self::public(codes::AUTHORITY_UNAVAILABLE)
            }
            PublicationWorkflowError::SingleAdoptedStoreRequired { .. } => {
                Self::public(codes::ARTIFACT_UNAVAILABLE)
            }
            PublicationWorkflowError::ExactReleaseMissing { .. } => {
                Self::public(codes::WASM_STORE_MANIFEST_MISSING)
            }
            PublicationWorkflowError::AdoptedBindingMismatch { .. } => {
                Self::public(codes::AUTHORITY_CONFLICT)
            }
            PublicationWorkflowError::InitialPublicationAuthorityPresent => {
                Self::public(codes::AUTHORITY_UNEXPECTED_STATE)
            }
            PublicationWorkflowError::InitialPublicationBindingCommitFailed => {
                Self::projected(codes::LIFECYCLE_FAILED, codes::STATE_INVALID)
            }
            PublicationWorkflowError::CapacityExceeded { .. } => {
                Self::public(codes::CAPACITY_LIMIT)
            }
            PublicationWorkflowError::ChunkHashMismatch { .. } => {
                Self::public(codes::DIGEST_CONFLICT)
            }
            PublicationWorkflowError::ChunkIndexOverflow { .. } => {
                Self::public(codes::POSITION_CAPACITY)
            }
            PublicationWorkflowError::InvalidState(_) => Self::public(codes::STATE_INVALID),
            PublicationWorkflowError::LifecycleBusy => Self::public(codes::REQUEST_INCOMPLETE),
            PublicationWorkflowError::ReleaseConflict { .. } => {
                Self::public(codes::ARTIFACT_CONFLICT)
            }
            PublicationWorkflowError::GcWriteFenced { .. } => Self::public(codes::STORAGE_INACTIVE),
            PublicationWorkflowError::TransportUnavailable { cause, .. } => {
                Self::projected(cause.code(), codes::PLATFORM_UNAVAILABLE)
            }
        };
        log!(
            Topic::Wasm,
            Warn,
            "ws failure code={} public_code={}",
            mapped.code(),
            mapped.public_error().code(),
        );
        mapped
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn public_code(err: PublicationWorkflowError) -> canic_core::diagnostics::DiagnosticCode {
        InternalError::from(err).public_error().code()
    }

    #[test]
    fn publication_causes_map_to_stable_public_codes() {
        let cases = [
            (
                PublicationWorkflowError::SoleActivePublicationBindingRequired {
                    binding: WasmStoreBinding::new("primary"),
                },
                codes::AUTHORITY_UNAVAILABLE.raw_code(),
            ),
            (
                PublicationWorkflowError::SingleAdoptedStoreRequired { observed_count: 2 },
                codes::ARTIFACT_UNAVAILABLE.raw_code(),
            ),
            (
                PublicationWorkflowError::AdoptedBindingMismatch {
                    requested_binding: WasmStoreBinding::new("requested"),
                    adopted_binding: WasmStoreBinding::new("adopted"),
                },
                codes::AUTHORITY_CONFLICT.raw_code(),
            ),
            (
                PublicationWorkflowError::InitialPublicationAuthorityPresent,
                codes::AUTHORITY_UNEXPECTED_STATE.raw_code(),
            ),
            (
                PublicationWorkflowError::InitialPublicationBindingCommitFailed,
                codes::STATE_INVALID.raw_code(),
            ),
            (
                PublicationWorkflowError::CapacityExceeded {
                    release: "app@1".to_string(),
                    target: "primary".to_string(),
                    payload_size_bytes: 20,
                    remaining_store_bytes: 10,
                },
                codes::CAPACITY_LIMIT.raw_code(),
            ),
            (
                PublicationWorkflowError::ExactReleaseMissing {
                    role: CanisterRole::new("app"),
                    template_id: TemplateId::new("embedded:app"),
                    version: TemplateVersion::new("1"),
                    expected_binding: WasmStoreBinding::new("primary"),
                },
                codes::WASM_STORE_MANIFEST_MISSING.raw_code(),
            ),
            (
                PublicationWorkflowError::ChunkHashMismatch {
                    template_id: TemplateId::new("embedded:app"),
                    chunk_index: 2,
                    store_pid: Principal::anonymous(),
                },
                codes::DIGEST_CONFLICT.raw_code(),
            ),
            (
                PublicationWorkflowError::ChunkIndexOverflow {
                    template_id: TemplateId::new("embedded:app"),
                },
                codes::POSITION_CAPACITY.raw_code(),
            ),
            (
                PublicationWorkflowError::InvalidState("missing snapshot".to_string()),
                codes::STATE_INVALID.raw_code(),
            ),
            (
                PublicationWorkflowError::LifecycleBusy,
                codes::REQUEST_INCOMPLETE.raw_code(),
            ),
            (
                PublicationWorkflowError::ReleaseConflict {
                    template_id: TemplateId::new("embedded:app"),
                    version: TemplateVersion::new("1"),
                    binding: WasmStoreBinding::new("primary"),
                    existing_payload_hash: vec![7; 32],
                    existing_payload_size_bytes: 10,
                },
                codes::ARTIFACT_CONFLICT.raw_code(),
            ),
            (
                PublicationWorkflowError::GcWriteFenced {
                    binding: WasmStoreBinding::new("retired"),
                    mode: WasmStoreGcMode::Complete,
                },
                codes::STORAGE_INACTIVE.raw_code(),
            ),
            (
                PublicationWorkflowError::TransportUnavailable {
                    surface: "store status",
                    cause: InternalError::platform_failure(),
                },
                codes::PLATFORM_UNAVAILABLE.raw_code(),
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(public_code(err), expected);
        }
    }

    fn assert_diagnostic_codes(
        error: PublicationWorkflowError,
        exact: canic_core::diagnostics::RegisteredDiagnosticCode,
        public: canic_core::diagnostics::RegisteredDiagnosticCode,
    ) {
        let error = InternalError::from(error);
        assert_eq!(error.code(), exact);
        assert_eq!(error.public_code(), Some(public));
    }

    #[test]
    fn finite_publication_causes_use_approved_registered_identities() {
        assert_diagnostic_codes(
            PublicationWorkflowError::SoleActivePublicationBindingRequired {
                binding: WasmStoreBinding::new("primary"),
            },
            codes::AUTHORITY_UNAVAILABLE,
            codes::AUTHORITY_UNAVAILABLE,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::SingleAdoptedStoreRequired { observed_count: 2 },
            codes::ARTIFACT_UNAVAILABLE,
            codes::ARTIFACT_UNAVAILABLE,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::AdoptedBindingMismatch {
                requested_binding: WasmStoreBinding::new("requested"),
                adopted_binding: WasmStoreBinding::new("adopted"),
            },
            codes::AUTHORITY_CONFLICT,
            codes::AUTHORITY_CONFLICT,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::InitialPublicationAuthorityPresent,
            codes::AUTHORITY_UNEXPECTED_STATE,
            codes::AUTHORITY_UNEXPECTED_STATE,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::InitialPublicationBindingCommitFailed,
            codes::LIFECYCLE_FAILED,
            codes::STATE_INVALID,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::CapacityExceeded {
                release: "app@1".to_string(),
                target: "primary".to_string(),
                payload_size_bytes: 20,
                remaining_store_bytes: 10,
            },
            codes::CAPACITY_LIMIT,
            codes::CAPACITY_LIMIT,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::ChunkHashMismatch {
                template_id: TemplateId::new("embedded:app"),
                chunk_index: 2,
                store_pid: Principal::anonymous(),
            },
            codes::DIGEST_CONFLICT,
            codes::DIGEST_CONFLICT,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::ChunkIndexOverflow {
                template_id: TemplateId::new("embedded:app"),
            },
            codes::POSITION_CAPACITY,
            codes::POSITION_CAPACITY,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::ExactReleaseMissing {
                role: CanisterRole::new("app"),
                template_id: TemplateId::new("embedded:app"),
                version: TemplateVersion::new("1"),
                expected_binding: WasmStoreBinding::new("primary"),
            },
            codes::WASM_STORE_MANIFEST_MISSING,
            codes::WASM_STORE_MANIFEST_MISSING,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::LifecycleBusy,
            codes::REQUEST_INCOMPLETE,
            codes::REQUEST_INCOMPLETE,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::ReleaseConflict {
                template_id: TemplateId::new("embedded:app"),
                version: TemplateVersion::new("1"),
                binding: WasmStoreBinding::new("primary"),
                existing_payload_hash: vec![7; 32],
                existing_payload_size_bytes: 10,
            },
            codes::ARTIFACT_CONFLICT,
            codes::ARTIFACT_CONFLICT,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::GcWriteFenced {
                binding: WasmStoreBinding::new("retired"),
                mode: WasmStoreGcMode::Complete,
            },
            codes::STORAGE_INACTIVE,
            codes::STORAGE_INACTIVE,
        );
    }

    #[test]
    fn broad_publication_wrappers_map_at_the_typed_boundary() {
        assert_diagnostic_codes(
            PublicationWorkflowError::InvalidState("missing snapshot".to_string()),
            codes::STATE_INVALID,
            codes::STATE_INVALID,
        );
        assert_diagnostic_codes(
            PublicationWorkflowError::TransportUnavailable {
                surface: "store status",
                cause: InternalError::platform_failure(),
            },
            codes::PLATFORM_FAILED,
            codes::PLATFORM_UNAVAILABLE,
        );
    }
}
