//! Complete funding census evidence; persisted payment records and reconciliation remain canonical.
use crate::{
    dto::{fleet_snapshot, funding_control},
    values::icp_refill::{IcpRefillErrorCode, IcpRefillStatus, IcpRefillTrigger},
};
use ic_platform::Principal;
/// External unsigned values use Wire's bounded minimal-magnitude convention.
pub const MAGNITUDE_BYTES: usize = 256 * 1024;

/// Complete native IcpRefillResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RefillResult {
    pub operation_id: [u8; 32],
    pub status: IcpRefillStatus,
    pub ledger_block_index: Option<u64>,
    pub cycles_sent: Option<ic_wire::Bytes<MAGNITUDE_BYTES>>,
    pub error_code: Option<IcpRefillErrorCode>,
    pub error_message: Option<String>,
}

/// Complete native RootIcpRefillReleaseEvidence evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RefillEvidence {
    pub transfer_uncertain: bool,
    pub record_id: u64,
    pub trigger: IcpRefillTrigger,
    pub policy_hash: [u8; 32],
    pub source_canister: Principal,
    pub source_subaccount: Option<[u8; 32]>,
    pub target_canister: Principal,
    pub ledger_canister_id: Principal,
    pub cmc_canister_id: Principal,
    pub cmc_to_account_owner: Principal,
    pub cmc_to_account_subaccount: Option<[u8; 32]>,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub budget_window_start_secs: u64,
    pub budget_reserved: bool,
    pub memo: Vec<u8>,
    pub created_at_time_ns: u64,
    pub notify_attempts: u32,
    pub response: RefillResult,
    pub refund_block_index: Option<u64>,
    pub transaction_too_old_min_block_index: Option<u64>,
}

/// Complete native RootFundingReleaseResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReleasePage {
    pub fleet_subnet_root: Principal,
    pub policy_generation: u64,
    pub policy_hash: [u8; 32],
    pub icp_refill_policy: Option<fleet_snapshot::FleetSubnetRootIcpRefillPolicy>,
    pub current_request: Option<funding_control::Request>,
    pub accepted_grant: Option<funding_control::AcceptanceReceipt>,
    pub rotation_current: Option<funding_control::PrepareRequest>,
    pub icp_refills: Vec<RefillEvidence>,
    pub next_after: Option<u64>,
}

/// Complete native RootIcpRefillStatusResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RefillStatus {
    pub trigger: IcpRefillTrigger,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub budget_window_start_secs: u64,
    pub resumable: bool,
    pub response: RefillResult,
}

/// Complete native RootFundingStatusResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootStatus {
    pub fleet_subnet_root: Principal,
    pub lifecycle_status: crate::dto::fleet_registry::FleetSubnetRootStatus,
    pub funding_eligible: bool,
    pub cycles_funding_enabled: bool,
    pub current_cycles: crate::cycles::Cycles,
    pub policy_generation: u64,
    pub funding_profile: crate::ids::FleetFundingProfile,
    pub policy_hash: [u8; 32],
    pub root_policy: crate::ids::FleetSubnetRootFundingPolicy,
    pub current_operation: Option<funding_control::Request>,
    pub last_result: Option<funding_control::Response>,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: crate::cycles::Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: crate::cycles::Cycles,
    pub rotation_current: Option<funding_control::RotationReceipt>,
    pub rotation_last: Option<funding_control::RotationReceipt>,
    pub icp_refill_policy: Option<fleet_snapshot::FleetSubnetRootIcpRefillPolicy>,
    pub icp_window_start_secs: Option<u64>,
    pub icp_window_reserved_e8s: u64,
    pub automatic_icp_refills: u32,
    pub automatic_icp_refill_e8s: u64,
    pub latest_icp_refill: Option<RefillStatus>,
}

/// Complete native CoordinatorRootFundingStatusResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CoordinatorRoot {
    pub fleet_subnet_root: Principal,
    pub lifecycle_status: crate::dto::fleet_registry::FleetSubnetRootStatus,
    pub policy_hash: [u8; 32],
    pub policy: crate::ids::FleetSubnetRootFundingPolicy,
    pub window: crate::dto::fleet_coordinator::CoordinatorFundingWindowStatusResponse,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: crate::cycles::Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: crate::cycles::Cycles,
    pub last_successful_grant_at_ns: Option<u64>,
    pub current_operation: Option<funding_control::Request>,
    pub last_result: Option<funding_control::Response>,
}

/// Complete native CoordinatorFundingStatusResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CoordinatorStatus {
    pub coordinator: Principal,
    pub current_cycles: crate::cycles::Cycles,
    pub policy_generation: u64,
    pub funding_enabled: bool,
    pub funding_profile: Option<crate::ids::FleetFundingProfile>,
    pub policy: Option<crate::ids::FleetCoordinatorRootFundingPolicy>,
    pub fleet_window: Option<crate::dto::fleet_coordinator::CoordinatorFundingWindowStatusResponse>,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: crate::cycles::Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: crate::cycles::Cycles,
    pub rotation_checkpoint_count: u32,
    pub rotation_checkpoint_root_count: u32,
    pub rotation_checkpoint_root_capacity_remaining: u32,
    pub rotation: Option<RotationStatus>,
    pub roots: Vec<CoordinatorRoot>,
}

/// Complete native FleetFundingPolicyRotationStatusResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RotationStatus {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub phase: RotationPhase,
}

/// Complete native FleetFundingPolicyRotationReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RotationCompletion {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_registry: crate::dto::registry_discovery::RegistryVersion,
    pub successor_registry: crate::dto::registry_discovery::RegistryVersion,
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub affected_root_count: u32,
    pub retained_historical_automatic_grants: u64,
    pub retained_historical_automatic_cycles: crate::cycles::Cycles,
    pub successor_policy_set_hash: [u8; 32],
    pub maximum_new_automatic_cycles: crate::cycles::Cycles,
    pub apply_operator_debit: crate::cycles::Cycles,
    pub completed_at_ns: u64,
}

/// Complete durable rotation phase, including terminal authority receipts.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum RotationPhase {
    Staging {
        staged_root_count: u32,
        expected_root_count: u32,
    },
    PreparingRoots {
        prepared_root_count: u32,
        expected_root_count: u32,
    },
    ActivatingRoots {
        activated_root_count: u32,
        expected_root_count: u32,
        successor_registry: Box<crate::dto::registry_discovery::RegistryVersion>,
    },
    Completed(Box<RotationCompletion>),
}
