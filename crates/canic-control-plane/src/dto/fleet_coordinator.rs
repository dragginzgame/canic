//! Module: dto::fleet_coordinator
//!
//! Responsibility: carry the protected fresh-install input for one Fleet Coordinator.
//! Does not own: validation, stable state, Registry compilation, or lifecycle effects.
//! Boundary: the Coordinator lifecycle adapter passes this passive payload to workflow.

use candid::CandidType;
use canic_core::{
    control_plane_support::config::ComponentDeploymentConfiguration,
    dto::{
        authority_restore::AuthorityRestoreFenceStatusResponse,
        authority_restore::AuthoritySnapshotRequest,
        component_provisioning::{
            FleetComponentProvisioningPrepareRequest, FleetComponentProvisioningStatusResponse,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
            FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootDeletionCompletionRequest,
            FleetSubnetRootDeletionExecutionRequest, FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessResponse, FleetSubnetRootDeletionResponse,
            FleetSubnetRootDrainingPublicationResponse, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationResponse, FleetSubnetRootJoinRequest,
            FleetSubnetRootJoinResponse, FleetSubnetRootRemovalPublicationResponse,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
        },
        role::{OperationReceipt, OperationStatusRequest, RoleOverviewResponse},
    },
    ids::{AppId, FleetRegistryAuthority},
};
use serde::Deserialize;

///
/// FleetCoordinatorInitArgs
///
/// Exact authority and compiled provisioning configuration installed into a fresh Coordinator.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetCoordinatorInitArgs {
    pub configured_app: AppId,
    pub authority: FleetRegistryAuthority,
    pub component_deployment_configuration: ComponentDeploymentConfiguration,
}

/// Closed controller command union for the Fleet Coordinator.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorCommand {
    AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgementRequest),
    ActivateRegistry(FleetRegistryActivationRequest),
    CompleteRootDeletion(FleetSubnetRootDeletionCompletionRequest),
    JoinRoot(FleetSubnetRootJoinRequest),
    PrepareAuthoritySnapshot(AuthoritySnapshotRequest),
    PrepareRootDeletionExecution(FleetSubnetRootDeletionExecutionRequest),
    ProvisionComponents(FleetComponentProvisioningPrepareRequest),
    RemoveRoot(FleetSubnetRootDrainingReservationRequest),
    ResumeAuthoritySnapshot(AuthoritySnapshotRequest),
}

/// Closed correlated success union for Fleet Coordinator commands.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorCommandResponse {
    AcknowledgeRootSnapshot(FleetSubnetRootSnapshotAcknowledgement),
    ActivateRegistry(FleetRegistryActivationResponse),
    CompleteRootDeletion(FleetSubnetRootDeletionResponse),
    JoinRoot(FleetSubnetRootJoinResponse),
    OperationAccepted(OperationReceipt),
    PrepareAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
    PrepareRootDeletionExecution(FleetSubnetRootDeletionExecutionResponse),
    ResumeAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
}

/// Closed Coordinator observation selector carried by its single status query.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum CoordinatorStatusRequest {
    AuthorityRestore,
    Operation(OperationStatusRequest),
    Overview,
    Registry,
    RegistryManifest,
    RegistryVersion,
    RootAcknowledgements,
}

/// Coordinator-owned durable operation detail selected by one operation ID.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the accepted Candid union keeps each existing status DTO as its direct payload"
)]
pub enum CoordinatorOperationStatusResponse {
    ComponentProvisioning(FleetComponentProvisioningStatusResponse),
    RootRemoval(CoordinatorRootRemovalOperationStatus),
}

/// Coordinator-owned progress across the existing durable root-removal boundaries.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CoordinatorRootRemovalOperationStatus {
    pub operation_id: [u8; 32],
    pub reservation: FleetSubnetRootDrainingReservationResponse,
    pub draining: Option<FleetSubnetRootDrainingPublicationResponse>,
    pub removal: Option<FleetSubnetRootRemovalPublicationResponse>,
    pub readiness_intent: Option<FleetSubnetRootDeletionReadinessIntentResponse>,
    pub readiness: Option<FleetSubnetRootDeletionReadinessResponse>,
    pub execution: Option<FleetSubnetRootDeletionExecutionResponse>,
    pub completion: Option<FleetSubnetRootDeletionResponse>,
}

/// Closed response union for the Coordinator's single status query.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the accepted Candid union keeps each existing status DTO as its direct payload"
)]
pub enum CoordinatorStatusResponse {
    AuthorityRestore(AuthorityRestoreFenceStatusResponse),
    Operation(CoordinatorOperationStatusResponse),
    Overview(RoleOverviewResponse),
    Registry(FleetRegistry),
    RegistryManifest(FleetRegistryManifest),
    RegistryVersion(FleetRegistryVersion),
    RootAcknowledgements(Vec<FleetSubnetRootSnapshotAcknowledgement>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};

    #[test]
    fn coordinator_status_request_is_one_closed_candid_variant() {
        let requests = [
            CoordinatorStatusRequest::AuthorityRestore,
            CoordinatorStatusRequest::Operation(OperationStatusRequest {
                operation_id: [4; 32],
            }),
            CoordinatorStatusRequest::Overview,
            CoordinatorStatusRequest::Registry,
            CoordinatorStatusRequest::RegistryManifest,
            CoordinatorStatusRequest::RegistryVersion,
            CoordinatorStatusRequest::RootAcknowledgements,
        ];

        for request in requests {
            let bytes = Encode!(&request).expect("encode Coordinator status request");
            assert_eq!(
                Decode!(&bytes, CoordinatorStatusRequest)
                    .expect("decode Coordinator status request"),
                request
            );
        }
    }
}
