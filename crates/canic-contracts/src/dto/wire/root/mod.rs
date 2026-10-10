#![expect(
    clippy::large_enum_variant,
    reason = "complete canonical role unions retain the existing inline wire payloads"
)]

//! Module: dto::wire::root
//!
//! Responsibility: own the root wire union declarations.
//! Does not own: endpoint dispatch, authority, state, or capability resolution.
//! Boundary: ordinary consumers use the complete types; endpoint emitters select
//! the same declarations with the build-resolved capability attributes.

// sort-derives-disable-start: preserve macro hygiene in derive paths.
/// Emit the root contract with an explicit capability selection.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_root_wire_types {
    (
        serde_crate = $serde_crate:literal,
        metrics_attributes = [$($metrics_attributes:tt)*],
        history_attributes = [$($history_attributes:tt)*],
        logs_attributes = [$($logs_attributes:tt)*],
        diagnostics_attributes = [$($diagnostics_attributes:tt)*],
        role_attestation_attributes = [$($role_attestation_attributes:tt)*],
        root_delegation_attributes = [$($root_delegation_attributes:tt)*],
    ) => {
        #[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// PublicStatusRequest wire union selected from the role's compiled capabilities.
        pub enum PublicStatusRequest {
            Health,
            $($metrics_attributes)*
            Metrics($crate::dto::public_status::PublicMetricsRequest),
            $($history_attributes)*
            History($crate::dto::public_status::PublicHistoryRequest),
            Overview,
            Children($crate::dto::page::PageRequest),
            ComponentDirectoryPage($crate::dto::component_registry::ComponentDirectoryPageRequest),
        }

#[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// PublicStatusResponse wire union selected from the role's compiled capabilities.
        pub enum PublicStatusResponse {
            Health($crate::dto::public_status::PublicHealth),
            $($metrics_attributes)*
            Metrics($crate::dto::public_status::PublicMetricsSnapshot),
            $($history_attributes)*
            History($crate::dto::public_status::PublicHistorySnapshot),
            Overview($crate::dto::role::RoleOverviewResponse),
            Children($crate::dto::page::Page<$crate::dto::canister::CanisterInfo>),
            ComponentDirectoryPage($crate::dto::component_registry::ComponentDirectoryPageResponse),
        }

#[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// ObservabilityRequest wire union selected from the role's compiled capabilities.
        pub enum ObservabilityRequest {
            ChildFunding($crate::__reexports::candid::Principal),
            CycleBalance,
            $($history_attributes)*
            CycleHistory($crate::dto::page::PageRequest),
            Health,
            InspectionReserve($crate::dto::canister::CanisterInspectionRequest),
            $($logs_attributes)*
            Logs($crate::dto::role::LogStatusRequest),
            $($diagnostics_attributes)*
            MemoryAllocations,
            $($metrics_attributes)*
            Metrics($crate::dto::role::MetricsStatusRequest),
            Readiness,
            $($diagnostics_attributes)*
            Runtime,
        }

#[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// ObservabilityResponse wire union selected from the role's compiled capabilities.
        pub enum ObservabilityResponse {
            ChildFunding($crate::dto::observability::ChildFundingUsage),
            CycleBalance($crate::dto::role::CycleBalanceStatusResponse),
            $($history_attributes)*
            CycleHistory($crate::dto::page::Page<$crate::dto::cycles::CycleTrackerEntry>),
            Health($crate::dto::runtime::CanicHealthStatus),
            InspectionReserve($crate::dto::canister::CanisterInspectionReserveResponse),
            $($logs_attributes)*
            Logs($crate::dto::page::Page<$crate::dto::log::LogEntry>),
            $($diagnostics_attributes)*
            MemoryAllocations($crate::dto::memory::MemoryAllocationsResponse),
            $($metrics_attributes)*
            Metrics($crate::dto::page::Page<$crate::dto::metrics::MetricEntry>),
            Readiness($crate::dto::runtime::CanicReadinessStatus),
            $($diagnostics_attributes)*
            Runtime($crate::dto::runtime::CanicRuntimeStatus),
        }

$($role_attestation_attributes)*
        #[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// RootAuthStatusRequest wire union selected from the role's compiled capabilities.
        pub enum RootAuthStatusRequest {
            $($role_attestation_attributes)*
            RoleAttestation($crate::dto::auth::RoleAttestationGetRequest),
        }

$($role_attestation_attributes)*
        #[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// RootAuthStatusResponse wire union selected from the role's compiled capabilities.
        pub enum RootAuthStatusResponse {
            $($role_attestation_attributes)*
            RoleAttestation($crate::dto::auth::SignedRoleAttestation),
        }

#[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// RootOperationStatusRequest wire union selected from the role's compiled capabilities.
        pub enum RootOperationStatusRequest {
            ComponentChildProvisioning($crate::dto::role::OperationStatusRequest),
            ComponentProvisioning($crate::dto::role::OperationStatusRequest),
            Operation($crate::dto::role::OperationStatusRequest),
        }

#[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// RootOperationStatusResponse wire union selected from the role's compiled capabilities.
        pub enum RootOperationStatusResponse {
            ComponentChildProvisioning(
                $crate::dto::component_registry::RootComponentChildAllocationResponse,
            ),
            ComponentProvisioning(
                $crate::dto::component_provisioning::RootComponentProvisioningStatusResponse,
            ),
            Operation($crate::dto::root::RootOperationStatusResponse),
        }

#[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// RootStatusRequest wire union selected from the role's compiled capabilities.
        pub enum RootStatusRequest {
            Admission($crate::dto::page::PageRequest),
            AuthorityRestore,
            ComponentDirectoryHead($crate::dto::component_registry::ComponentDirectoryHeadRequest),
            ComponentDirectoryPage($crate::dto::component_registry::ComponentDirectoryPageRequest),
            ComponentRegistry($crate::dto::component_registry::RootComponentRegistryPreparationRequest),
            ComponentRegistryActivePartition(
                $crate::dto::component_registry::ComponentRegistryActivePartitionRequest,
            ),
            ComponentRegistryPartition($crate::dto::component_registry::ComponentRegistryPartitionRequest),
            Config,
            FleetAuthority,
            FleetState,
            Funding,
            FundingRelease(Option<u64>),
            Inventory,
            $($root_delegation_attributes)*
            IssuerRenewal($crate::dto::auth::RootIssuerRenewalStatusRequest),
            Pool($crate::dto::pool::CanisterPoolStatusRequest),
            PoolImport($crate::dto::pool_import::PoolImportIdentity),
            PoolImportContext,
            PoolRelease,
            ProvisioningRelease(Option<$crate::dto::root::RootProvisioningReleaseKey>),
            IntentRelease(Option<$crate::dto::release_intents::IntentReleaseKey>),
            ReplayRelease(Option<[u8; 32]>),
            StoreOverview,
        }

#[derive($crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// RootStatusResponse wire union selected from the role's compiled capabilities.
        pub enum RootStatusResponse {
            Admission($crate::dto::fleet_admission::FleetAdmissionRootStatusResponse),
            AuthorityRestore($crate::dto::authority_restore::AuthorityRestoreFenceStatusResponse),
            ComponentDirectoryHead($crate::dto::component_registry::ComponentDirectoryHead),
            ComponentDirectoryPage($crate::dto::component_registry::ComponentDirectoryPageResponse),
            ComponentRegistry($crate::dto::component_registry::RootComponentRegistryStatusResponse),
            ComponentRegistryActivePartition(
                $crate::dto::component_registry::ComponentRegistryActivePartitionResponse,
            ),
            ComponentRegistryPartition(
                $crate::dto::component_registry::ComponentRegistryPartitionResponse,
            ),
            Config($crate::dto::role::ConfigStatusResponse),
            FleetAuthority($crate::dto::fleet_subnet_root::FleetSubnetRootAuthority),
            FleetState($crate::dto::state::FleetStateResponse),
            Funding($crate::dto::root::RootFundingStatusResponse),
            FundingRelease($crate::dto::root::RootFundingReleaseResponse),
            Inventory($crate::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary),
            $($root_delegation_attributes)*
            IssuerRenewal($crate::dto::auth::RootIssuerRenewalStatusResponse),
            Pool($crate::dto::pool::CanisterPoolResponse),
            PoolImport($crate::dto::pool_import::PoolImportStatus),
            PoolImportContext($crate::dto::pool_import::PoolImportContext),
            PoolRelease($crate::dto::root::RootPoolReleaseResponse),
            ProvisioningRelease($crate::dto::root::RootProvisioningReleaseResponse),
            IntentRelease($crate::dto::release_intents::IntentReleaseResponse),
            ReplayRelease($crate::dto::release_receipts::ReplayReleaseResponse),
            StoreOverview($crate::dto::template::WasmStoreOverviewResponse),
        }
    };
}

// sort-derives-disable-end

crate::__canic_root_wire_types! {
    serde_crate = "serde",
    metrics_attributes = [],
    history_attributes = [],
    logs_attributes = [],
    diagnostics_attributes = [],
    role_attestation_attributes = [],
    root_delegation_attributes = [],
}
