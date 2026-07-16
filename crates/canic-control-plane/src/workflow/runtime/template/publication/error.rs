//! Module: workflow::runtime::template::publication::error
//!
//! Responsibility: classify publication workflow failures before public projection.
//! Does not own: endpoint DTO construction, metrics, or store-side validation.
//! Boundary: publication workflow code raises these causes and converts once to internal errors.

use crate::ids::{CanisterRole, TemplateId, TemplateVersion, WasmStoreBinding, WasmStoreGcMode};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::error::InternalError,
    dto::error::{Error, ErrorCode},
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
        "wasm store lifecycle state changed for {binding}: expected generation {expected_generation}, found {actual_generation}"
    )]
    LifecycleStateChanged {
        binding: WasmStoreBinding,
        expected_generation: u64,
        actual_generation: u64,
    },

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

    #[error("wasm store {0} is not registered")]
    StoreNotRegistered(Principal),

    #[error(
        "wasm store binding '{binding}' gc state changed: expected {expected:?}, found {actual:?}"
    )]
    StoreGcStateChanged {
        binding: WasmStoreBinding,
        expected: WasmStoreGcMode,
        actual: WasmStoreGcMode,
    },

    #[error("wasm store binding '{binding}' is not writable while gc={mode:?}")]
    StoreNotWritable {
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
        let code = match &err {
            PublicationWorkflowError::CapacityExceeded { .. } => {
                ErrorCode::WasmStoreCapacityExceeded
            }
            PublicationWorkflowError::ChunkHashMismatch { .. } => ErrorCode::WasmStoreHashMismatch,
            PublicationWorkflowError::ChunkIndexOverflow { .. }
            | PublicationWorkflowError::InvalidState(_) => ErrorCode::InvariantViolation,
            PublicationWorkflowError::ExactReleaseMissing { .. } => {
                ErrorCode::WasmStoreManifestMissing
            }
            PublicationWorkflowError::LifecycleBusy
            | PublicationWorkflowError::LifecycleStateChanged { .. }
            | PublicationWorkflowError::ReleaseConflict { .. }
            | PublicationWorkflowError::StoreGcStateChanged { .. }
            | PublicationWorkflowError::StoreNotWritable { .. } => ErrorCode::Conflict,
            PublicationWorkflowError::StoreNotRegistered(_) => ErrorCode::NotFound,
            PublicationWorkflowError::TransportUnavailable { .. } => ErrorCode::Unavailable,
        };

        Self::public(Error::new(code, err.to_string()))
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::control_plane_support::error::InternalErrorOrigin;

    fn public_code(err: PublicationWorkflowError) -> ErrorCode {
        InternalError::from(err)
            .public_error()
            .expect("publication causes are public")
            .code
    }

    #[test]
    fn publication_causes_map_to_stable_public_codes() {
        let cases = [
            (
                PublicationWorkflowError::CapacityExceeded {
                    release: "app@1".to_string(),
                    target: "primary".to_string(),
                    payload_size_bytes: 20,
                    remaining_store_bytes: 10,
                },
                ErrorCode::WasmStoreCapacityExceeded,
            ),
            (
                PublicationWorkflowError::ExactReleaseMissing {
                    role: CanisterRole::new("app"),
                    template_id: TemplateId::new("embedded:app"),
                    version: TemplateVersion::new("1"),
                    expected_binding: WasmStoreBinding::new("primary"),
                },
                ErrorCode::WasmStoreManifestMissing,
            ),
            (
                PublicationWorkflowError::ChunkHashMismatch {
                    template_id: TemplateId::new("embedded:app"),
                    chunk_index: 2,
                    store_pid: Principal::anonymous(),
                },
                ErrorCode::WasmStoreHashMismatch,
            ),
            (
                PublicationWorkflowError::ChunkIndexOverflow {
                    template_id: TemplateId::new("embedded:app"),
                },
                ErrorCode::InvariantViolation,
            ),
            (
                PublicationWorkflowError::InvalidState("missing snapshot".to_string()),
                ErrorCode::InvariantViolation,
            ),
            (PublicationWorkflowError::LifecycleBusy, ErrorCode::Conflict),
            (
                PublicationWorkflowError::LifecycleStateChanged {
                    binding: WasmStoreBinding::new("primary"),
                    expected_generation: 3,
                    actual_generation: 4,
                },
                ErrorCode::Conflict,
            ),
            (
                PublicationWorkflowError::ReleaseConflict {
                    template_id: TemplateId::new("embedded:app"),
                    version: TemplateVersion::new("1"),
                    binding: WasmStoreBinding::new("primary"),
                    existing_payload_hash: vec![7; 32],
                    existing_payload_size_bytes: 10,
                },
                ErrorCode::Conflict,
            ),
            (
                PublicationWorkflowError::StoreNotRegistered(Principal::anonymous()),
                ErrorCode::NotFound,
            ),
            (
                PublicationWorkflowError::StoreGcStateChanged {
                    binding: WasmStoreBinding::new("retired"),
                    expected: WasmStoreGcMode::Complete,
                    actual: WasmStoreGcMode::InProgress,
                },
                ErrorCode::Conflict,
            ),
            (
                PublicationWorkflowError::StoreNotWritable {
                    binding: WasmStoreBinding::new("retired"),
                    mode: WasmStoreGcMode::Complete,
                },
                ErrorCode::Conflict,
            ),
            (
                PublicationWorkflowError::TransportUnavailable {
                    surface: "store status",
                    cause: InternalError::infra(InternalErrorOrigin::Infra, "rejected"),
                },
                ErrorCode::Unavailable,
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(public_code(err), expected);
        }
    }
}
