#![expect(
    clippy::large_enum_variant,
    reason = "complete canonical role unions retain the existing inline wire payloads"
)]

//! Module: dto::wire::managed
//!
//! Responsibility: own the managed wire union declarations.
//! Does not own: endpoint dispatch, authority, state, or capability resolution.
//! Boundary: ordinary consumers use the complete types; endpoint emitters select
//! the same declarations with the build-resolved capability attributes.

/// Emit the managed contract with an explicit capability selection.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_managed_wire_types {
    (
        serde_crate = $serde_crate:literal,
        metrics_attributes = [$($metrics_attributes:tt)*],
        history_attributes = [$($history_attributes:tt)*],
        child_provisioning_attributes = [$($child_provisioning_attributes:tt)*],
        automatic_topup_attributes = [$($automatic_topup_attributes:tt)*],
        logs_attributes = [$($logs_attributes:tt)*],
        diagnostics_attributes = [$($diagnostics_attributes:tt)*],
        local_auth_attributes = [$($local_auth_attributes:tt)*],
        token_issuer_attributes = [$($token_issuer_attributes:tt)*],
        application_authorization_attributes = [$($application_authorization_attributes:tt)*],
        caller_authority_attributes = [$($caller_authority_attributes:tt)*],
        fleet_admission_attributes = [$($fleet_admission_attributes:tt)*],
    ) => {
        #[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// CanisterOperationStatusResponse wire union selected from the role's compiled capabilities.
        pub enum CanisterOperationStatusResponse {
            ConfigureRuntime($crate::dto::role::ComponentRuntimeOperationStatus),
        }

#[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// PublicStatusRequest wire union selected from the role's compiled capabilities.
        pub enum PublicStatusRequest {
            Health,
            $($metrics_attributes)*
            Metrics($crate::dto::public_status::PublicMetricsRequest),
            $($history_attributes)*
            History($crate::dto::public_status::PublicHistoryRequest),
            Overview,
            $($child_provisioning_attributes)*
            Children($crate::dto::page::PageRequest),
        }

#[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// PublicStatusResponse wire union selected from the role's compiled capabilities.
        pub enum PublicStatusResponse {
            Health($crate::dto::public_status::PublicHealth),
            $($metrics_attributes)*
            Metrics($crate::dto::public_status::PublicMetricsSnapshot),
            $($history_attributes)*
            History($crate::dto::public_status::PublicHistorySnapshot),
            Overview($crate::dto::role::RoleOverviewResponse),
            $($child_provisioning_attributes)*
            Children($crate::dto::page::Page<$crate::dto::canister::CanisterInfo>),
        }

#[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// ObservabilityRequest wire union selected from the role's compiled capabilities.
        pub enum ObservabilityRequest {
            Binding,
            ChildFunding($crate::__reexports::candid::Principal),
            CycleBalance,
            $($history_attributes)*
            CycleHistory($crate::dto::page::PageRequest),
            $($automatic_topup_attributes)*
            $($history_attributes)*
            CycleTopups($crate::dto::page::PageRequest),
            Health,
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

#[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// ObservabilityResponse wire union selected from the role's compiled capabilities.
        pub enum ObservabilityResponse {
            Binding($crate::ids::ManagedCanisterBinding),
            ChildFunding($crate::dto::observability::ChildFundingUsage),
            CycleBalance($crate::dto::role::CycleBalanceStatusResponse),
            $($history_attributes)*
            CycleHistory($crate::dto::page::Page<$crate::dto::cycles::CycleTrackerEntry>),
            $($automatic_topup_attributes)*
            $($history_attributes)*
            CycleTopups($crate::dto::page::Page<$crate::dto::cycles::CycleTopupEvent>),
            Health($crate::dto::runtime::CanicHealthStatus),
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

$($local_auth_attributes)*
        #[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// AuthStatusRequest wire union selected from the role's compiled capabilities.
        pub enum AuthStatusRequest {
            $($token_issuer_attributes)*
            ActiveDelegationProof,
            $($application_authorization_attributes)*
            ApplicationSession,
            $($token_issuer_attributes)*
            DelegatedToken($crate::dto::auth::DelegatedTokenGetRequest),
        }

$($local_auth_attributes)*
        #[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// AuthStatusResponse wire union selected from the role's compiled capabilities.
        pub enum AuthStatusResponse {
            $($token_issuer_attributes)*
            ActiveDelegationProof($crate::dto::auth::ActiveDelegationProofStatusResponse),
            $($application_authorization_attributes)*
            ApplicationSession($crate::dto::auth::ApplicationSessionStatus),
            $($token_issuer_attributes)*
            DelegatedToken($crate::dto::auth::DelegatedToken),
        }

#[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// ControlStatusRequest wire union selected from the role's compiled capabilities.
        pub enum ControlStatusRequest {
            $($application_authorization_attributes)*
            ApplicationSessionAudit($crate::dto::page::PageRequest),
            $($caller_authority_attributes)*
            CallerAuthority($crate::dto::role::OperationStatusRequest),
            Operation($crate::dto::role::OperationStatusRequest),
        }

#[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// ControlStatusResponse wire union selected from the role's compiled capabilities.
        pub enum ControlStatusResponse {
            $($application_authorization_attributes)*
            ApplicationSessionAudit($crate::dto::auth::ApplicationSessionAuditResponse),
            $($caller_authority_attributes)*
            CallerAuthority($crate::dto::caller_authority::CallerAuthorityStatus),
            Operation(CanisterOperationStatusResponse),
        }

$($fleet_admission_attributes)*
        #[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// AdmissionStatusRequest wire union selected from the role's compiled capabilities.
        pub enum AdmissionStatusRequest {
            $($fleet_admission_attributes)*
            Admission($crate::dto::page::PageRequest),
        }

$($fleet_admission_attributes)*
        #[derive(::__reexports::candid::CandidType, ::__reexports::serde::Deserialize)]
        #[serde(crate = $serde_crate)]
        /// AdmissionStatusResponse wire union selected from the role's compiled capabilities.
        pub enum AdmissionStatusResponse {
            $($fleet_admission_attributes)*
            Admission($crate::dto::fleet_admission::FleetAdmissionProjectionStatusResponse),
        }
    };
}

crate::__canic_managed_wire_types! {
    serde_crate = "serde",
    metrics_attributes = [],
    history_attributes = [],
    child_provisioning_attributes = [],
    automatic_topup_attributes = [],
    logs_attributes = [],
    diagnostics_attributes = [],
    local_auth_attributes = [],
    token_issuer_attributes = [],
    application_authorization_attributes = [],
    caller_authority_attributes = [],
    fleet_admission_attributes = [],
}
