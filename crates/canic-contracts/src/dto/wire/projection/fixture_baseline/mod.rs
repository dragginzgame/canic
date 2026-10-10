//! Bounded Canic transports for framework validation.
//!
//! Retain only exercised selectors and reply payloads, checked against canonical contracts.

use crate::{
    dto::{
        authority_restore::{AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest},
        canister::{CanisterHistoryResponse, CanisterInspectionRequest},
        capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
        component_registry::{
            ComponentRegistryActivePartitionRequest, ComponentRegistryActivePartitionResponse,
            ComponentRegistryPartitionRequest, ComponentRegistryPartitionResponse,
            RootComponentAllocationRequest, RootComponentRegistryPreparationRequest,
            RootComponentRegistryStatusResponse, RootComponentSubtreeRemovalRequest,
        },
        cycles::CycleTrackerEntry,
        fixture_provisioning::{FixtureSourceStatus, FixtureStoreError},
        fleet_activation::FleetActivationResumeRequest,
        fleet_admission::{
            FleetAdmissionProjectionStatusResponse, FleetAdmissionRootStatusResponse,
        },
        fleet_registry::FleetSubnetRootRegistrySyncRequest,
        fleet_subnet_root::{
            FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary,
            FleetSubnetWasmStoreAdoptionRequest,
        },
        metrics::MetricEntry,
        observability::{CanisterObservabilityResponse, FleetCanisterObservabilityRequest},
        page::{Page, PageRequest},
        pool::{
            CanisterPoolResponse, CanisterPoolStatusRequest, PoolCanisterRequest,
            PoolImportResponse, PoolMaintenanceResponse,
        },
        public_status::{PublicHealth, PublicMetricsRequest, PublicMetricsSnapshot},
        role::{
            CycleBalanceStatusResponse, MetricsStatusRequest, OperationReceipt,
            OperationStatusRequest,
        },
        root::{RootFundingStatusResponse, RootOperationStatusResponse},
        root_store::{RootStoreBootstrapRequest, RootStoreFixturePrepareRequest},
        runtime::CanicRuntimeStatus,
        wire::projection::component_registry::CanisterOperationStatusFragment as ManagedOperationStatusResponseFragment,
    },
    ids::ManagedCanisterBinding,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded transport selectors used by framework validation.
#[derive(CandidType)]
pub enum RootCommandFragment {
    AdoptStore(FleetSubnetWasmStoreAdoptionRequest),
    BootstrapStore(RootStoreBootstrapRequest),
    PrepareStoreFixture(RootStoreFixturePrepareRequest),
    InspectCanisterHistory(CanisterInspectionRequest),
    ImportPoolCanister(PoolCanisterRequest),
    MaintainPool,
    ObserveCanister(FleetCanisterObservabilityRequest),
    PrepareAuthoritySnapshot(AuthoritySnapshotRequest),
    PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
    PrepareFleetActivation,
    ProvisionComponent(RootComponentAllocationRequest),
    RemoveSubtree(RootComponentSubtreeRemovalRequest),
    RespondCapability(RootCapabilityEnvelopeV1),
    ResumeAuthoritySnapshot(AuthoritySnapshotRequest),
    ResumeFleetActivation(FleetActivationResumeRequest),
    SynchronizeRegistry(FleetSubnetRootRegistrySyncRequest),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the direct Root wire decoder retains its component registry response inline"
)]
pub enum RootCommandResponseFragment {
    PrepareStoreFixture(Result<FixtureSourceStatus, FixtureStoreError>),
    ImportPoolCanister(PoolImportResponse),
    InspectCanisterHistory(CanisterHistoryResponse),
    MaintainPool(PoolMaintenanceResponse),
    ObserveCanister(CanisterObservabilityResponse),
    OperationAccepted(OperationReceipt),
    PrepareAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
    PrepareComponentRegistry(RootComponentRegistryStatusResponse),
    ResumeAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
    RespondCapability(RootCapabilityResponseV1),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType)]
pub enum RootStatusRequestFragment {
    Admission(PageRequest),
    AuthorityRestore,
    ComponentRegistry(RootComponentRegistryPreparationRequest),
    ComponentRegistryActivePartition(ComponentRegistryActivePartitionRequest),
    ComponentRegistryPartition(ComponentRegistryPartitionRequest),
    CycleBalance,
    CycleHistory(PageRequest),
    FleetAuthority,
    Funding,
    Inventory,
    Operation(OperationStatusRequest),
    Pool(CanisterPoolStatusRequest),
    Metrics(MetricsStatusRequest),
    Runtime,
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the PocketIC decoder mirrors the direct Root status wire"
)]
pub enum RootStatusResponseFragment {
    Admission(FleetAdmissionRootStatusResponse),
    AuthorityRestore(AuthorityRestoreFenceStatusResponse),
    ComponentRegistry(RootComponentRegistryStatusResponse),
    ComponentRegistryActivePartition(ComponentRegistryActivePartitionResponse),
    ComponentRegistryPartition(ComponentRegistryPartitionResponse),
    CycleBalance(CycleBalanceStatusResponse),
    CycleHistory(Page<CycleTrackerEntry>),
    FleetAuthority(FleetSubnetRootAuthority),
    Funding(RootFundingStatusResponse),
    Inventory(FleetSubnetRootCanisterSummary),
    Operation(RootOperationStatusResponse),
    Pool(CanisterPoolResponse),
    Metrics(Page<MetricEntry>),
    Runtime(Box<CanicRuntimeStatus>),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType)]
pub enum ManagedStatusRequestFragment {
    Binding,
    CycleHistory(PageRequest),
    Operation(OperationStatusRequest),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType, Deserialize)]
pub enum ManagedStatusResponseFragment {
    Binding(Box<ManagedCanisterBinding>),
    CycleHistory(Page<CycleTrackerEntry>),
    Operation(Box<ManagedOperationStatusResponseFragment>),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType, Debug, Deserialize)]
pub enum ManagedAdmissionStatusResponseFragment {
    Admission(FleetAdmissionProjectionStatusResponse),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType)]
pub enum PublicMemoryRequest {
    Health,
    Metrics(PublicMetricsRequest),
}

/// Bounded transport selectors used by framework validation.
#[derive(CandidType, Deserialize)]
pub enum PublicMemoryResponse {
    Health(PublicHealth),
    Metrics(PublicMetricsSnapshot),
}
