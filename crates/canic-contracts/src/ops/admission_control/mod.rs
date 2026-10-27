//! Lossless admission transport conversion; no policy or persistent-state mutation.
use crate::dto::{admission_control as native, fleet_admission as stored, page::Page};

impl From<stored::FleetAdmissionMutationRequest> for native::MutationRequest {
    fn from(value: stored::FleetAdmissionMutationRequest) -> Self {
        Self {
            authority: value.authority.into(),
            expected_generation: value.expected_generation,
            expected_policy_digest: value.expected_policy_digest,
            action: value.action,
            selector: value.selector.into(),
            principal: value.principal.into(),
            operation_id: value.operation_id,
            successor_policy_digest: value.successor_policy_digest,
            participant_catalog_digest: value.participant_catalog_digest,
            participant_count: value.participant_count,
        }
    }
}

impl From<native::MutationRequest> for stored::FleetAdmissionMutationRequest {
    fn from(value: native::MutationRequest) -> Self {
        Self {
            authority: value.authority.into(),
            expected_generation: value.expected_generation,
            expected_policy_digest: value.expected_policy_digest,
            action: value.action,
            selector: value.selector.into(),
            principal: value.principal.into(),
            operation_id: value.operation_id,
            successor_policy_digest: value.successor_policy_digest,
            participant_catalog_digest: value.participant_catalog_digest,
            participant_count: value.participant_count,
        }
    }
}

impl From<stored::FleetAdmissionOperationStatusResponse> for native::OperationStatus {
    fn from(value: stored::FleetAdmissionOperationStatusResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            action: value.action,
            selector: value.selector.into(),
            principal: value.principal.into(),
            phase: value.phase,
        }
    }
}

impl From<native::OperationStatus> for stored::FleetAdmissionOperationStatusResponse {
    fn from(value: native::OperationStatus) -> Self {
        Self {
            operation_id: value.operation_id,
            action: value.action,
            selector: value.selector.into(),
            principal: value.principal.into(),
            phase: value.phase,
        }
    }
}

impl From<stored::FleetAdmissionStatusRequest> for native::StatusRequest {
    fn from(value: stored::FleetAdmissionStatusRequest) -> Self {
        Self {
            selector: value.selector.into(),
            page: value.page,
        }
    }
}

impl From<native::StatusRequest> for stored::FleetAdmissionStatusRequest {
    fn from(value: native::StatusRequest) -> Self {
        Self {
            selector: value.selector.into(),
            page: value.page,
        }
    }
}

impl From<stored::FleetAdmissionStatusResponse> for native::StatusResponse {
    fn from(value: stored::FleetAdmissionStatusResponse) -> Self {
        Self {
            fleet: value.fleet,
            active: value.active,
            selector: value.selector.into(),
            principals: Page {
                entries: value
                    .principals
                    .entries
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                total: value.principals.total,
            },
            maximum_page_size: value.maximum_page_size,
            current_operation: value.current_operation.map(Into::into),
            last_result: value.last_result.map(Into::into),
        }
    }
}

impl From<native::StatusResponse> for stored::FleetAdmissionStatusResponse {
    fn from(value: native::StatusResponse) -> Self {
        Self {
            fleet: value.fleet,
            active: value.active,
            selector: value.selector.into(),
            principals: Page {
                entries: value
                    .principals
                    .entries
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                total: value.principals.total,
            },
            maximum_page_size: value.maximum_page_size,
            current_operation: value.current_operation.map(Into::into),
            last_result: value.last_result.map(Into::into),
        }
    }
}

impl From<stored::FleetAdmissionRootReceipt> for native::RootReceipt {
    fn from(value: stored::FleetAdmissionRootReceipt) -> Self {
        Self {
            operation_id: value.operation_id,
            phase: value.phase,
            root: value.root.into(),
            generation: value.generation,
            policy_digest: value.policy_digest,
            participant_catalog_digest: value.participant_catalog_digest,
            participant_count: value.participant_count,
            receipt_hash: value.receipt_hash,
        }
    }
}

impl From<native::RootReceipt> for stored::FleetAdmissionRootReceipt {
    fn from(value: native::RootReceipt) -> Self {
        Self {
            operation_id: value.operation_id,
            phase: value.phase,
            root: value.root.into(),
            generation: value.generation,
            policy_digest: value.policy_digest,
            participant_catalog_digest: value.participant_catalog_digest,
            participant_count: value.participant_count,
            receipt_hash: value.receipt_hash,
        }
    }
}

impl From<stored::FleetAdmissionRootParticipantStatus> for native::RootParticipantStatus {
    fn from(value: stored::FleetAdmissionRootParticipantStatus) -> Self {
        Self {
            target: value.target.into(),
            projection_digest: value.projection_digest,
            phase: value.phase,
            last_receipt_hash: value.last_receipt_hash,
        }
    }
}

impl From<native::RootParticipantStatus> for stored::FleetAdmissionRootParticipantStatus {
    fn from(value: native::RootParticipantStatus) -> Self {
        Self {
            target: value.target.into(),
            projection_digest: value.projection_digest,
            phase: value.phase,
            last_receipt_hash: value.last_receipt_hash,
        }
    }
}

impl From<stored::FleetAdmissionRootStatusResponse> for native::RootStatus {
    fn from(value: stored::FleetAdmissionRootStatusResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            phase: value.phase,
            active_generation: value.active_generation,
            active_policy_digest: value.active_policy_digest,
            successor_generation: value.successor_generation,
            successor_policy_digest: value.successor_policy_digest,
            participant_catalog_digest: value.participant_catalog_digest,
            participants: Page {
                entries: value
                    .participants
                    .entries
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                total: value.participants.total,
            },
            maximum_page_size: value.maximum_page_size,
            last_result: value.last_result.map(Into::into),
        }
    }
}

impl From<native::RootStatus> for stored::FleetAdmissionRootStatusResponse {
    fn from(value: native::RootStatus) -> Self {
        Self {
            operation_id: value.operation_id,
            phase: value.phase,
            active_generation: value.active_generation,
            active_policy_digest: value.active_policy_digest,
            successor_generation: value.successor_generation,
            successor_policy_digest: value.successor_policy_digest,
            participant_catalog_digest: value.participant_catalog_digest,
            participants: Page {
                entries: value
                    .participants
                    .entries
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                total: value.participants.total,
            },
            maximum_page_size: value.maximum_page_size,
            last_result: value.last_result.map(Into::into),
        }
    }
}

impl From<stored::FleetAdmissionProjectionStatusResponse> for native::ManagedStatus {
    fn from(value: stored::FleetAdmissionProjectionStatusResponse) -> Self {
        Self {
            authority: value.authority.into(),
            target: value.target.into(),
            generation: value.generation,
            policy_digest: value.policy_digest,
            projection_digest: value.projection_digest,
            phase: value.phase,
            prepared: value.prepared,
            principals: Page {
                entries: value
                    .principals
                    .entries
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                total: value.principals.total,
            },
            maximum_page_size: value.maximum_page_size,
        }
    }
}

impl From<native::ManagedStatus> for stored::FleetAdmissionProjectionStatusResponse {
    fn from(value: native::ManagedStatus) -> Self {
        Self {
            authority: value.authority.into(),
            target: value.target.into(),
            generation: value.generation,
            policy_digest: value.policy_digest,
            projection_digest: value.projection_digest,
            phase: value.phase,
            prepared: value.prepared,
            principals: Page {
                entries: value
                    .principals
                    .entries
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                total: value.principals.total,
            },
            maximum_page_size: value.maximum_page_size,
        }
    }
}

impl From<crate::ids::FleetAdmissionProjection> for native::TargetProjection {
    fn from(value: crate::ids::FleetAdmissionProjection) -> Self {
        Self {
            schema_version: value.schema_version,
            authority: value.authority.into(),
            target: value.target.into(),
            generation: value.generation,
            policy_digest: value.policy_digest,
            projection_digest: value.projection_digest,
            principals: value.principals.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::TargetProjection> for crate::ids::FleetAdmissionProjection {
    fn from(value: native::TargetProjection) -> Self {
        Self {
            schema_version: value.schema_version,
            authority: value.authority.into(),
            target: value.target.into(),
            generation: value.generation,
            policy_digest: value.policy_digest,
            projection_digest: value.projection_digest,
            principals: value.principals.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<stored::FleetAdmissionPrepareTargetRequest> for native::PrepareTargetRequest {
    fn from(value: stored::FleetAdmissionPrepareTargetRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            expected_generation: value.expected_generation,
            expected_policy_digest: value.expected_policy_digest,
            successor: value.successor.into(),
        }
    }
}

impl From<native::PrepareTargetRequest> for stored::FleetAdmissionPrepareTargetRequest {
    fn from(value: native::PrepareTargetRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            expected_generation: value.expected_generation,
            expected_policy_digest: value.expected_policy_digest,
            successor: value.successor.into(),
        }
    }
}

impl From<stored::FleetAdmissionTargetReceipt> for native::TargetReceipt {
    fn from(value: stored::FleetAdmissionTargetReceipt) -> Self {
        Self {
            operation_id: value.operation_id,
            phase: value.phase,
            target: value.target.into(),
            generation: value.generation,
            policy_digest: value.policy_digest,
            projection_digest: value.projection_digest,
            receipt_hash: value.receipt_hash,
        }
    }
}

impl From<native::TargetReceipt> for stored::FleetAdmissionTargetReceipt {
    fn from(value: native::TargetReceipt) -> Self {
        Self {
            operation_id: value.operation_id,
            phase: value.phase,
            target: value.target.into(),
            generation: value.generation,
            policy_digest: value.policy_digest,
            projection_digest: value.projection_digest,
            receipt_hash: value.receipt_hash,
        }
    }
}

impl From<stored::FleetAdmissionPrepareRootRequest> for native::PrepareRootRequest {
    fn from(value: stored::FleetAdmissionPrepareRootRequest) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            expected_generation: value.expected_generation,
            expected_policy_digest: value.expected_policy_digest,
            successor: value.successor.into(),
            stage: value.stage,
        }
    }
}

impl From<native::PrepareRootRequest> for stored::FleetAdmissionPrepareRootRequest {
    fn from(value: native::PrepareRootRequest) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            expected_generation: value.expected_generation,
            expected_policy_digest: value.expected_policy_digest,
            successor: value.successor.into(),
            stage: value.stage,
        }
    }
}

impl From<stored::FleetAdmissionActivateRootRequest> for native::ActivateRootRequest {
    fn from(value: stored::FleetAdmissionActivateRootRequest) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            expected_generation: value.expected_generation,
            expected_policy_digest: value.expected_policy_digest,
            successor_generation: value.successor_generation,
            successor_policy_digest: value.successor_policy_digest,
        }
    }
}

impl From<native::ActivateRootRequest> for stored::FleetAdmissionActivateRootRequest {
    fn from(value: native::ActivateRootRequest) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            expected_generation: value.expected_generation,
            expected_policy_digest: value.expected_policy_digest,
            successor_generation: value.successor_generation,
            successor_policy_digest: value.successor_policy_digest,
        }
    }
}

impl From<stored::FleetAdmissionOpenRootRequest> for native::OpenRootRequest {
    fn from(value: stored::FleetAdmissionOpenRootRequest) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            generation: value.generation,
            policy_digest: value.policy_digest,
        }
    }
}

impl From<native::OpenRootRequest> for stored::FleetAdmissionOpenRootRequest {
    fn from(value: native::OpenRootRequest) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            generation: value.generation,
            policy_digest: value.policy_digest,
        }
    }
}
