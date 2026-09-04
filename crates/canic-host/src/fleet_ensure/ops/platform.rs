//! Module: fleet_ensure::ops::platform
//!
//! Responsibility: mechanically observe and mutate the exact current Fleet through ICP CLI.
//! Does not own: effect ordering, durable intent, retry policy, or plan approval.
//! Boundary: the workflow calls one method only after persisting its exact action identity.

use crate::{
    canister_protocol::{CanisterProtocolError, call_with_candid, query_with_candid},
    fleet_ensure::{
        model::{
            CanisterRuntimeStatus, DesiredCanisterKind, DesiredFleet, EffectRecord, EnsureAction,
            EstateFundingDomainObservation, EstatePoolAssetLifecycle, EstatePoolAssetObservation,
            EstatePoolAssetOrigin, EstatePoolCreationDiagnostic,
            EstatePoolCreationReceiptObservation, EstatePoolInventoryObservation,
            EstatePoolPendingCreationObservation, FleetEnsureStateRecord, FleetObservation,
            InstallMode, LiveCanister, MAX_FLEET_ENSURE_CANISTERS, RetirementTransferBalances,
            RetirementTransferInvariantError, RetirementTransferReconciliation,
            RootManagementCanisterObservation, RootManagementObservation,
            RootOwnedCanisterLifecycle, create_balance_is_terminal, reconcile_retirement_transfer,
        },
        ops::{
            EffectObservation, EffectOutcome, EffectRetry, EnsurePaths, EnsurePlatform,
            EnsureStateError, TerminalFleetInventory, canic_init, current_protocol,
            predecessor_root_status, protocol, read_root_start_authority, root_owned_lifecycle,
            verify_root_start_release_authority,
        },
    },
    icp::{
        IcpCandidCallError, IcpCanisterStatusReport, IcpCli, IcpCommandError, IcpDiagnostic,
        IcpManagementCallError, LocalReplicaTarget, run_status,
    },
    icp_config::resolve_icp_build_network_from_root,
    subnet_catalog::load_mainnet_subnet_catalog,
};
use candid::{CandidType, Nat, Principal};
use canic_core::{
    cdk::{types::Cycles, utils::hash::hex_bytes},
    dto::canister::{CanisterInspectionRequest, CanisterStatusResponse},
    dto::pool::{
        CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus,
        CanisterPoolCreationFailure, CanisterPoolCreationProgress, CanisterPoolHandoff,
        CanisterPoolResponse, CanisterPoolStatusRequest,
    },
    ids::BuildNetwork,
    protocol as canic_protocol,
};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

#[derive(CandidType)]
struct ManagementCanisterStatusRequest {
    canister_id: Principal,
}

#[derive(CandidType)]
struct CyclesLedgerAccount {
    owner: Principal,
    subaccount: Option<[u8; 32]>,
}

#[derive(CandidType)]
struct CyclesLedgerTransferArgs {
    amount: Nat,
    created_at_time: Option<u64>,
    fee: Option<Nat>,
    from_subaccount: Option<[u8; 32]>,
    memo: Option<Vec<u8>>,
    to: CyclesLedgerAccount,
}

#[derive(CandidType, Deserialize)]
enum CyclesLedgerTransferError {
    BadBurn { min_burn_amount: Nat },
    BadFee { expected_fee: Nat },
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    GenericError { error_code: Nat, message: String },
    InsufficientFunds { balance: Nat },
    TemporarilyUnavailable,
    TooOld,
}

#[derive(CandidType, Deserialize)]
struct ManagementCanisterStatusResponse {
    version: u64,
    module_hash: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize)]
struct ManagementCanisterObservationSettings {
    controllers: Vec<Principal>,
}

#[derive(CandidType, Deserialize)]
enum ManagementCanisterRuntimeStatus {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
}

#[derive(CandidType, Deserialize)]
struct ManagementCanisterObservationResponse {
    status: ManagementCanisterRuntimeStatus,
    settings: ManagementCanisterObservationSettings,
    module_hash: Option<Vec<u8>>,
    cycles: Nat,
    version: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct ExactInstallCanisterStatus {
    canister_version: u64,
    module_sha256: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
struct RetainedStoreControlBinding {
    action_root: String,
    retained_root: Option<String>,
    retained_store: Option<String>,
    root_kind: Option<DesiredCanisterKind>,
    root_parent: Option<String>,
    store_kind: Option<DesiredCanisterKind>,
    store_parent: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
struct RetainedStoreControlLiveBinding {
    root_controllers: Vec<String>,
    root_module_sha256: Option<String>,
    store_controllers: Vec<String>,
    store_module_sha256: Option<String>,
}

fn retained_store_control_binding(
    action_root: &str,
    state: &FleetEnsureStateRecord,
    root_name: &str,
    store_name: &str,
) -> RetainedStoreControlBinding {
    let root = state.topology.get(root_name);
    let store = state.topology.get(store_name);
    RetainedStoreControlBinding {
        action_root: action_root.to_string(),
        retained_root: state.principals.get(root_name).cloned(),
        retained_store: state.principals.get(store_name).cloned(),
        root_kind: root.map(|topology| topology.kind),
        root_parent: root.and_then(|topology| topology.parent.clone()),
        store_kind: store.map(|topology| topology.kind),
        store_parent: store.and_then(|topology| topology.parent.clone()),
    }
}

fn retained_store_control_live_binding(
    root: LiveCanister,
    store: LiveCanister,
) -> RetainedStoreControlLiveBinding {
    RetainedStoreControlLiveBinding {
        root_controllers: root.controllers,
        root_module_sha256: root.module_sha256,
        store_controllers: store.controllers,
        store_module_sha256: store.module_sha256,
    }
}

fn retained_predecessor_module_matches(
    retained_module_sha256: Option<&str>,
    observed_module_sha256: Option<&str>,
    reviewed_successor_sha256: &str,
) -> bool {
    let Some(observed_module_sha256) = observed_module_sha256 else {
        return false;
    };
    observed_module_sha256 != reviewed_successor_sha256
        && retained_module_sha256.is_none_or(|retained| retained == observed_module_sha256)
}

fn retained_store_control_live_is_exact(
    observed: &RetainedStoreControlLiveBinding,
    root_controllers: &[String],
    store_controllers: &[String],
    retained_root_module_sha256: Option<&str>,
    retained_store_module_sha256: Option<&str>,
    root_successor_sha256: &str,
    store_successor_sha256: &str,
) -> bool {
    let controllers_match = observed.root_controllers == root_controllers
        && observed.store_controllers == store_controllers;
    let root_module_matches = retained_predecessor_module_matches(
        retained_root_module_sha256,
        observed.root_module_sha256.as_deref(),
        root_successor_sha256,
    );
    let store_module_matches = retained_predecessor_module_matches(
        retained_store_module_sha256,
        observed.store_module_sha256.as_deref(),
        store_successor_sha256,
    );
    controllers_match && root_module_matches && store_module_matches
}

fn desired_root_by_principal<'a>(
    desired: &'a DesiredFleet,
    state: &FleetEnsureStateRecord,
    principal: &str,
) -> Option<&'a crate::fleet_ensure::model::DesiredCanister> {
    desired.canisters.iter().find(|canister| {
        canister.kind == DesiredCanisterKind::Root
            && state.principals.get(&canister.name).map(String::as_str) == Some(principal)
    })
}

fn is_unallocated_fresh_root(
    desired: &DesiredFleet,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    reviewed_targets: &BTreeSet<String>,
) -> bool {
    reviewed_targets.is_empty()
        && configured.principal.is_none()
        && desired.bootstrap.as_ref().is_some_and(|bootstrap| {
            bootstrap.fresh_estate
                && bootstrap
                    .roots
                    .iter()
                    .any(|root| root.root == configured.name)
        })
}

/// Exact input expected by a configured cycle-safe retirement endpoint.

#[derive(CandidType)]
struct DrainRequest {
    operation_id: String,
    destination: Principal,
    cycles: Nat,
}

#[derive(CandidType, Deserialize)]
enum DrainResponse {
    Accepted { transferred_cycles: Nat },
    Replayed { transferred_cycles: Nat },
}

#[derive(CandidType)]
enum RootPoolStatusRequest {
    Pool(CanisterPoolStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootPoolStatusResponse {
    Pool(Box<CanisterPoolResponse>),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct EstatePoolLifecycleCounts {
    claimed: u32,
    failed: u32,
    handing_off: u32,
    pending_reset: u32,
    ready: u32,
    recycling: u32,
    store: u32,
    store_deletion_pending: u32,
    tracked: u32,
    workload: u32,
}

impl EstatePoolLifecycleCounts {
    const fn declared(response: &CanisterPoolResponse) -> Self {
        Self {
            claimed: response.claimed,
            failed: response.failed,
            handing_off: response.handing_off,
            pending_reset: response.pending_reset,
            ready: response.ready,
            recycling: response.recycling,
            store: response.store,
            store_deletion_pending: response.store_deletion_pending,
            tracked: response.tracked,
            workload: response.workload,
        }
    }

    fn observe(&mut self, status: &CanisterPoolAssetStatus) -> Option<()> {
        self.tracked = self.tracked.checked_add(1)?;
        let count = match status {
            CanisterPoolAssetStatus::Store => &mut self.store,
            CanisterPoolAssetStatus::StoreDeletionPending { .. } => {
                &mut self.store_deletion_pending
            }
            CanisterPoolAssetStatus::PendingReset => &mut self.pending_reset,
            CanisterPoolAssetStatus::Ready => &mut self.ready,
            CanisterPoolAssetStatus::Claimed { .. } => &mut self.claimed,
            CanisterPoolAssetStatus::Workload { .. } => &mut self.workload,
            CanisterPoolAssetStatus::Recycling { .. } => &mut self.recycling,
            CanisterPoolAssetStatus::HandingOff { .. } => &mut self.handing_off,
            CanisterPoolAssetStatus::Failed { .. } => &mut self.failed,
        };
        *count = count.checked_add(1)?;
        Some(())
    }

    fn pooled(self) -> Option<u32> {
        self.pending_reset
            .checked_add(self.ready)?
            .checked_add(self.handing_off)?
            .checked_add(self.failed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EstatePoolPageAuthority {
    completed_handoffs: u64,
    counts: EstatePoolLifecycleCounts,
    pending_handoff: Option<CanisterPoolHandoff>,
    pooled: u32,
    surplus: u32,
}

impl EstatePoolPageAuthority {
    fn from_response(response: &CanisterPoolResponse) -> Self {
        Self {
            completed_handoffs: response.completed_handoffs,
            counts: EstatePoolLifecycleCounts::declared(response),
            pending_handoff: response.pending_handoff.clone(),
            pooled: response.pooled,
            surplus: response.surplus,
        }
    }

    fn matches_complete_inventory(
        &self,
        observed: EstatePoolLifecycleCounts,
        maximum_size: u32,
    ) -> bool {
        let Some(pooled) = observed.pooled() else {
            return false;
        };
        self.counts == observed
            && self.pooled == pooled
            && self.surplus == pooled.saturating_sub(maximum_size)
    }
}

#[derive(Default)]
struct EstatePoolInventoryAccumulator {
    assets: Vec<EstatePoolAssetObservation>,
    expected_config: Option<canic_core::ids::FleetSubnetCanisterPoolConfig>,
    expected_page_authority: Option<EstatePoolPageAuthority>,
    expected_pending: Option<canic_core::dto::pool::CanisterPoolCreation>,
    observed_counts: EstatePoolLifecycleCounts,
    seen: BTreeSet<Principal>,
}

impl EstatePoolInventoryAccumulator {
    fn observe_page(
        &mut self,
        root_name: &str,
        page: CanisterPoolResponse,
    ) -> Result<Option<Principal>, IcpEnsurePlatformError> {
        let page_authority = EstatePoolPageAuthority::from_response(&page);
        let first_page = self.expected_config.is_none();
        if !first_page
            && (self.expected_config.as_ref() != Some(&page.config)
                || self.expected_pending != page.pending_creation
                || self.expected_page_authority.as_ref() != Some(&page_authority))
        {
            return Err(pool_configuration_error(format!(
                "Root {root_name} pool authority changed during pagination"
            )));
        }
        if first_page {
            self.expected_config = Some(page.config.clone());
            self.expected_pending.clone_from(&page.pending_creation);
            self.expected_page_authority = Some(page_authority);
        }

        for asset in page.entries {
            if !self.seen.insert(asset.canister_id) {
                return Err(pool_configuration_error(format!(
                    "Root {root_name} pool repeats canister {}",
                    asset.canister_id
                )));
            }
            if self.seen.len() > MAX_FLEET_ENSURE_CANISTERS
                || self.observed_counts.observe(&asset.status).is_none()
            {
                return Err(pool_configuration_error(format!(
                    "Root {root_name} pool exceeds the Fleet observation bound"
                )));
            }
            if let Some(lifecycle) = estate_pool_lifecycle(&asset.status) {
                self.assets.push(EstatePoolAssetObservation {
                    creation_receipt: asset.creation_receipt.map(|receipt| {
                        EstatePoolCreationReceiptObservation {
                            block_index: receipt.block_index,
                            operation_id: hex_bytes(receipt.operation_id),
                            cycles_ledger: receipt.cycles_ledger.to_text(),
                            ledger_amount_cycles: receipt.ledger_amount.to_u128(),
                            ledger_fee_cycles: receipt.ledger_fee.to_u128(),
                            readiness_floor_cycles: receipt.readiness_floor.to_u128(),
                            creation_execution_margin_cycles: receipt
                                .creation_execution_margin
                                .to_u128(),
                            management_creation_fee_cycles: receipt
                                .management_creation_fee
                                .to_u128(),
                            first_observed_cycles: receipt
                                .first_observed_cycles
                                .map(|cycles| cycles.to_u128()),
                        }
                    }),
                    cycles: asset.cycles.to_u128(),
                    lifecycle,
                    origin: match asset.origin {
                        CanisterPoolAssetOrigin::InfrastructureStore => {
                            EstatePoolAssetOrigin::InfrastructureStore
                        }
                        CanisterPoolAssetOrigin::Created => EstatePoolAssetOrigin::Created,
                        CanisterPoolAssetOrigin::Imported => EstatePoolAssetOrigin::Imported,
                        CanisterPoolAssetOrigin::Recycled => EstatePoolAssetOrigin::Recycled,
                    },
                    principal: asset.canister_id.to_text(),
                });
            }
        }
        Ok(page.next_start_after)
    }

    fn finish(
        self,
        root_name: &str,
    ) -> Result<EstatePoolInventoryObservation, IcpEnsurePlatformError> {
        let config = self.expected_config.ok_or_else(|| {
            pool_configuration_error(format!("Root {root_name} returned no pool authority"))
        })?;
        if !self.expected_page_authority.is_some_and(|authority| {
            authority.matches_complete_inventory(self.observed_counts, config.maximum_size)
        }) {
            return Err(pool_configuration_error(format!(
                "Root {root_name} pool lifecycle totals differ from its complete paged inventory"
            )));
        }
        Ok(EstatePoolInventoryObservation {
            assets: self.assets,
            maximum_size: config.maximum_size,
            minimum_size: config.minimum_size,
            pending_creation: self.expected_pending.map(estate_pool_pending_creation),
            readiness_floor_cycles: config.canister_cycles.to_u128(),
            creation_execution_margin_cycles: config.creation_execution_margin.to_u128(),
        })
    }
}

fn pool_configuration_error(reason: String) -> IcpEnsurePlatformError {
    current_protocol::CurrentProtocolError::Configuration(reason).into()
}

const fn pool_policy_is_current(
    observed: &EstatePoolInventoryObservation,
    desired: &canic_core::ids::FleetSubnetCanisterPoolConfig,
) -> bool {
    observed.maximum_size == desired.maximum_size
        && observed.minimum_size == desired.minimum_size
        && observed.readiness_floor_cycles == desired.canister_cycles.to_u128()
        && observed.creation_execution_margin_cycles == desired.creation_execution_margin.to_u128()
}

fn estate_pool_pending_creation(
    creation: canic_core::dto::pool::CanisterPoolCreation,
) -> EstatePoolPendingCreationObservation {
    let (
        available_cycles,
        created_principal,
        diagnostic,
        required_cycles,
        retry_at_ns,
        shortfall_cycles,
        uncertain_result,
    ) = match creation.progress {
        CanisterPoolCreationProgress::Intent { uncertain_result } => {
            (None, None, None, None, None, None, uncertain_result)
        }
        CanisterPoolCreationProgress::Created { canister_id, .. } => (
            None,
            Some(canister_id.to_text()),
            None,
            None,
            None,
            None,
            false,
        ),
        CanisterPoolCreationProgress::WaitingForFunding {
            available,
            required,
            retry_at_ns,
            shortfall,
            ..
        } => (
            Some(available.to_u128()),
            None,
            Some(EstatePoolCreationDiagnostic::FundingRequired),
            Some(required.to_u128()),
            Some(retry_at_ns),
            Some(shortfall.to_u128()),
            false,
        ),
        CanisterPoolCreationProgress::Blocked { failure } => (
            None,
            None,
            Some(match failure {
                CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow => {
                    EstatePoolCreationDiagnostic::UnresolvedAfterLedgerWindow
                }
                CanisterPoolCreationFailure::LedgerCreationFailed => {
                    EstatePoolCreationDiagnostic::LedgerCreationFailed
                }
                CanisterPoolCreationFailure::LedgerRejected => {
                    EstatePoolCreationDiagnostic::LedgerRejected
                }
            }),
            None,
            None,
            None,
            false,
        ),
    };
    EstatePoolPendingCreationObservation {
        attempt_count: creation.attempt_count,
        available_cycles,
        creation_amount_cycles: creation.ledger_amount.to_u128(),
        created_principal,
        diagnostic,
        last_attempt_at_ns: creation.last_attempt_at_ns,
        operation_id: hex_bytes(creation.operation_id),
        required_cycles,
        retry_at_ns,
        shortfall_cycles,
        uncertain_result,
    }
}

const fn estate_pool_lifecycle(
    status: &CanisterPoolAssetStatus,
) -> Option<EstatePoolAssetLifecycle> {
    match status {
        CanisterPoolAssetStatus::Store | CanisterPoolAssetStatus::StoreDeletionPending { .. } => {
            None
        }
        CanisterPoolAssetStatus::PendingReset => Some(EstatePoolAssetLifecycle::PendingReset),
        CanisterPoolAssetStatus::Ready => Some(EstatePoolAssetLifecycle::Ready),
        CanisterPoolAssetStatus::Claimed { .. } => Some(EstatePoolAssetLifecycle::Claimed),
        CanisterPoolAssetStatus::Workload { .. } => Some(EstatePoolAssetLifecycle::Workload),
        CanisterPoolAssetStatus::Recycling { .. } => Some(EstatePoolAssetLifecycle::Recycling),
        CanisterPoolAssetStatus::HandingOff { .. } => Some(EstatePoolAssetLifecycle::HandingOff),
        CanisterPoolAssetStatus::Failed { .. } => Some(EstatePoolAssetLifecycle::Failed),
    }
}

#[derive(CandidType)]
enum RootInspectionCommand {
    InspectCanister(CanisterInspectionRequest),
}

#[derive(CandidType, Deserialize)]
enum RootInspectionResponse {
    InspectCanister(CanisterStatusResponse),
}

#[derive(CandidType)]
enum ManagedCanisterStatusRequest {
    CycleBalance,
}

#[derive(CandidType, Deserialize)]
enum ManagedCanisterStatusResponse {
    CycleBalance(canic_core::dto::role::CycleBalanceStatusResponse),
}

#[derive(CandidType)]
struct CreateCanisterArgs {
    amount: Nat,
    created_at_time: Option<u64>,
    creation_args: Option<CmcCreateCanisterArgs>,
    from_subaccount: Option<[u8; 32]>,
}

#[derive(CandidType)]
struct CmcCreateCanisterArgs {
    settings: Option<CanisterSettings>,
    subnet_selection: Option<SubnetSelection>,
}

#[derive(CandidType)]
struct CanisterSettings {
    compute_allocation: Option<Nat>,
    controllers: Option<Vec<Principal>>,
    freezing_threshold: Option<Nat>,
    memory_allocation: Option<Nat>,
    reserved_cycles_limit: Option<Nat>,
}

struct CreateCanisterAuthority<'a> {
    controller_canisters: &'a [String],
    controllers: &'a [String],
    created_at_time: u64,
    ledger: &'a str,
    requested_initial_cycles: u128,
    subnet: &'a str,
}

#[derive(CandidType)]
enum SubnetSelection {
    Subnet { subnet: Principal },
}

#[derive(CandidType, Deserialize)]
struct CreateCanisterSuccess {
    block_id: Nat,
    canister_id: Principal,
}

#[derive(CandidType, Deserialize)]
enum CreateCanisterError {
    CreatedInFuture {
        ledger_time: u64,
    },
    Duplicate {
        duplicate_of: Nat,
        canister_id: Option<Principal>,
    },
    FailedToCreate {
        error: String,
        fee_block: Option<Nat>,
        refund_block: Option<Nat>,
    },
    GenericError {
        error_code: Nat,
        message: String,
    },
    InsufficientFunds {
        balance: Nat,
    },
    TemporarilyUnavailable,
    TooOld,
}

#[derive(CandidType)]
struct WithdrawArgs {
    amount: Nat,
    created_at_time: Option<u64>,
    from_subaccount: Option<[u8; 32]>,
    to: Principal,
}

#[derive(CandidType, Deserialize)]
enum WithdrawError {
    BadFee {
        expected_fee: Nat,
    },
    CreatedInFuture {
        ledger_time: u64,
    },
    Duplicate {
        duplicate_of: Nat,
    },
    FailedToWithdraw {
        fee_block: Option<Nat>,
        rejection_code: RejectionCode,
        rejection_reason: String,
    },
    GenericError {
        error_code: Nat,
        message: String,
    },
    InsufficientFunds {
        balance: Nat,
    },
    InvalidReceiver {
        receiver: Principal,
    },
    TemporarilyUnavailable,
    TooOld,
}

#[derive(CandidType, Deserialize)]
enum RejectionCode {
    CanisterError,
    CanisterReject,
    DestinationInvalid,
    NoError,
    SysFatal,
    SysTransient,
    Unknown,
}

/// Typed failure at the current IC effect boundary.

#[derive(Debug, ThisError)]
pub enum IcpEnsurePlatformError {
    #[error("cycle arithmetic overflow while executing {0}")]
    Arithmetic(&'static str),

    #[error("configured Candid file is not a regular file: {}", .0.display())]
    CandidUnavailable(PathBuf),

    #[error("configured effect artifact is unavailable: {}", .0.display())]
    ArtifactUnavailable(PathBuf),

    #[error("configured {kind} changed after plan review: expected {expected}, observed {actual}")]
    ArtifactDigestMismatch {
        actual: String,
        expected: String,
        kind: &'static str,
    },

    #[error("configured Principal is invalid for {field}: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("Cycles Ledger returned an unsupported fee value: {0}")]
    InvalidLedgerFee(String),

    #[error("active ICP identity is {actual}, but reviewed Fleet operator is {expected}")]
    OperatorMismatch { actual: String, expected: String },

    #[error("management status for configured canister {expected} returned identity {actual}")]
    StatusIdentityMismatch { actual: String, expected: String },

    #[error("retained Root management observation failed: {0}")]
    RootManagement(String),

    #[error("ICP status has invalid cycle balance for {canister}: {value}")]
    InvalidStatusCycles { canister: String, value: String },

    #[error("ICP status omitted exact {field} required for controlled canister {canister}")]
    IncompleteCanisterStatus {
        canister: String,
        field: &'static str,
    },

    #[error(
        "ICP returned only public non-controller status for {canister}; exact runtime and cycle evidence is unavailable"
    )]
    PublicCanisterStatusUnavailable { canister: String },

    #[error(
        "Root-owned observation for {canister} conflicts with reviewed {field}; no paid effect was repeated"
    )]
    RootOwnedObservationAuthorityConflict {
        canister: String,
        field: &'static str,
    },

    #[error(
        "ICP CLI status JSON omitted canister_version for {canister}, and the exact typed \
         effective-ID-routed management-canister status call failed: {source}; no install was \
         issued. Restore the selected controller identity and management-status access, then \
         resume the same reviewed plan"
    )]
    InstallVersionProofUnavailable {
        canister: String,
        #[source]
        source: Box<IcpManagementCallError>,
    },

    #[error("typed local management-status observation failed: {0}")]
    LocalManagementStatus(#[source] Box<IcpManagementCallError>),

    #[error(
        "completed reinstall proof for {canister} conflicts with current {field}; no install was authorized"
    )]
    CompletedReinstallAuthorityConflict {
        canister: String,
        field: &'static str,
    },

    #[error(transparent)]
    Candid(#[from] IcpCandidCallError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("Cycles Ledger create_canister failed: {0}")]
    LedgerCreate(String),

    #[error("Cycles Ledger duplicate does not yet identify its created canister")]
    LedgerCreatePending,

    #[error("Cycles Ledger withdraw failed: {0}")]
    LedgerWithdraw(String),

    #[error("Cycles Ledger estate funding transfer failed: {0}")]
    LedgerTransfer(String),

    #[error(
        "Root-authorized funding inspection for {canister} conflicts with reviewed {field}; no Ledger withdrawal was repeated"
    )]
    FundingInspectionAuthorityConflict {
        canister: String,
        field: &'static str,
    },

    #[error(
        "canister {canister} retains {actual} cycles above deletion limit {maximum}; left untouched"
    )]
    MaterialCycles {
        actual: u128,
        canister: String,
        maximum: u128,
    },

    #[error("canister {canister} is not stopped; deletion was not attempted")]
    NotStopped { canister: String },

    #[error("effect references unresolved created canister {0}")]
    UnresolvedCreated(String),

    #[error("configured retirement endpoint transferred {actual} cycles, expected {expected}")]
    WrongTransferAmount { actual: Nat, expected: u128 },

    #[error(
        "retirement transfer for {canister} is not conserved: source debit {source_debit}, treasury credit {destination_credit}, expected credit {expected}, maximum source debit {maximum_source_debit}; source and treasury were left untouched by later retirement steps"
    )]
    UnbalancedTransfer {
        canister: String,
        destination_credit: u128,
        expected: u128,
        maximum_source_debit: u128,
        source_debit: u128,
    },

    #[error("retirement transfer for {canister} is missing its durable {side} balance")]
    MissingTransferBalance {
        canister: String,
        side: &'static str,
    },

    #[error("retirement transfer balance for {canister} moved in an invalid direction")]
    InvalidTransferBalance { canister: String },

    #[error(transparent)]
    CurrentProtocol(#[from] current_protocol::CurrentProtocolError),

    #[error(transparent)]
    CanicInit(#[from] canic_init::CanicInitError),

    #[error(transparent)]
    Protocol(#[from] protocol::ProtocolEffectError),

    #[error("retained Root status authority is invalid: {0}")]
    RetainedRootStatusAuthority(#[source] Box<EnsureStateError>),

    #[error("predecessor Root status is invalid: {0}")]
    PredecessorRootStatus(#[source] Box<predecessor_root_status::PredecessorRootStatusError>),
}

/// Production ICP adapter for the current desired Fleet.
pub struct IcpEnsurePlatform {
    desired: DesiredFleet,
    icp: IcpCli,
    initial_observation_delay: Duration,
    maximum_observation_delay: Duration,
    recovery_reinstalls: RefCell<BTreeSet<String>>,
    root: PathBuf,
}

const INITIAL_PROTOCOL_OBSERVATION_DELAY: Duration = Duration::from_millis(250);
const MAXIMUM_PROTOCOL_OBSERVATION_DELAY: Duration = Duration::from_secs(5);

fn protocol_observation_delay(
    consecutive_unchanged_observations: u32,
    initial_observation_delay: Duration,
    maximum_observation_delay: Duration,
) -> Duration {
    let exponent = consecutive_unchanged_observations.saturating_sub(1).min(5);
    let multiplier = 1_u32.checked_shl(exponent).unwrap_or(u32::MAX);
    initial_observation_delay
        .checked_mul(multiplier)
        .unwrap_or(MAXIMUM_PROTOCOL_OBSERVATION_DELAY)
        .min(MAXIMUM_PROTOCOL_OBSERVATION_DELAY)
        .min(maximum_observation_delay)
}

#[derive(Clone, Copy)]
enum RetainedRootOwnedObservationMode {
    DeferredUntilRootStart,
    ReinstallRecovery,
}

impl IcpEnsurePlatform {
    #[must_use]
    pub fn new(desired: DesiredFleet, icp_executable: &str, root: &Path) -> Self {
        let icp = IcpCli::new(icp_executable, Some(desired.environment.clone()))
            .with_cwd(root.to_path_buf());
        Self {
            desired,
            icp,
            initial_observation_delay: INITIAL_PROTOCOL_OBSERVATION_DELAY,
            maximum_observation_delay: MAXIMUM_PROTOCOL_OBSERVATION_DELAY,
            recovery_reinstalls: RefCell::new(BTreeSet::new()),
            root: root.to_path_buf(),
        }
    }

    /// Return this adapter bound to one explicit local replica endpoint.
    ///
    /// This keeps every ordinary ICP CLI call and effective-ID management
    /// observation on the same replica when no named ICP project network owns
    /// the test or operator session.
    #[must_use]
    pub fn with_local_replica(mut self, target: LocalReplicaTarget) -> Self {
        self.icp = self.icp.with_local_replica(Some(target));
        self
    }

    /// Select passive observation pacing without changing retry or stall bounds.
    ///
    /// Production callers retain the default five-second cap. Deterministic
    /// test replicas may select shorter fixed delays while retaining every
    /// production observation and terminal predicate.
    #[doc(hidden)]
    #[must_use]
    pub fn with_observation_delay_bounds(mut self, initial: Duration, maximum: Duration) -> Self {
        self.initial_observation_delay = initial.min(maximum);
        self.maximum_observation_delay = maximum;
        self
    }

    fn require_operator(&self) -> Result<(), IcpEnsurePlatformError> {
        let actual = self.icp.identity_principal_text()?;
        if actual != self.desired.operator {
            return Err(IcpEnsurePlatformError::OperatorMismatch {
                actual,
                expected: self.desired.operator.clone(),
            });
        }
        Ok(())
    }

    fn current_principal<'a>(
        &'a self,
        state: &'a FleetEnsureStateRecord,
        name: &str,
    ) -> Option<&'a str> {
        state
            .pending_principals
            .get(name)
            .or_else(|| state.principals.get(name))
            .map(String::as_str)
            .or_else(|| {
                self.desired
                    .canisters
                    .iter()
                    .find(|configured| configured.name == name)
                    .and_then(|configured| configured.principal.as_deref())
            })
    }

    fn protocol_principals(&self, state: &FleetEnsureStateRecord) -> BTreeMap<String, String> {
        self.desired
            .canisters
            .iter()
            .filter_map(|configured| {
                state
                    .pending_principals
                    .get(&configured.name)
                    .or_else(|| state.principals.get(&configured.name))
                    .or(configured.principal.as_ref())
                    .map(|principal| (configured.name.clone(), principal.clone()))
            })
            .collect()
    }

    fn current_protocol_artifacts_are_live(
        &self,
        state: &FleetEnsureStateRecord,
    ) -> Result<bool, IcpEnsurePlatformError> {
        for configured in self.desired.canisters.iter().filter(|configured| {
            configured.presence == crate::fleet_ensure::model::DesiredPresence::Present
                && matches!(
                    configured.kind,
                    DesiredCanisterKind::Coordinator
                        | DesiredCanisterKind::Root
                        | DesiredCanisterKind::Store
                )
        }) {
            let Some(principal) = self.current_principal(state, &configured.name) else {
                return Ok(false);
            };
            let Some(live) = self.status_optional(principal)? else {
                return Ok(false);
            };
            let Some(wasm) = configured.wasm.as_deref() else {
                return Ok(false);
            };
            let expected = artifact_hash(&resolve_path(&self.root, wasm))?;
            if live.module_sha256.as_deref() != Some(expected.as_str()) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn observe_estate_funding_domains(
        &self,
        state: &FleetEnsureStateRecord,
    ) -> Result<BTreeMap<String, EstateFundingDomainObservation>, IcpEnsurePlatformError> {
        let Some(bootstrap) = self.desired.bootstrap.as_ref() else {
            return Ok(BTreeMap::new());
        };
        bootstrap
            .roots
            .iter()
            .map(|root| {
                let root_principal = self.current_principal(state, &root.root);
                let balance_cycles = root_principal
                    .map(|principal| self.cycles_ledger_balance(principal))
                    .transpose()?;
                let pool = root_principal
                    .map(|principal| {
                        self.observe_estate_pool_inventory(&root.root, principal, state)
                    })
                    .transpose()?
                    .flatten();
                Ok((
                    root.root.clone(),
                    EstateFundingDomainObservation {
                        balance_cycles,
                        cycles_ledger: self.desired.cycles_ledger.clone(),
                        pool,
                        root_principal: root_principal.map(str::to_string),
                    },
                ))
            })
            .collect()
    }

    fn cycles_ledger_balance(&self, owner: &str) -> Result<u128, IcpEnsurePlatformError> {
        let balance: Nat = self.icp.canister_query_candid(
            &self.desired.cycles_ledger,
            "icrc1_balance_of",
            &CyclesLedgerAccount {
                owner: parse_principal("Cycles Ledger account owner", owner)?,
                subaccount: None,
            },
            None,
        )?;
        ledger_fee_cycles(balance)
    }

    fn record_pool_policy_reinstalls(
        &self,
        domains: &BTreeMap<String, EstateFundingDomainObservation>,
    ) {
        let Some(bootstrap) = self.desired.bootstrap.as_ref() else {
            return;
        };
        for root in &bootstrap.roots {
            let Some(pool) = domains
                .get(&root.root)
                .and_then(|domain| domain.pool.as_ref())
            else {
                continue;
            };
            if !pool_policy_is_current(pool, &root.limits.canister_pool) {
                self.record_recovery_reinstalls(&root.root);
            }
        }
    }

    fn observe_estate_pool_inventory(
        &self,
        root_name: &str,
        root: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<EstatePoolInventoryObservation>, IcpEnsurePlatformError> {
        if self.required_root_status(root_name, root)? != CanisterRuntimeStatus::Running {
            return Ok(None);
        }
        let predecessor = self.predecessor_root_status_authorized(root_name, root)?;
        let candid = self.root_protocol_candid()?;
        let root_principal = parse_principal("Fleet Subnet Root", root)?;
        let mut start_after = None;
        let mut inventory = EstatePoolInventoryAccumulator::default();
        loop {
            let page =
                self.query_estate_pool_page(predecessor, &candid, root_principal, start_after)?;
            let next = inventory.observe_page(root_name, page)?;
            if next.is_none() {
                break;
            }
            if next == start_after {
                return Err(pool_configuration_error(format!(
                    "Root {root_name} pool cursor did not advance"
                )));
            }
            start_after = next;
        }
        inventory.finish(root_name).map(Some)
    }

    fn query_estate_pool_page(
        &self,
        predecessor: bool,
        candid: &Path,
        root: Principal,
        start_after: Option<Principal>,
    ) -> Result<CanisterPoolResponse, IcpEnsurePlatformError> {
        if predecessor {
            return predecessor_root_status::query_pool(&self.icp, root, start_after, 256)
                .map_err(|error| IcpEnsurePlatformError::PredecessorRootStatus(Box::new(error)));
        }
        let response: RootPoolStatusResponse = query_with_candid(
            &self.icp,
            candid,
            root,
            canic_protocol::CANIC_ROOT_STATUS,
            &RootPoolStatusRequest::Pool(CanisterPoolStatusRequest {
                start_after,
                limit: 256,
            }),
        )
        .map_err(current_protocol::CurrentProtocolError::from)?;
        let RootPoolStatusResponse::Pool(page) = response;
        Ok(*page)
    }

    fn resolved_controllers(
        &self,
        state: &FleetEnsureStateRecord,
        controllers: &[String],
        controller_canisters: &[String],
    ) -> Result<Vec<String>, IcpEnsurePlatformError> {
        let mut resolved = controllers.to_vec();
        for name in controller_canisters {
            resolved.push(
                self.current_principal(state, name)
                    .ok_or_else(|| IcpEnsurePlatformError::UnresolvedCreated(name.clone()))?
                    .to_string(),
            );
        }
        resolved.sort();
        resolved.dedup();
        Ok(resolved)
    }

    fn retained_store_control_replan_is_exact(
        &self,
        operation_id: &str,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<bool, IcpEnsurePlatformError> {
        let EnsureAction::FleetProtocol {
            action: current_action,
            name,
            principal,
            ..
        } = action
        else {
            return Ok(false);
        };
        let crate::fleet_ensure::model::CurrentFleetProtocolAction::AdoptStore { request } =
            current_action.as_ref()
        else {
            return Ok(false);
        };
        let requested_root = request.authority.fleet_subnet_root.to_text();
        let Some(root) = desired_root_by_principal(&self.desired, state, &requested_root) else {
            return Ok(false);
        };
        let root_name = root.name.as_str();
        if name != &format!("root-store-control:{root_name}") {
            return Ok(false);
        }
        if !current_protocol::retained_store_control_request_is_exact(
            &self.root,
            &self.desired,
            operation_id,
            state,
            root_name,
            request,
        )? {
            return Ok(false);
        }
        let principals = self.protocol_principals(state);
        let Some((_name, expected)) =
            canic_init::compile_root_authorities(&self.root, &self.desired, &principals)?
                .into_iter()
                .find(|(name, _authority)| name == root_name)
        else {
            return Ok(false);
        };
        let root_principal = expected.binding.fleet_subnet_root.to_text();
        let store_principal = expected.wasm_store_authority.wasm_store.to_text();
        let root_topology = state.topology.get(root_name);
        let store = self.desired.canisters.iter().find(|canister| {
            canister.kind == DesiredCanisterKind::Store
                && canister.parent.as_deref() == Some(root_name)
        });
        let Some(store) = store else {
            return Ok(false);
        };
        let store_topology = state.topology.get(&store.name);
        let observed_binding =
            retained_store_control_binding(principal, state, root_name, &store.name);
        let expected_binding = RetainedStoreControlBinding {
            action_root: root_principal.clone(),
            retained_root: Some(root_principal.clone()),
            retained_store: Some(store_principal.clone()),
            root_kind: Some(DesiredCanisterKind::Root),
            root_parent: root.parent.clone(),
            store_kind: Some(DesiredCanisterKind::Store),
            store_parent: Some(root_name.to_string()),
        };
        if observed_binding != expected_binding {
            return Ok(false);
        }

        let Some(root_live) = self.install_status_optional(&root_principal)? else {
            return Ok(false);
        };
        let Some(store_live) = self.install_status_optional(&store_principal)? else {
            return Ok(false);
        };
        let Some(root_wasm) = root.wasm.as_ref() else {
            return Ok(false);
        };
        let Some(store_wasm) = store.wasm.as_ref() else {
            return Ok(false);
        };
        let root_successor = artifact_hash(&resolve_path(&self.root, root_wasm))?;
        let store_successor = artifact_hash(&resolve_path(&self.root, store_wasm))?;
        let root_controllers =
            self.resolved_controllers(state, &root.controllers, &root.controller_canisters)?;
        let store_controllers =
            self.resolved_controllers(state, &store.controllers, &store.controller_canisters)?;
        let observed_live = retained_store_control_live_binding(root_live, store_live);
        Ok(retained_store_control_live_is_exact(
            &observed_live,
            &root_controllers,
            &store_controllers,
            root_topology.and_then(|topology| topology.module_hash.as_deref()),
            store_topology.and_then(|topology| topology.module_hash.as_deref()),
            &root_successor,
            &store_successor,
        ))
    }

    fn observed_protocol_action(
        &self,
        step: &crate::fleet_ensure::model::DesiredProtocolStep,
        principal: String,
    ) -> Result<EnsureAction, IcpEnsurePlatformError> {
        Ok(EnsureAction::Protocol {
            candid: step.candid.clone(),
            candid_sha256: artifact_hash(&resolve_path(&self.root, &step.candid))?,
            command_args: step.command_args.clone(),
            command_args_sha256: artifact_hash(&resolve_path(&self.root, &step.command_args))?,
            command_method: step.command_method.clone(),
            expected_status: step.expected_status.clone(),
            expected_status_sha256: artifact_hash(&resolve_path(
                &self.root,
                &step.expected_status,
            ))?,
            maximum_execution_burn_cycles: step
                .maximum_execution_burn_cycles
                .parse()
                .map_err(|_| IcpEnsurePlatformError::Arithmetic("protocol burn"))?,
            name: step.name.clone(),
            principal,
            status_args: step.status_args.clone(),
            status_args_sha256: artifact_hash(&resolve_path(&self.root, &step.status_args))?,
            status_method: step.status_method.clone(),
        })
    }

    fn action_principal<'a>(
        state: &'a FleetEnsureStateRecord,
        principal: &'a str,
    ) -> Result<&'a str, IcpEnsurePlatformError> {
        if let Some(name) = principal.strip_prefix("created:") {
            return state
                .pending_principals
                .get(name)
                .map(String::as_str)
                .ok_or_else(|| IcpEnsurePlatformError::UnresolvedCreated(name.to_string()));
        }
        Ok(principal)
    }

    fn status_optional(
        &self,
        principal: &str,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        if self.icp.uses_direct_local_replica() {
            return self.direct_local_status_optional(principal);
        }
        let report = match self.icp.canister_status_report(principal) {
            Ok(report) => report,
            Err(error)
                if matches!(
                    error.diagnostic(),
                    Some(IcpDiagnostic::CanisterNotFound { .. })
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        if report.id != principal {
            return Err(IcpEnsurePlatformError::StatusIdentityMismatch {
                actual: report.id,
                expected: principal.to_string(),
            });
        }
        reject_public_status_projection(&report, principal)?;
        let status_text =
            report
                .status
                .ok_or_else(|| IcpEnsurePlatformError::IncompleteCanisterStatus {
                    canister: principal.to_string(),
                    field: "status",
                })?;
        let cycles_text =
            report
                .cycles
                .ok_or_else(|| IcpEnsurePlatformError::IncompleteCanisterStatus {
                    canister: principal.to_string(),
                    field: "cycles",
                })?;
        let cycles = parse_status_cycles(&cycles_text).ok_or_else(|| {
            IcpEnsurePlatformError::InvalidStatusCycles {
                canister: principal.to_string(),
                value: cycles_text,
            }
        })?;
        let mut controllers = report
            .settings
            .ok_or_else(|| IcpEnsurePlatformError::IncompleteCanisterStatus {
                canister: principal.to_string(),
                field: "controllers",
            })?
            .controllers;
        controllers.sort();
        let status = match status_text.to_ascii_lowercase().as_str() {
            "stopped" => CanisterRuntimeStatus::Stopped,
            "stopping" => CanisterRuntimeStatus::Stopping,
            _ => CanisterRuntimeStatus::Running,
        };
        Ok(Some(LiveCanister {
            canister_version: report.canister_version,
            controllers,
            cycles,
            module_sha256: report.module_hash.map(|hash| normalize_hash(&hash)),
            principal: principal.to_string(),
            reinstall_required: false,
            root_owned_lifecycle: None,
            status,
        }))
    }

    fn direct_local_status_optional(
        &self,
        principal: &str,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let canister_id = parse_principal("local canister status target", principal)?;
        let response = self
            .icp
            .management_canister_status_candid::<_, ManagementCanisterObservationResponse>(
                canister_id,
                &ManagementCanisterStatusRequest { canister_id },
            );
        let response = match response {
            Ok(response) => response,
            Err(error)
                if matches!(
                    crate::icp::classify_icp_diagnostic(&error.to_string()),
                    Some(IcpDiagnostic::CanisterNotFound { .. })
                ) =>
            {
                return Ok(None);
            }
            Err(error) => {
                return Err(IcpEnsurePlatformError::LocalManagementStatus(Box::new(
                    error,
                )));
            }
        };
        let cycles_text = response.cycles.to_string();
        let cycles = u128::try_from(response.cycles.0).map_err(|_| {
            IcpEnsurePlatformError::InvalidStatusCycles {
                canister: principal.to_string(),
                value: cycles_text,
            }
        })?;
        let mut controllers = response
            .settings
            .controllers
            .into_iter()
            .map(|controller| controller.to_text())
            .collect::<Vec<_>>();
        controllers.sort();
        controllers.dedup();
        let status = match response.status {
            ManagementCanisterRuntimeStatus::Running => CanisterRuntimeStatus::Running,
            ManagementCanisterRuntimeStatus::Stopping => CanisterRuntimeStatus::Stopping,
            ManagementCanisterRuntimeStatus::Stopped => CanisterRuntimeStatus::Stopped,
        };
        Ok(Some(LiveCanister {
            canister_version: Some(response.version),
            controllers,
            cycles,
            module_sha256: response
                .module_hash
                .map(|hash| canic_core::cdk::utils::hash::hex_bytes(&hash)),
            principal: principal.to_string(),
            reinstall_required: false,
            root_owned_lifecycle: None,
            status,
        }))
    }

    fn inspect_root_owned_canister(
        &self,
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<CanisterStatusResponse>, IcpEnsurePlatformError> {
        let parent = configured.parent.as_deref().ok_or_else(|| {
            IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                canister: configured.name.clone(),
                field: "Root parent",
            }
        })?;
        let root = self.current_principal(state, parent).ok_or_else(|| {
            IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                canister: configured.name.clone(),
                field: "Root Principal",
            }
        })?;
        let root_configured = self
            .desired
            .canisters
            .iter()
            .find(|candidate| {
                candidate.name == parent && candidate.kind == DesiredCanisterKind::Root
            })
            .ok_or_else(
                || IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                    canister: configured.name.clone(),
                    field: "Root desired authority",
                },
            )?;
        let root_wasm = root_configured.wasm.as_deref().ok_or_else(|| {
            IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                canister: configured.name.clone(),
                field: "Root artifact",
            }
        })?;
        let expected_root_module = artifact_hash(&resolve_path(&self.root, root_wasm))?;
        let root_live = self.status_optional(root)?.ok_or_else(|| {
            IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                canister: configured.name.clone(),
                field: "live Root",
            }
        })?;
        let expected_root_controllers = self.resolved_controllers(
            state,
            &root_configured.controllers,
            &root_configured.controller_canisters,
        )?;
        if root_live.controllers != expected_root_controllers {
            return Err(
                IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                    canister: configured.name.clone(),
                    field: "Root controllers",
                },
            );
        }
        let Some(root_module) = root_live.module_sha256.as_deref() else {
            return Ok(None);
        };
        if root_module != expected_root_module {
            return Err(
                IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                    canister: configured.name.clone(),
                    field: "Root module SHA-256",
                },
            );
        }
        if root_live.status != CanisterRuntimeStatus::Running {
            return Ok(None);
        }
        self.require_operator()?;
        let response: RootInspectionResponse = call_with_candid(
            &self.icp,
            &self.root_protocol_candid()?,
            parse_principal("Fleet Subnet Root", root)?,
            canic_protocol::CANIC_ROOT_COMMAND,
            &RootInspectionCommand::InspectCanister(CanisterInspectionRequest {
                canister_id: parse_principal("Root-owned canister", principal)?,
            }),
        )
        .map_err(current_protocol::CurrentProtocolError::from)?;
        let RootInspectionResponse::InspectCanister(response) = response;
        Ok(Some(response))
    }

    fn created_canister_cycles(
        &self,
        name: &str,
        principal: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<(Option<u128>, bool), IcpEnsurePlatformError> {
        match self.status_optional(principal) {
            Ok(live) => return Ok((live.map(|live| live.cycles), false)),
            Err(IcpEnsurePlatformError::PublicCanisterStatusUnavailable { .. }) => {}
            Err(error) => return Err(error),
        }
        let configured = self
            .desired
            .canisters
            .iter()
            .find(|configured| configured.name == name)
            .ok_or_else(
                || IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                    canister: name.to_string(),
                    field: "desired canister",
                },
            )?;
        if configured.kind != DesiredCanisterKind::Pool {
            return Err(IcpEnsurePlatformError::PublicCanisterStatusUnavailable {
                canister: principal.to_string(),
            });
        }
        let Some(response) = self.inspect_root_owned_canister(configured, principal, state)? else {
            return Ok((None, true));
        };
        let parent = configured.parent.as_deref().ok_or_else(|| {
            IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                canister: name.to_string(),
                field: "Root parent",
            }
        })?;
        let root = self.current_principal(state, parent).ok_or_else(|| {
            IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                canister: name.to_string(),
                field: "Root Principal",
            }
        })?;
        let cycles = validate_root_funding_inspection(
            name,
            configured.kind,
            root,
            &response.settings.controllers,
            response.module_hash.as_deref(),
            &response.cycles,
        )?;
        Ok((Some(cycles), false))
    }

    fn install_status_optional(
        &self,
        principal: &str,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let Some(mut live) = self.status_optional(principal)? else {
            return Ok(None);
        };
        let exact = exact_install_canister_status(
            &self.icp,
            principal,
            live.canister_version,
            live.module_sha256.clone(),
        )?;
        live.canister_version = Some(exact.canister_version);
        live.module_sha256 = exact.module_sha256;
        Ok(Some(live))
    }

    fn has_stopped_retained_protocol_owner(
        &self,
        state: &FleetEnsureStateRecord,
    ) -> Result<bool, IcpEnsurePlatformError> {
        for configured in self.desired.canisters.iter().filter(|configured| {
            configured.presence == crate::fleet_ensure::model::DesiredPresence::Present
                && matches!(
                    configured.kind,
                    DesiredCanisterKind::Coordinator
                        | DesiredCanisterKind::Root
                        | DesiredCanisterKind::Store
                )
        }) {
            let Some(principal) = self.current_principal(state, &configured.name) else {
                continue;
            };
            let Some(live) = self.status_optional(principal)? else {
                continue;
            };
            if live.status != CanisterRuntimeStatus::Running {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn observe_configured_canister(
        &self,
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        if configured.kind == DesiredCanisterKind::Pool {
            return self.observe_root_owned_canister(configured, principal, state);
        }
        match self.status_optional(principal) {
            Ok(observed) => Ok(observed),
            Err(error)
                if matches!(
                    configured.kind,
                    DesiredCanisterKind::Store | DesiredCanisterKind::Pool
                ) && !matches!(
                    error,
                    IcpEnsurePlatformError::Icp(ref source)
                        if matches!(
                            source.diagnostic(),
                            Some(IcpDiagnostic::CanisterNotFound { .. })
                        )
                ) =>
            {
                self.observe_root_owned_canister(configured, principal, state)
            }
            Err(error) => Err(error),
        }
    }

    fn observe_root_owned_canister(
        &self,
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let candid = self.root_protocol_candid()?;
        let parent = configured.parent.as_deref().ok_or_else(|| {
            current_protocol::CurrentProtocolError::Configuration(format!(
                "Root-owned canister {} has no Root parent",
                configured.name
            ))
        })?;
        let root = self.current_principal(state, parent).ok_or_else(|| {
            current_protocol::CurrentProtocolError::Configuration(format!(
                "Root-owned canister {} has no resolved Root",
                configured.name
            ))
        })?;
        let retained_observation = |mode| {
            self.retained_root_owned_observation(configured, principal, parent, root, state, mode)
        };
        if self.required_root_status(&configured.name, root)? == CanisterRuntimeStatus::Stopped {
            return retained_observation(RetainedRootOwnedObservationMode::DeferredUntilRootStart);
        }
        let predecessor_status = self.predecessor_root_status_authorized(parent, root)?;
        if predecessor_status {
            self.record_recovery_reinstalls(parent);
        }
        let target = parse_principal("Root-owned canister", principal)?;
        let mut start_after = None;
        loop {
            let page = if predecessor_status {
                predecessor_root_status::query_pool(
                    &self.icp,
                    parse_principal("Fleet Subnet Root", root)?,
                    start_after,
                    256,
                )
                .map_err(|error| IcpEnsurePlatformError::PredecessorRootStatus(Box::new(error)))?
            } else {
                let response: Result<RootPoolStatusResponse, CanisterProtocolError> =
                    query_with_candid(
                        &self.icp,
                        &candid,
                        parse_principal("Fleet Subnet Root", root)?,
                        canic_protocol::CANIC_ROOT_STATUS,
                        &RootPoolStatusRequest::Pool(CanisterPoolStatusRequest {
                            start_after,
                            limit: 256,
                        }),
                    );
                let response = match response {
                    Ok(response) => response,
                    Err(error) if recoverable_root_status_error(&error) => {
                        return retained_observation(
                            RetainedRootOwnedObservationMode::ReinstallRecovery,
                        );
                    }
                    Err(error) => {
                        return Err(current_protocol::CurrentProtocolError::from(error).into());
                    }
                };
                let RootPoolStatusResponse::Pool(page) = response;
                *page
            };
            if let Some(asset) = page
                .entries
                .into_iter()
                .find(|asset| asset.canister_id == target)
            {
                if matches!(
                    asset.status,
                    canic_core::dto::pool::CanisterPoolAssetStatus::PendingReset
                ) && asset.cycles.to_u128() == 0
                {
                    return retained_observation(
                        RetainedRootOwnedObservationMode::ReinstallRecovery,
                    );
                }
                return Self::observed_root_owned_asset(configured, principal, root, asset);
            }
            let next = page.next_start_after;
            if next.is_none() {
                return Ok(None);
            }
            if next == start_after {
                return Err(
                    current_protocol::CurrentProtocolError::Configuration(format!(
                        "Root {parent} pool cursor did not advance"
                    ))
                    .into(),
                );
            }
            start_after = next;
        }
    }

    fn inspect_root_owned_funding_balance(
        &self,
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<u128, IcpEnsurePlatformError> {
        let parent = configured.parent.as_deref().ok_or_else(|| {
            IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                canister: configured.name.clone(),
                field: "Root parent",
            }
        })?;
        let root = self.current_principal(state, parent).ok_or_else(|| {
            IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                canister: configured.name.clone(),
                field: "Root Principal",
            }
        })?;
        if self.required_root_status(&configured.name, root)? != CanisterRuntimeStatus::Running {
            return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                canister: configured.name.clone(),
                field: "running Root",
            });
        }
        self.require_operator()?;
        let target = parse_principal("Root-owned funding target", principal)?;
        let response: RootInspectionResponse = call_with_candid(
            &self.icp,
            &self.root_protocol_candid()?,
            parse_principal("Fleet Subnet Root", root)?,
            canic_protocol::CANIC_ROOT_COMMAND,
            &RootInspectionCommand::InspectCanister(CanisterInspectionRequest {
                canister_id: target,
            }),
        )
        .map_err(current_protocol::CurrentProtocolError::from)?;
        let RootInspectionResponse::InspectCanister(response) = response;
        validate_root_funding_inspection(
            &configured.name,
            configured.kind,
            root,
            &response.settings.controllers,
            response.module_hash.as_deref(),
            &response.cycles,
        )
    }

    fn observed_root_owned_asset(
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        root: &str,
        asset: CanisterPoolAsset,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let Some(root_owned_lifecycle) = root_owned_lifecycle(configured.kind, &asset.status)
        else {
            return Err(
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "Root-owned canister {} has a live pool role incompatible with desired {:?}",
                    configured.name, configured.kind
                ))
                .into(),
            );
        };
        let status = match root_owned_lifecycle {
            RootOwnedCanisterLifecycle::Store | RootOwnedCanisterLifecycle::Workload => {
                CanisterRuntimeStatus::Running
            }
            RootOwnedCanisterLifecycle::Claimed
            | RootOwnedCanisterLifecycle::Idle
            | RootOwnedCanisterLifecycle::Retained => CanisterRuntimeStatus::Stopped,
        };
        Ok(Some(LiveCanister {
            canister_version: None,
            controllers: vec![root.to_string()],
            cycles: asset.cycles.to_u128(),
            module_sha256: None,
            principal: principal.to_string(),
            reinstall_required: false,
            root_owned_lifecycle: Some(root_owned_lifecycle),
            status,
        }))
    }

    fn predecessor_root_status_authorized(
        &self,
        root_name: &str,
        root: &str,
    ) -> Result<bool, IcpEnsurePlatformError> {
        let paths = EnsurePaths::under(&self.root, &self.desired.environment, &self.desired.fleet);
        let Some(authority) = read_root_start_authority(&paths).map_err(|error| {
            IcpEnsurePlatformError::RetainedRootStatusAuthority(Box::new(error))
        })?
        else {
            return Ok(false);
        };
        let Some(binding) = authority
            .roots
            .iter()
            .find(|binding| binding.principal == root)
        else {
            return Ok(false);
        };
        let live = self.status_optional(root)?.ok_or_else(|| {
            current_protocol::CurrentProtocolError::Configuration(format!(
                "retained predecessor Root {root_name} is unavailable"
            ))
        })?;
        if live.module_sha256.as_deref() != Some(binding.predecessor_module_sha256.as_str()) {
            return Ok(false);
        }
        let bootstrap = self.desired.bootstrap.as_ref().ok_or_else(|| {
            current_protocol::CurrentProtocolError::Configuration(
                "predecessor Root status requires current Fleet bootstrap authority".to_string(),
            )
        })?;
        let configured = self
            .desired
            .canisters
            .iter()
            .find(|configured| {
                configured.name == root_name && configured.kind == DesiredCanisterKind::Root
            })
            .ok_or_else(|| {
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "predecessor Root {root_name} is absent from desired topology"
                ))
            })?;
        let successor_wasm = configured.wasm.as_deref().ok_or_else(|| {
            current_protocol::CurrentProtocolError::Configuration(format!(
                "predecessor Root {root_name} has no desired successor artifact"
            ))
        })?;
        let successor_module = artifact_hash(&resolve_path(&self.root, successor_wasm))?;
        let identity_matches = authority.environment == self.desired.environment
            && authority.fleet == self.desired.fleet
            && authority.fleet_id == bootstrap.fleet_id
            && authority.release_build_id == bootstrap.release_build_id
            && authority.successor_module_sha256 == successor_module
            && binding.name == configured.name
            && configured.principal.as_deref() == Some(root)
            && binding.subnet == configured.subnet
            && binding.controllers == configured.controllers
            && live.controllers == configured.controllers;
        if !identity_matches {
            return Err(
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "predecessor Root {root_name} conflicts with sealed successor authority"
                ))
                .into(),
            );
        }
        verify_root_start_release_authority(&self.root, &authority).map_err(|error| {
            IcpEnsurePlatformError::RetainedRootStatusAuthority(Box::new(error))
        })?;
        Ok(true)
    }

    fn root_protocol_candid(&self) -> Result<PathBuf, IcpEnsurePlatformError> {
        let protocol = self.desired.protocol.as_ref().ok_or_else(|| {
            current_protocol::CurrentProtocolError::Configuration(
                "Root-owned observation requires typed Fleet protocol".to_string(),
            )
        })?;
        Ok(resolve_path(&self.root, &protocol.root_candid))
    }

    fn required_root_status(
        &self,
        configured_name: &str,
        root: &str,
    ) -> Result<CanisterRuntimeStatus, IcpEnsurePlatformError> {
        self.status_optional(root)?
            .map(|live| live.status)
            .ok_or_else(|| {
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "Root-owned canister {configured_name} has no live Root"
                ))
                .into()
            })
    }

    fn retained_root_owned_observation(
        &self,
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        parent: &str,
        root: &str,
        state: &FleetEnsureStateRecord,
        mode: RetainedRootOwnedObservationMode,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let Some(retained_topology) = exact_retained_root_owned_topology(
            state,
            &configured.name,
            configured.kind,
            parent,
            principal,
            root,
        ) else {
            return Err(
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "Root-owned canister {} has no exact retained topology authority",
                    configured.name
                ))
                .into(),
            );
        };
        let cycles = self
            .controller_cycle_balance(principal)
            .or_else(|| state.retained_cycles_by_principal.get(principal).copied())
            .ok_or_else(|| {
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "Root-owned canister {} has no current or retained exact native balance",
                    configured.name
                ))
            })?;
        if !matches!(
            configured.kind,
            DesiredCanisterKind::Pool | DesiredCanisterKind::Store
        ) || configured.presence != crate::fleet_ensure::model::DesiredPresence::Present
            || configured.replace
            || configured.drain.is_some()
            || (configured.kind == DesiredCanisterKind::Pool && configured.wasm.is_some())
            || (configured.kind == DesiredCanisterKind::Store && configured.wasm.is_none())
        {
            return Err(
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "Root-owned recovery evidence for {} cannot authorize a mutation",
                    configured.name
                ))
                .into(),
            );
        }
        let root_config = self
            .desired
            .canisters
            .iter()
            .find(|canister| canister.name == parent && canister.kind == DesiredCanisterKind::Root)
            .ok_or_else(|| {
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "Root-owned canister {} has no exact configured Root",
                    configured.name
                ))
            })?;
        let live_root = self.status_optional(root)?.ok_or_else(|| {
            current_protocol::CurrentProtocolError::Configuration(format!(
                "Root-owned canister {} has no live Root",
                configured.name
            ))
        })?;
        let mut expected_controllers = root_config.controllers.clone();
        expected_controllers.sort();
        if live_root.controllers != expected_controllers {
            return Err(
                current_protocol::CurrentProtocolError::Configuration(format!(
                    "Root-owned canister {} Root controllers drifted",
                    configured.name
                ))
                .into(),
            );
        }
        if matches!(mode, RetainedRootOwnedObservationMode::ReinstallRecovery) {
            self.record_recovery_reinstalls(parent);
        }
        Ok(Some(LiveCanister {
            canister_version: None,
            controllers: vec![root.to_string()],
            cycles,
            module_sha256: retained_topology.module_hash.clone(),
            principal: principal.to_string(),
            reinstall_required: false,
            root_owned_lifecycle: Some(RootOwnedCanisterLifecycle::Retained),
            status: if configured.kind == DesiredCanisterKind::Store {
                CanisterRuntimeStatus::Running
            } else {
                CanisterRuntimeStatus::Stopped
            },
        }))
    }

    fn record_recovery_reinstalls(&self, root_name: &str) {
        let mut recovery = self.recovery_reinstalls.borrow_mut();
        recovery.insert(root_name.to_string());
        recovery.extend(
            self.desired
                .canisters
                .iter()
                .filter(|canister| {
                    canister.kind == DesiredCanisterKind::Coordinator
                        || (canister.kind == DesiredCanisterKind::Store
                            && canister.parent.as_deref() == Some(root_name))
                })
                .map(|canister| canister.name.clone()),
        );
    }

    fn completed_reinstall_is_current(
        &self,
        state: &FleetEnsureStateRecord,
        name: &str,
        live: &LiveCanister,
    ) -> Result<bool, IcpEnsurePlatformError> {
        if state.completed_reinstall_operation_id.is_none()
            || !state.completed_reinstall_action_sha256.contains_key(name)
        {
            return Ok(false);
        }
        let Some(pre_canister_version) = state.completed_reinstalls.get(name) else {
            return Ok(false);
        };
        let Some(configured) = self
            .desired
            .canisters
            .iter()
            .find(|configured| configured.name == name)
        else {
            return Ok(false);
        };
        let Some(wasm) = configured.wasm.as_deref() else {
            return Ok(false);
        };
        let principal_matches = state
            .principals
            .get(name)
            .is_some_and(|principal| principal == &live.principal);
        let retained_topology = state.topology.get(name);
        let topology_matches = retained_topology.is_some_and(|topology| {
            topology.kind == configured.kind && topology.parent == configured.parent
        });
        let desired_module_sha256 = artifact_hash(&resolve_path(&self.root, wasm))?;
        let root_owned_store_module = (live.module_sha256.is_none()
            && live.root_owned_lifecycle == Some(RootOwnedCanisterLifecycle::Store))
        .then(|| {
            configured
                .parent
                .as_deref()
                .and_then(|parent| state.principals.get(parent))
                .filter(|root| live.controllers.as_slice() == [root.as_str()])
                .and(retained_topology)
                .and_then(|topology| topology.module_hash.as_deref())
        })
        .flatten();
        completed_reinstall_continuity(
            *pre_canister_version,
            principal_matches,
            topology_matches,
            live.module_sha256.as_deref().or(root_owned_store_module),
            &desired_module_sha256,
            live.canister_version,
        )
        .map_err(
            |field| IcpEnsurePlatformError::CompletedReinstallAuthorityConflict {
                canister: name.to_string(),
                field,
            },
        )?;
        Ok(true)
    }

    fn controller_cycle_balance(&self, principal: &str) -> Option<u128> {
        let response: Result<ManagedCanisterStatusResponse, canic_core::dto::error::Error> = self
            .icp
            .canister_query_candid(
                principal,
                canic_protocol::CANIC_STATUS,
                &ManagedCanisterStatusRequest::CycleBalance,
                None,
            )
            .ok()?;
        let ManagedCanisterStatusResponse::CycleBalance(balance) = response.ok()?;
        Some(balance.cycles)
    }

    fn apply_create(
        &self,
        authority: CreateCanisterAuthority<'_>,
        state: &FleetEnsureStateRecord,
    ) -> Result<EffectOutcome, IcpEnsurePlatformError> {
        let creation_fee = self
            .desired
            .management_creation_fee_cycles
            .parse::<Cycles>()
            .map(|cycles| cycles.to_u128())
            .map_err(|_| IcpEnsurePlatformError::Arithmetic("management creation fee"))?;
        let amount = authority
            .requested_initial_cycles
            .checked_add(creation_fee)
            .ok_or(IcpEnsurePlatformError::Arithmetic("creation amount"))?;
        let controllers = self
            .resolved_controllers(state, authority.controllers, authority.controller_canisters)?
            .iter()
            .map(|value| parse_principal("controller", value))
            .collect::<Result<Vec<_>, _>>()?;
        let request = CreateCanisterArgs {
            amount: Nat::from(amount),
            created_at_time: Some(authority.created_at_time),
            creation_args: Some(CmcCreateCanisterArgs {
                settings: Some(CanisterSettings {
                    compute_allocation: None,
                    controllers: Some(controllers),
                    freezing_threshold: None,
                    memory_allocation: None,
                    reserved_cycles_limit: None,
                }),
                subnet_selection: Some(SubnetSelection::Subnet {
                    subnet: parse_principal("subnet", authority.subnet)?,
                }),
            }),
            from_subaccount: None,
        };
        let response: Result<CreateCanisterSuccess, CreateCanisterError> = self
            .icp
            .canister_call_candid(authority.ledger, "create_canister", &request, None)?;
        create_canister_response_outcome(response, authority.requested_initial_cycles)
    }

    fn apply_fund(
        &self,
        amount: u128,
        created_at_time: u64,
        ledger: &str,
        principal: &str,
    ) -> Result<EffectOutcome, IcpEnsurePlatformError> {
        let request = WithdrawArgs {
            amount: Nat::from(amount),
            created_at_time: Some(created_at_time),
            from_subaccount: None,
            to: parse_principal("funding target", principal)?,
        };
        let response: Result<Nat, WithdrawError> = self
            .icp
            .canister_call_candid(ledger, "withdraw", &request, None)?;
        match response {
            Ok(block) => Ok(EffectOutcome {
                created_principal: None,
                post_cycles: None,
                receipt: Some(block.to_string()),
            }),
            Err(WithdrawError::Duplicate { duplicate_of }) => Ok(EffectOutcome {
                created_principal: None,
                post_cycles: None,
                receipt: Some(duplicate_of.to_string()),
            }),
            Err(error) => Err(IcpEnsurePlatformError::LedgerWithdraw(
                render_withdraw_error(error),
            )),
        }
    }

    fn apply_estate_fund(
        &self,
        amount: u128,
        created_at_time: u64,
        ledger: &str,
        principal: &str,
    ) -> Result<EffectOutcome, IcpEnsurePlatformError> {
        let request = CyclesLedgerTransferArgs {
            amount: Nat::from(amount),
            created_at_time: Some(created_at_time),
            fee: None,
            from_subaccount: None,
            memo: None,
            to: CyclesLedgerAccount {
                owner: parse_principal("estate funding target", principal)?,
                subaccount: None,
            },
        };
        let response: Result<Nat, CyclesLedgerTransferError> =
            self.icp
                .canister_call_candid(ledger, "icrc1_transfer", &request, None)?;
        match response {
            Ok(block) => Ok(EffectOutcome {
                created_principal: None,
                post_cycles: None,
                receipt: Some(block.to_string()),
            }),
            Err(CyclesLedgerTransferError::Duplicate { duplicate_of }) => Ok(EffectOutcome {
                created_principal: None,
                post_cycles: None,
                receipt: Some(duplicate_of.to_string()),
            }),
            Err(error) => Err(IcpEnsurePlatformError::LedgerTransfer(
                render_ledger_transfer_error(error),
            )),
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the install boundary verifies one complete immutable action tuple"
    )]
    fn apply_install(
        &self,
        operation_id: &str,
        principals: &BTreeMap<String, String>,
        canic_init: Option<&crate::fleet_ensure::model::DesiredCanisterInit>,
        init_arg: Option<&str>,
        init_arg_sha256: Option<&str>,
        init_candid: Option<&str>,
        init_candid_sha256: Option<&str>,
        mode: InstallMode,
        principal: &str,
        wasm: &str,
        wasm_sha256: &str,
    ) -> Result<EffectOutcome, IcpEnsurePlatformError> {
        verify_artifact(&resolve_path(&self.root, wasm), wasm_sha256, "Wasm")?;
        let mut command = self.icp.canister_command();
        command.args(["install", principal, "--mode"]);
        command.arg(match mode {
            InstallMode::Install => "install",
            InstallMode::Reinstall => "reinstall",
        });
        command.args(["--yes", "--wasm"]);
        command.arg(resolve_path(&self.root, wasm));
        let generated_init = if let Some(canic_init) = canic_init {
            let path = canic_init::write_arguments(canic_init::CanicInitRequest {
                desired: &self.desired,
                init: canic_init,
                operation_id,
                principals,
                root: &self.root,
                wasm,
                wasm_sha256,
            })?;
            command.arg("--args-file");
            command.arg(&path);
            command.args(["--args-format", "bin"]);
            Some(path)
        } else if let Some(init_arg) = init_arg {
            let expected =
                init_arg_sha256.ok_or_else(|| IcpEnsurePlatformError::ArtifactDigestMismatch {
                    actual: "missing".to_string(),
                    expected: "reviewed init argument identity".to_string(),
                    kind: "init argument",
                })?;
            let init_candid =
                init_candid.ok_or_else(|| IcpEnsurePlatformError::ArtifactDigestMismatch {
                    actual: "missing".to_string(),
                    expected: "reviewed init Candid".to_string(),
                    kind: "init Candid",
                })?;
            let init_candid_sha256 = init_candid_sha256.ok_or_else(|| {
                IcpEnsurePlatformError::ArtifactDigestMismatch {
                    actual: "missing".to_string(),
                    expected: "reviewed init Candid identity".to_string(),
                    kind: "init Candid",
                }
            })?;
            let path = protocol::write_init_arguments(
                &self.root,
                operation_id,
                principals,
                init_candid,
                init_candid_sha256,
                init_arg,
                expected,
            )?;
            command.arg("--args-file");
            command.arg(&path);
            command.args(["--args-format", "bin"]);
            Some(path)
        } else {
            command.args(["--args", "()"]);
            None
        };
        self.icp.add_target_args(&mut command);
        let result = run_status(&mut command).map_err(IcpEnsurePlatformError::from);
        if let Some(path) = generated_init {
            std::fs::remove_file(&path)
                .map_err(|_| IcpEnsurePlatformError::ArtifactUnavailable(path))?;
        }
        result?;
        Ok(empty_outcome())
    }

    fn apply_controllers(
        &self,
        principal: &str,
        controllers: &[String],
    ) -> Result<EffectOutcome, IcpEnsurePlatformError> {
        let mut command = self.icp.canister_command();
        command.args([
            "settings",
            "update",
            principal,
            "--force",
            "--remove-all-controllers",
        ]);
        for controller in controllers {
            parse_principal("controller", controller)?;
            command.args(["--add-controller", controller]);
        }
        self.icp.add_target_args(&mut command);
        run_status(&mut command)?;
        Ok(empty_outcome())
    }

    fn observe_configured_canisters(
        &self,
        state: &FleetEnsureStateRecord,
    ) -> Result<BTreeMap<String, Option<LiveCanister>>, IcpEnsurePlatformError> {
        self.desired
            .canisters
            .iter()
            .map(|configured| {
                let observed = self
                    .current_principal(state, &configured.name)
                    .map(|principal| self.observe_configured_canister(configured, principal, state))
                    .transpose()?
                    .flatten();
                Ok((configured.name.clone(), observed))
            })
            .collect()
    }

    fn reconcile_recovery_reinstalls(
        &self,
        state: &FleetEnsureStateRecord,
        canisters: &mut BTreeMap<String, Option<LiveCanister>>,
    ) -> Result<(), IcpEnsurePlatformError> {
        let mut completed = BTreeSet::new();
        for name in self.recovery_reinstalls.borrow().iter() {
            let Some(live) = canisters.get_mut(name).and_then(Option::as_mut) else {
                continue;
            };
            if self.completed_reinstall_is_current(state, name, live)? {
                if live.module_sha256.is_none()
                    && live.root_owned_lifecycle == Some(RootOwnedCanisterLifecycle::Store)
                {
                    live.module_sha256 = state
                        .topology
                        .get(name)
                        .and_then(|topology| topology.module_hash.clone());
                }
                completed.insert(name.clone());
            }
        }
        self.recovery_reinstalls
            .borrow_mut()
            .retain(|name| !completed.contains(name));
        for name in self.recovery_reinstalls.borrow().iter() {
            if let Some(live) = canisters.get_mut(name).and_then(Option::as_mut) {
                live.reinstall_required = true;
            }
        }
        Ok(())
    }

    fn additional_pool_cycles(
        canisters: &BTreeMap<String, Option<LiveCanister>>,
        domains: &BTreeMap<String, EstateFundingDomainObservation>,
    ) -> Result<BTreeMap<String, u128>, IcpEnsurePlatformError> {
        let configured = canisters
            .values()
            .filter_map(|live| live.as_ref().map(|live| live.principal.as_str()))
            .collect::<BTreeSet<_>>();
        let mut additional = BTreeMap::new();
        for asset in domains
            .values()
            .filter_map(|domain| domain.pool.as_ref())
            .flat_map(|pool| &pool.assets)
        {
            if configured.contains(asset.principal.as_str()) {
                continue;
            }
            if additional
                .insert(asset.principal.clone(), asset.cycles)
                .is_some()
            {
                return Err(pool_configuration_error(format!(
                    "controlled pool canister {} is retained by more than one Root",
                    asset.principal
                )));
            }
        }
        Ok(additional)
    }

    fn observe_protocol_readiness(
        &self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
        canisters: &BTreeMap<String, Option<LiveCanister>>,
    ) -> Result<BTreeMap<String, bool>, IcpEnsurePlatformError> {
        let principals = self.protocol_principals(state);
        self.desired
            .protocol_steps
            .iter()
            .map(|step| {
                let Some(principal) = principals.get(&step.canister) else {
                    return Ok((step.name.clone(), false));
                };
                let Some(live) = canisters.get(&step.canister).and_then(Option::as_ref) else {
                    return Ok((step.name.clone(), false));
                };
                let configured = self
                    .desired
                    .canisters
                    .iter()
                    .find(|configured| configured.name == step.canister)
                    .expect("protocol target was validated against desired canisters");
                if live.reinstall_required {
                    return Ok((step.name.clone(), false));
                }
                if let Some(wasm) = &configured.wasm {
                    let desired_hash = artifact_hash(&resolve_path(&self.root, wasm))?;
                    if live.module_sha256.as_deref() != Some(desired_hash.as_str()) {
                        return Ok((step.name.clone(), false));
                    }
                }
                let action = self.observed_protocol_action(step, principal.clone())?;
                protocol::observe(&self.icp, &self.root, operation_id, &principals, &action)
                    .map(|observation| (step.name.clone(), observation.applied))
                    .map_err(IcpEnsurePlatformError::from)
            })
            .collect()
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one typed transfer effect carries the complete reviewed authority tuple"
    )]
    fn apply_transfer(
        &self,
        amount: u128,
        candid: &str,
        candid_sha256: &str,
        destination: &str,
        method: &str,
        operation_id: &str,
        principal: &str,
    ) -> Result<EffectOutcome, IcpEnsurePlatformError> {
        let candid = resolve_path(&self.root, candid);
        if !candid.is_file() {
            return Err(IcpEnsurePlatformError::CandidUnavailable(candid));
        }
        verify_artifact(&candid, candid_sha256, "drain Candid")?;
        let response: DrainResponse = self.icp.canister_call_candid(
            principal,
            method,
            &DrainRequest {
                operation_id: operation_id.to_string(),
                destination: parse_principal("treasury", destination)?,
                cycles: Nat::from(amount),
            },
            Some(&candid),
        )?;
        let transferred = match response {
            DrainResponse::Accepted { transferred_cycles }
            | DrainResponse::Replayed { transferred_cycles } => transferred_cycles,
        };
        if transferred != amount {
            return Err(IcpEnsurePlatformError::WrongTransferAmount {
                actual: transferred,
                expected: amount,
            });
        }
        Ok(EffectOutcome {
            created_principal: None,
            post_cycles: None,
            receipt: Some(operation_id.to_string()),
        })
    }
}

impl EnsurePlatform for IcpEnsurePlatform {
    type Error = IcpEnsurePlatformError;

    fn bind_reviewed_desired(&mut self, desired: &DesiredFleet) -> Result<(), Self::Error> {
        self.desired = desired.clone();
        Ok(())
    }

    fn pace_effect_observation(
        &mut self,
        _action: &EnsureAction,
        consecutive_unchanged_observations: u32,
    ) {
        thread::sleep(protocol_observation_delay(
            consecutive_unchanged_observations,
            self.initial_observation_delay,
            self.maximum_observation_delay,
        ));
    }

    fn pace_root_owned_observation(
        &mut self,
        _target: &str,
        consecutive_retained_observations: u32,
    ) {
        thread::sleep(protocol_observation_delay(
            consecutive_retained_observations,
            self.initial_observation_delay,
            self.maximum_observation_delay,
        ));
    }

    fn observe_root_management(
        &mut self,
        state: &FleetEnsureStateRecord,
        reviewed_targets: &BTreeSet<String>,
    ) -> Result<Option<RootManagementObservation>, Self::Error> {
        let configured_roots = self
            .desired
            .canisters
            .iter()
            .filter(|configured| {
                configured.kind == DesiredCanisterKind::Root
                    && configured.presence == crate::fleet_ensure::model::DesiredPresence::Present
            })
            .collect::<Vec<_>>();
        if configured_roots.is_empty() {
            return Ok(None);
        }
        self.require_operator()?;
        let mut observed_roots = Vec::new();
        for configured in configured_roots {
            let Some(principal) = self.current_principal(state, &configured.name) else {
                if is_unallocated_fresh_root(&self.desired, configured, reviewed_targets) {
                    continue;
                }
                return Err(IcpEnsurePlatformError::RootManagement(format!(
                    "configured Root {} has no exact Principal",
                    configured.name
                )));
            };
            let live = self.status_optional(principal)?.ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement(format!(
                    "configured Root {} is unavailable",
                    configured.name
                ))
            })?;
            observed_roots.push((configured, live));
        }
        if reviewed_targets.is_empty()
            && observed_roots
                .iter()
                .all(|(_, live)| live.status == CanisterRuntimeStatus::Running)
        {
            return Ok(None);
        }
        let network = resolve_icp_build_network_from_root(&self.root, &self.desired.environment)
            .map_err(|error| IcpEnsurePlatformError::RootManagement(error.to_string()))?;
        let catalog = if network == BuildNetwork::Ic {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| {
                    IcpEnsurePlatformError::RootManagement(
                        "system clock is before the Unix epoch".to_string(),
                    )
                })?
                .as_secs();
            Some(
                load_mainnet_subnet_catalog(&self.root, now)
                    .map_err(|error| IcpEnsurePlatformError::RootManagement(error.to_string()))?,
            )
        } else {
            None
        };
        let mut roots = BTreeMap::new();
        for (configured, live) in observed_roots {
            let principal = live.principal.clone();
            let subnet = catalog.as_ref().map_or_else(
                || Ok(configured.subnet.clone()),
                |catalog| {
                    catalog
                        .catalog
                        .resolve_canister_route(&principal)
                        .map(|route| route.subnet.to_text())
                        .map_err(|error| IcpEnsurePlatformError::RootManagement(error.to_string()))
                },
            )?;
            let name = configured.name.clone();
            if roots
                .insert(
                    name.clone(),
                    RootManagementCanisterObservation { live, name, subnet },
                )
                .is_some()
            {
                return Err(IcpEnsurePlatformError::RootManagement(
                    "configured Root names are not unique".to_string(),
                ));
            }
        }
        let operator_cycles = self
            .icp
            .identity_cycles_balance()
            .map_err(|error| IcpEnsurePlatformError::LedgerWithdraw(error.to_string()))?;
        Ok(Some(RootManagementObservation {
            operator_cycles,
            roots,
        }))
    }

    fn observe(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        self.require_operator()?;
        self.recovery_reinstalls.borrow_mut().clear();
        let mut canisters = self.observe_configured_canisters(state)?;
        self.reconcile_recovery_reinstalls(state, &mut canisters)?;
        let estate_funding_domains = self.observe_estate_funding_domains(state)?;
        self.record_pool_policy_reinstalls(&estate_funding_domains);
        self.reconcile_recovery_reinstalls(state, &mut canisters)?;
        let additional_controlled_cycles =
            Self::additional_pool_cycles(&canisters, &estate_funding_domains)?;
        let protocol_ready = self.observe_protocol_readiness(operation_id, state, &canisters)?;
        Ok(FleetObservation {
            additional_controlled_cycles,
            canisters,
            estate_funding_domains,
            ledger_fee_cycles: ledger_fee_cycles(self.icp.canister_query_candid(
                &self.desired.cycles_ledger,
                "icrc1_fee",
                &(),
                None,
            )?)?,
            operator_cycles: self
                .icp
                .identity_cycles_balance()
                .map_err(|error| IcpEnsurePlatformError::LedgerWithdraw(error.to_string()))?,
            protocol_ready,
        })
    }

    fn protocol_actions(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Vec<EnsureAction>, Self::Error> {
        if self.desired.protocol.is_none() {
            return Ok(Vec::new());
        }
        current_protocol::validate_component_pool_capacity(&self.root, &self.desired)?;
        if self.has_stopped_retained_protocol_owner(state)? {
            return Ok(Vec::new());
        }
        if !self.recovery_reinstalls.borrow().is_empty() {
            return Ok(Vec::new());
        }
        if !self.current_protocol_artifacts_are_live(state)? {
            return Ok(Vec::new());
        }
        current_protocol::compile(&self.icp, &self.root, &self.desired, operation_id, state)
            .map_err(Into::into)
    }

    fn terminal_inventory(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<TerminalFleetInventory, Self::Error> {
        if self.desired.protocol.is_none() {
            if state.active_registry.is_some() {
                return Err(current_protocol::CurrentProtocolError::Configuration(
                    "managed terminal Fleet cannot drop its typed protocol intent".to_string(),
                )
                .into());
            }
            return Ok(TerminalFleetInventory::default());
        }
        super::current_inventory::terminal_inventory(
            &self.icp,
            &self.root,
            &self.desired,
            operation_id,
            state,
        )
        .map_err(Into::into)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the platform keeps every effect's exact live terminal predicate in one exhaustive match"
    )]
    fn observe_effect(
        &mut self,
        operation_id: &str,
        action: &EnsureAction,
        record: &EffectRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error> {
        let mut retry = EffectRetry::None;
        let mut post_cycles = None;
        let (applied, progress_identity) = match action {
            EnsureAction::Create {
                name,
                requested_initial_cycles,
                ..
            } => {
                let (live_cycles, deferred) =
                    if record.created_principal.is_some() && record.receipt.is_some() {
                        self.created_canister_cycles(
                            name,
                            record
                                .created_principal
                                .as_deref()
                                .ok_or(IcpEnsurePlatformError::LedgerCreatePending)?,
                            state,
                        )?
                    } else {
                        (None, false)
                    };
                post_cycles = live_cycles;
                let maximum_observation_burn_cycles = self
                    .desired
                    .maximum_observation_burn_cycles
                    .parse::<Cycles>()
                    .map(|cycles| cycles.to_u128())
                    .map_err(|_| IcpEnsurePlatformError::Arithmetic("observation burn"))?;
                let applied = create_balance_is_terminal(
                    live_cycles,
                    *requested_initial_cycles,
                    maximum_observation_burn_cycles,
                );
                if live_cycles.is_some() && !applied {
                    retry = EffectRetry::ReplanRequiredAfterCreateBalanceDrift;
                } else if deferred {
                    retry = EffectRetry::DeferUntilControllerObservation;
                }
                (
                    applied,
                    format!(
                        "created:{:?}:actual:{live_cycles:?}:requested:{requested_initial_cycles}",
                        record.created_principal,
                    ),
                )
            }
            EnsureAction::Delete { principal, .. } => {
                let live = self.status_optional(Self::action_principal(state, principal)?)?;
                (live.is_none(), format!("delete:{live:?}"))
            }
            EnsureAction::Fund {
                amount,
                expected_post_cycles,
                funding_deficit_cycles,
                funding_margin_cycles,
                name,
                principal,
                ..
            } => {
                let live_cycles = if record.receipt.is_some() {
                    let configured = self
                        .desired
                        .canisters
                        .iter()
                        .find(|configured| configured.name == *name)
                        .ok_or_else(|| {
                            current_protocol::CurrentProtocolError::Configuration(format!(
                                "funding target {name} is absent from desired topology"
                            ))
                        })?;
                    if configured.kind == DesiredCanisterKind::Pool {
                        Some(self.inspect_root_owned_funding_balance(
                            configured,
                            Self::action_principal(state, principal)?,
                            state,
                        )?)
                    } else {
                        self.action_cycles(action, state)?
                    }
                } else {
                    self.action_cycles(action, state)?
                };
                post_cycles = live_cycles;
                (
                    record.receipt.is_some()
                        && native_funding_applied(NativeFundingObservation {
                            amount: *amount,
                            expected_post_cycles: *expected_post_cycles,
                            funding_deficit_cycles: *funding_deficit_cycles,
                            funding_margin_cycles: *funding_margin_cycles,
                            live_cycles,
                            pre_cycles: record.pre_cycles,
                        }),
                    format!(
                        "native-topup:ledger-withdraw:{}:actual:{live_cycles:?}:expected:{expected_post_cycles}:margin:{funding_margin_cycles}",
                        record.receipt.as_deref().unwrap_or("pending"),
                    ),
                )
            }
            EnsureAction::FundEstate {
                amount,
                expected_post_cycles,
                ledger_fee_cycles,
                principal,
                ..
            } => {
                let source_cycles = self
                    .icp
                    .identity_cycles_balance()
                    .map_err(|error| IcpEnsurePlatformError::LedgerTransfer(error.to_string()))?;
                let target = Self::action_principal(state, principal)?;
                let destination_cycles = self.cycles_ledger_balance(target)?;
                post_cycles = Some(source_cycles);
                (
                    record.receipt.is_some()
                        && estate_funding_applied(EstateFundingObservation {
                            amount: *amount,
                            destination_after: destination_cycles,
                            destination_before: record.destination_pre_cycles,
                            expected_destination_after: *expected_post_cycles,
                            ledger_fee_cycles: *ledger_fee_cycles,
                            source_after: source_cycles,
                            source_before: record.pre_cycles,
                        }),
                    format!(
                        "estate-funding:ledger-transfer:{}:source:{source_cycles}:destination:{destination_cycles}:expected:{expected_post_cycles}",
                        record.receipt.as_deref().unwrap_or("pending"),
                    ),
                )
            }
            EnsureAction::Install {
                mode,
                principal,
                wasm_sha256,
                ..
            } => {
                let live =
                    self.install_status_optional(Self::action_principal(state, principal)?)?;
                let applied = live.as_ref().is_some_and(|live| {
                    install_effect_applied(
                        *mode,
                        wasm_sha256,
                        live.module_sha256.as_deref(),
                        record.pre_canister_version,
                        live.canister_version,
                    )
                });
                let progress_identity = live.as_ref().map_or_else(
                    || "install:missing".to_string(),
                    |live| {
                        format!(
                            "install:{:?}:version:{:?}",
                            live.module_sha256, live.canister_version
                        )
                    },
                );
                (applied, progress_identity)
            }
            EnsureAction::FleetProtocol {
                action: current_action,
                ..
            } => {
                let observation = match current_protocol::observe(&self.icp, &self.root, action) {
                    Ok(observation) => observation,
                    Err(error)
                        if matches!(
                            current_action.as_ref(),
                            crate::fleet_ensure::model::CurrentFleetProtocolAction::AdoptStore { .. }
                        ) && predecessor_store_control_rejection(&error) =>
                    {
                        if !self.retained_store_control_replan_is_exact(
                            operation_id,
                            action,
                            state,
                        )? {
                            return Err(error.into());
                        }
                        EffectObservation {
                            applied: false,
                            estate_funding_required: None,
                            post_cycles: None,
                            progress_identity: "store-adoption:replan-required:diagnostic:132"
                                .to_string(),
                            retry: EffectRetry::ReplanRequiredAfterRejectedPrerequisite,
                        }
                    }
                    Err(error)
                        if matches!(
                            current_action.as_ref(),
                            crate::fleet_ensure::model::CurrentFleetProtocolAction::AdoptStore { .. }
                        ) && recoverable_current_protocol_error(&error) =>
                    {
                        EffectObservation {
                            applied: false,
                            estate_funding_required: None,
                            post_cycles: None,
                            progress_identity: "store-adoption:protected-status-unavailable"
                                .to_string(),
                            retry: EffectRetry::None,
                        }
                    }
                    Err(error) => return Err(error.into()),
                };
                retry = observation.retry;
                (observation.applied, observation.progress_identity)
            }
            EnsureAction::Protocol { .. } => {
                let observation = protocol::observe(
                    &self.icp,
                    &self.root,
                    operation_id,
                    &self.protocol_principals(state),
                    &resolved_protocol_action(action, state)?,
                )?;
                (observation.applied, observation.progress_identity)
            }
            EnsureAction::SetControllers {
                controller_canisters,
                controllers,
                name,
                principal,
                ..
            } => {
                let expected =
                    self.resolved_controllers(state, controllers, controller_canisters)?;
                let principal = Self::action_principal(state, principal)?;
                let configured = self
                    .desired
                    .canisters
                    .iter()
                    .find(|configured| configured.name == *name);
                let mut observed_controllers = if let Some(configured) =
                    configured.filter(|configured| configured.kind == DesiredCanisterKind::Pool)
                {
                    self.inspect_root_owned_canister(configured, principal, state)?
                        .map(|response| {
                            response
                                .settings
                                .controllers
                                .into_iter()
                                .map(|controller| controller.to_text())
                                .collect::<Vec<_>>()
                        })
                } else {
                    self.status_optional(principal)?
                        .map(|live| live.controllers)
                };
                if let Some(controllers) = &mut observed_controllers {
                    controllers.sort();
                    controllers.dedup();
                }
                (
                    observed_controllers.as_ref() == Some(&expected),
                    format!("controllers:{observed_controllers:?}"),
                )
            }
            EnsureAction::Start { principal, .. } => {
                let live = self.status_optional(Self::action_principal(state, principal)?)?;
                (
                    live.as_ref()
                        .is_some_and(|live| live.status == CanisterRuntimeStatus::Running),
                    format!("start:{:?}", live.map(|live| live.status)),
                )
            }
            EnsureAction::Stop { principal, .. } => {
                let live = self.status_optional(Self::action_principal(state, principal)?)?;
                (
                    live.as_ref()
                        .is_some_and(|live| live.status == CanisterRuntimeStatus::Stopped),
                    format!("stop:{:?}", live.map(|live| live.status)),
                )
            }
            EnsureAction::Transfer {
                amount,
                maximum_execution_burn_cycles,
                name,
                principal,
                destination,
                ..
            } => {
                let source = self
                    .status_optional(Self::action_principal(state, principal)?)?
                    .map(|live| live.cycles)
                    .ok_or_else(|| IcpEnsurePlatformError::MissingTransferBalance {
                        canister: name.clone(),
                        side: "live source",
                    })?;
                let destination = self
                    .status_optional(self.current_principal(state, destination).ok_or_else(
                        || IcpEnsurePlatformError::UnresolvedCreated(destination.clone()),
                    )?)?
                    .map(|live| live.cycles)
                    .ok_or_else(|| IcpEnsurePlatformError::MissingTransferBalance {
                        canister: name.clone(),
                        side: "live treasury",
                    })?;
                let source_pre = record.pre_cycles.ok_or_else(|| {
                    IcpEnsurePlatformError::MissingTransferBalance {
                        canister: name.clone(),
                        side: "retained source",
                    }
                })?;
                let destination_pre = record.destination_pre_cycles.ok_or_else(|| {
                    IcpEnsurePlatformError::MissingTransferBalance {
                        canister: name.clone(),
                        side: "retained treasury",
                    }
                })?;
                let reconciliation = reconcile_retirement_transfer(RetirementTransferBalances {
                    destination_after: destination,
                    destination_before: destination_pre,
                    maximum_execution_burn: *maximum_execution_burn_cycles,
                    source_after: source,
                    source_before: source_pre,
                    transfer_amount: *amount,
                });
                let (applied, source_debit, destination_credit) = match reconciliation {
                    Ok(RetirementTransferReconciliation::Pending) => (false, 0, 0),
                    Ok(RetirementTransferReconciliation::Conserved {
                        destination_credit,
                        source_debit,
                    }) => (true, source_debit, destination_credit),
                    Err(RetirementTransferInvariantError::ArithmeticOverflow) => {
                        return Err(IcpEnsurePlatformError::Arithmetic(
                            "retirement transfer debit",
                        ));
                    }
                    Err(RetirementTransferInvariantError::BalanceMovedInInvalidDirection) => {
                        return Err(IcpEnsurePlatformError::InvalidTransferBalance {
                            canister: name.clone(),
                        });
                    }
                    Err(RetirementTransferInvariantError::Unbalanced {
                        destination_credit,
                        maximum_source_debit,
                        source_debit,
                    }) => {
                        return Err(IcpEnsurePlatformError::UnbalancedTransfer {
                            canister: name.clone(),
                            destination_credit,
                            expected: *amount,
                            maximum_source_debit,
                            source_debit,
                        });
                    }
                };
                (
                    applied,
                    format!("transfer:{source_debit}:{destination_credit}"),
                )
            }
        };
        Ok(EffectObservation {
            applied,
            estate_funding_required: None,
            post_cycles,
            progress_identity,
            retry,
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the platform keeps every approved single-step effect in one exhaustive match"
    )]
    fn apply(
        &mut self,
        operation_id: &str,
        action: &EnsureAction,
        _record: &EffectRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<EffectOutcome, Self::Error> {
        self.require_operator()?;
        match action {
            EnsureAction::Create {
                controller_canisters,
                controllers,
                created_at_time,
                ledger,
                requested_initial_cycles,
                subnet,
                ..
            } => self.apply_create(
                CreateCanisterAuthority {
                    controller_canisters,
                    controllers,
                    created_at_time: *created_at_time,
                    ledger,
                    requested_initial_cycles: *requested_initial_cycles,
                    subnet,
                },
                state,
            ),
            EnsureAction::Delete {
                maximum_remaining_cycles,
                principal,
                ..
            } => {
                let principal = Self::action_principal(state, principal)?;
                if let Some(live) = self.status_optional(principal)? {
                    if live.cycles > *maximum_remaining_cycles {
                        return Err(IcpEnsurePlatformError::MaterialCycles {
                            actual: live.cycles,
                            canister: principal.to_string(),
                            maximum: *maximum_remaining_cycles,
                        });
                    }
                    if live.status != CanisterRuntimeStatus::Stopped {
                        return Err(IcpEnsurePlatformError::NotStopped {
                            canister: principal.to_string(),
                        });
                    }
                    self.icp.delete_canister_without_cycle_recovery(principal)?;
                }
                Ok(empty_outcome())
            }
            EnsureAction::Fund {
                amount,
                created_at_time,
                ledger,
                principal,
                ..
            } => self.apply_fund(
                *amount,
                *created_at_time,
                ledger,
                Self::action_principal(state, principal)?,
            ),
            EnsureAction::FundEstate {
                amount,
                created_at_time,
                ledger,
                principal,
                ..
            } => self.apply_estate_fund(
                *amount,
                *created_at_time,
                ledger,
                Self::action_principal(state, principal)?,
            ),
            EnsureAction::Install {
                canic_init,
                init_arg,
                init_arg_sha256,
                init_candid,
                init_candid_sha256,
                mode,
                principal,
                wasm,
                wasm_sha256,
                ..
            } => self.apply_install(
                operation_id,
                &self.protocol_principals(state),
                canic_init.as_ref(),
                init_arg.as_deref(),
                init_arg_sha256.as_deref(),
                init_candid.as_deref(),
                init_candid_sha256.as_deref(),
                *mode,
                Self::action_principal(state, principal)?,
                wasm,
                wasm_sha256,
            ),
            EnsureAction::FleetProtocol { .. } => {
                current_protocol::apply(&self.icp, &self.root, action).map_err(Into::into)
            }
            EnsureAction::Protocol { .. } => {
                let action = resolved_protocol_action(action, state)?;
                protocol::apply(
                    &self.icp,
                    &self.root,
                    operation_id,
                    &self.protocol_principals(state),
                    &action,
                )?;
                Ok(EffectOutcome {
                    created_principal: None,
                    post_cycles: None,
                    receipt: Some(operation_id.to_string()),
                })
            }
            EnsureAction::SetControllers {
                controller_canisters,
                controllers,
                principal,
                ..
            } => self.apply_controllers(
                Self::action_principal(state, principal)?,
                &self.resolved_controllers(state, controllers, controller_canisters)?,
            ),
            EnsureAction::Start { principal, .. } => {
                self.icp
                    .start_canister(Self::action_principal(state, principal)?)?;
                Ok(empty_outcome())
            }
            EnsureAction::Stop { principal, .. } => {
                self.icp
                    .stop_canister(Self::action_principal(state, principal)?)?;
                Ok(empty_outcome())
            }
            EnsureAction::Transfer {
                amount,
                candid,
                candid_sha256,
                destination,
                method,
                principal,
                ..
            } => self.apply_transfer(
                *amount,
                candid,
                candid_sha256,
                self.current_principal(state, destination).ok_or_else(|| {
                    IcpEnsurePlatformError::UnresolvedCreated(destination.clone())
                })?,
                method,
                operation_id,
                Self::action_principal(state, principal)?,
            ),
        }
    }

    fn action_cycles(
        &mut self,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        if matches!(action, EnsureAction::FundEstate { .. }) {
            return self
                .icp
                .identity_cycles_balance()
                .map(Some)
                .map_err(|error| IcpEnsurePlatformError::LedgerTransfer(error.to_string()));
        }
        let (name, principal) = match action {
            EnsureAction::Create { .. } => return Ok(None),
            EnsureAction::Delete {
                name, principal, ..
            }
            | EnsureAction::FleetProtocol {
                name, principal, ..
            }
            | EnsureAction::Fund {
                name, principal, ..
            }
            | EnsureAction::FundEstate {
                name, principal, ..
            }
            | EnsureAction::Install {
                name, principal, ..
            }
            | EnsureAction::Protocol {
                name, principal, ..
            }
            | EnsureAction::SetControllers {
                name, principal, ..
            }
            | EnsureAction::Start {
                name, principal, ..
            }
            | EnsureAction::Stop {
                name, principal, ..
            }
            | EnsureAction::Transfer {
                name, principal, ..
            } => (name, Self::action_principal(state, principal)?),
        };
        let observed = self
            .desired
            .canisters
            .iter()
            .find(|configured| configured.name == *name)
            .map_or_else(
                || self.status_optional(principal),
                |configured| self.observe_configured_canister(configured, principal, state),
            )?;
        Ok(observed.map(|live| live.cycles))
    }

    fn action_destination_cycles(
        &mut self,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        if let EnsureAction::FundEstate { principal, .. } = action {
            let owner = Self::action_principal(state, principal)?;
            return self.cycles_ledger_balance(owner).map(Some);
        }
        let EnsureAction::Transfer { destination, .. } = action else {
            return Ok(None);
        };
        let destination = self
            .current_principal(state, destination)
            .ok_or_else(|| IcpEnsurePlatformError::UnresolvedCreated(destination.clone()))?;
        Ok(self.status_optional(destination)?.map(|live| live.cycles))
    }

    fn action_canister_version(
        &mut self,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<u64>, Self::Error> {
        let EnsureAction::Install { principal, .. } = action else {
            return Ok(None);
        };
        let principal = Self::action_principal(state, principal)?;
        let version = self
            .install_status_optional(principal)?
            .and_then(|live| live.canister_version)
            .ok_or_else(|| IcpEnsurePlatformError::IncompleteCanisterStatus {
                canister: principal.to_string(),
                field: "canister_version",
            })?;
        Ok(Some(version))
    }
}

fn exact_install_canister_status(
    icp: &IcpCli,
    canister: &str,
    projected_canister_version: Option<u64>,
    projected_module_sha256: Option<String>,
) -> Result<ExactInstallCanisterStatus, IcpEnsurePlatformError> {
    exact_install_canister_status_with(
        canister,
        projected_canister_version,
        projected_module_sha256,
        |canister_id| {
            icp.management_canister_status_candid::<_, ManagementCanisterStatusResponse>(
                canister_id,
                &ManagementCanisterStatusRequest { canister_id },
            )
        },
    )
}

fn exact_install_canister_status_with(
    canister: &str,
    projected_canister_version: Option<u64>,
    projected_module_sha256: Option<String>,
    management_status: impl FnOnce(
        Principal,
    )
        -> Result<ManagementCanisterStatusResponse, IcpManagementCallError>,
) -> Result<ExactInstallCanisterStatus, IcpEnsurePlatformError> {
    if let Some(canister_version) = projected_canister_version {
        return Ok(ExactInstallCanisterStatus {
            canister_version,
            module_sha256: projected_module_sha256,
        });
    }
    let canister_id = parse_principal("install target", canister)?;
    let response = management_status(canister_id).map_err(|source| {
        IcpEnsurePlatformError::InstallVersionProofUnavailable {
            canister: canister.to_string(),
            source: Box::new(source),
        }
    })?;
    Ok(ExactInstallCanisterStatus {
        canister_version: response.version,
        module_sha256: response
            .module_hash
            .map(|hash| canic_core::cdk::utils::hash::hex_bytes(&hash)),
    })
}

fn exact_retained_root_owned_topology<'a>(
    state: &'a FleetEnsureStateRecord,
    name: &str,
    kind: DesiredCanisterKind,
    parent: &str,
    principal: &str,
    root: &str,
) -> Option<&'a crate::fleet_ensure::model::FleetEnsureTopologyRecord> {
    let child_identity_matches = retained_state_principal_matches(state, name, principal);
    let root_identity_matches = retained_state_principal_matches(state, parent, root);
    let topology = state
        .topology
        .get(name)
        .filter(|topology| topology.kind == kind && topology.parent.as_deref() == Some(parent));
    (child_identity_matches && root_identity_matches)
        .then_some(topology)
        .flatten()
}

fn retained_state_principal_matches(
    state: &FleetEnsureStateRecord,
    name: &str,
    principal: &str,
) -> bool {
    let pending = state.pending_principals.get(name).map(String::as_str);
    let terminal = state.principals.get(name).map(String::as_str);
    let retained = pending.is_some() || terminal.is_some();
    retained
        && pending.is_none_or(|retained| retained == principal)
        && terminal.is_none_or(|retained| retained == principal)
}

fn completed_reinstall_continuity(
    pre_canister_version: u64,
    principal_matches: bool,
    topology_matches: bool,
    live_module_sha256: Option<&str>,
    desired_module_sha256: &str,
    live_canister_version: Option<u64>,
) -> Result<(), &'static str> {
    if !principal_matches {
        return Err("Principal");
    }
    if !topology_matches {
        return Err("Root/parent/kind topology");
    }
    if live_module_sha256 != Some(desired_module_sha256) {
        return Err("module SHA-256");
    }
    if live_canister_version.is_some_and(|version| version <= pre_canister_version) {
        return Err("canister version");
    }
    Ok(())
}

fn recoverable_root_status_error(error: &CanisterProtocolError) -> bool {
    error.is_rejected_with(canic_core::diagnostics::codes::STATE_CONFLICT)
        || error.is_rejected_with(canic_core::diagnostics::codes::STATE_UNAVAILABLE)
}

fn recoverable_current_protocol_error(error: &current_protocol::CurrentProtocolError) -> bool {
    matches!(
        error,
        current_protocol::CurrentProtocolError::Transport(source)
            if recoverable_root_status_error(source)
    )
}

fn predecessor_store_control_rejection(error: &current_protocol::CurrentProtocolError) -> bool {
    matches!(
        error,
        current_protocol::CurrentProtocolError::Transport(source)
            if source.is_rejected_with(canic_core::diagnostics::codes::STATE_CONFLICT)
    )
}

pub fn install_effect_applied(
    mode: InstallMode,
    expected_hash: &str,
    live_hash: Option<&str>,
    pre_canister_version: Option<u64>,
    live_canister_version: Option<u64>,
) -> bool {
    if live_hash != Some(expected_hash) {
        return false;
    }
    match mode {
        InstallMode::Install => true,
        InstallMode::Reinstall => pre_canister_version
            .zip(live_canister_version)
            .is_some_and(|(before, after)| after > before),
    }
}

/// Exact retained and live evidence for one Cycles Ledger withdrawal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeFundingObservation {
    pub amount: u128,
    pub expected_post_cycles: u128,
    pub funding_deficit_cycles: u128,
    pub funding_margin_cycles: u128,
    pub live_cycles: Option<u128>,
    pub pre_cycles: Option<u128>,
}

pub const fn native_funding_applied(observation: NativeFundingObservation) -> bool {
    let Some(pre_cycles) = observation.pre_cycles else {
        return false;
    };
    let Some(live_cycles) = observation.live_cycles else {
        return false;
    };
    let Some(expected_from_amount) = pre_cycles.checked_add(observation.amount) else {
        return false;
    };
    let Some(minimum_live_cycles) = observation
        .expected_post_cycles
        .checked_sub(observation.funding_margin_cycles)
    else {
        return false;
    };
    let Some(minimum_from_deficit) = pre_cycles.checked_add(observation.funding_deficit_cycles)
    else {
        return false;
    };
    observation.funding_deficit_cycles > 0
        && expected_from_amount == observation.expected_post_cycles
        && minimum_live_cycles == minimum_from_deficit
        && minimum_live_cycles > pre_cycles
        && live_cycles >= minimum_live_cycles
}

/// Exact retained and live evidence for one operator-to-Root Ledger transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EstateFundingObservation {
    pub amount: u128,
    pub destination_after: u128,
    pub destination_before: Option<u128>,
    pub expected_destination_after: u128,
    pub ledger_fee_cycles: u128,
    pub source_after: u128,
    pub source_before: Option<u128>,
}

/// Require both sides of an estate-funding transfer to match its reviewed debit.
#[must_use]
pub const fn estate_funding_applied(observation: EstateFundingObservation) -> bool {
    let Some(source_before) = observation.source_before else {
        return false;
    };
    let Some(destination_before) = observation.destination_before else {
        return false;
    };
    let Some(expected_source_debit) = observation
        .amount
        .checked_add(observation.ledger_fee_cycles)
    else {
        return false;
    };
    let Some(source_debit) = source_before.checked_sub(observation.source_after) else {
        return false;
    };
    let Some(destination_credit) = observation
        .destination_after
        .checked_sub(destination_before)
    else {
        return false;
    };
    source_debit == expected_source_debit
        && destination_credit == observation.amount
        && observation.destination_after == observation.expected_destination_after
}

fn validate_root_funding_inspection(
    canister: &str,
    kind: DesiredCanisterKind,
    root: &str,
    controllers: &[Principal],
    module_hash: Option<&[u8]>,
    cycles: &Nat,
) -> Result<u128, IcpEnsurePlatformError> {
    if controllers.len() != 1 || controllers[0].to_text() != root {
        return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
            canister: canister.to_string(),
            field: "Root-only controllers",
        });
    }
    if kind == DesiredCanisterKind::Pool && module_hash.is_some() {
        return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
            canister: canister.to_string(),
            field: "module-free pool asset",
        });
    }
    u128::try_from(cycles.0.clone()).map_err(|_| IcpEnsurePlatformError::InvalidStatusCycles {
        canister: canister.to_string(),
        value: cycles.to_string(),
    })
}

fn created_canister_outcome(
    canister_id: Principal,
    receipt: String,
    requested_initial_cycles: u128,
) -> EffectOutcome {
    EffectOutcome {
        created_principal: Some(canister_id.to_text()),
        post_cycles: Some(requested_initial_cycles),
        receipt: Some(receipt),
    }
}

fn create_canister_response_outcome(
    response: Result<CreateCanisterSuccess, CreateCanisterError>,
    requested_initial_cycles: u128,
) -> Result<EffectOutcome, IcpEnsurePlatformError> {
    match response {
        Ok(success) => Ok(created_canister_outcome(
            success.canister_id,
            success.block_id.to_string(),
            requested_initial_cycles,
        )),
        Err(CreateCanisterError::Duplicate {
            duplicate_of,
            canister_id: Some(canister_id),
        }) => Ok(created_canister_outcome(
            canister_id,
            duplicate_of.to_string(),
            requested_initial_cycles,
        )),
        Err(CreateCanisterError::Duplicate {
            canister_id: None, ..
        }) => Err(IcpEnsurePlatformError::LedgerCreatePending),
        Err(error) => Err(IcpEnsurePlatformError::LedgerCreate(render_create_error(
            error,
        ))),
    }
}

fn ledger_fee_cycles(value: Nat) -> Result<u128, IcpEnsurePlatformError> {
    let rendered = value.to_string();
    u128::try_from(value.0).map_err(|_| IcpEnsurePlatformError::InvalidLedgerFee(rendered))
}

const fn empty_outcome() -> EffectOutcome {
    EffectOutcome {
        created_principal: None,
        post_cycles: None,
        receipt: None,
    }
}

fn parse_status_cycles(value: &str) -> Option<u128> {
    value.replace('_', "").trim().parse().ok()
}

fn normalize_hash(value: &str) -> String {
    value
        .strip_prefix("0x")
        .unwrap_or(value)
        .to_ascii_lowercase()
}

fn parse_principal(field: &'static str, value: &str) -> Result<Principal, IcpEnsurePlatformError> {
    Principal::from_text(value).map_err(|_| IcpEnsurePlatformError::InvalidPrincipal {
        field,
        value: value.to_string(),
    })
}

fn reject_public_status_projection(
    report: &IcpCanisterStatusReport,
    principal: &str,
) -> Result<(), IcpEnsurePlatformError> {
    if report.status.is_none()
        && report.settings.is_none()
        && report.cycles.is_none()
        && report.public_controllers.is_some()
    {
        return Err(IcpEnsurePlatformError::PublicCanisterStatusUnavailable {
            canister: principal.to_string(),
        });
    }
    Ok(())
}

fn resolve_path(root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn verify_artifact(
    path: &Path,
    expected: &str,
    kind: &'static str,
) -> Result<(), IcpEnsurePlatformError> {
    let bytes = std::fs::read(path)
        .map_err(|_| IcpEnsurePlatformError::ArtifactUnavailable(path.to_path_buf()))?;
    let actual = canic_core::cdk::utils::hash::sha256_hex(&bytes);
    if actual != expected {
        return Err(IcpEnsurePlatformError::ArtifactDigestMismatch {
            actual,
            expected: expected.to_string(),
            kind,
        });
    }
    Ok(())
}

fn artifact_hash(path: &Path) -> Result<String, IcpEnsurePlatformError> {
    let bytes = std::fs::read(path)
        .map_err(|_| IcpEnsurePlatformError::ArtifactUnavailable(path.to_path_buf()))?;
    Ok(canic_core::cdk::utils::hash::sha256_hex(&bytes))
}

fn resolved_protocol_action(
    action: &EnsureAction,
    state: &FleetEnsureStateRecord,
) -> Result<EnsureAction, IcpEnsurePlatformError> {
    let mut resolved = action.clone();
    if let EnsureAction::Protocol { principal, .. } = &mut resolved {
        *principal = IcpEnsurePlatform::action_principal(state, principal)?.to_string();
        return Ok(resolved);
    }
    Err(IcpEnsurePlatformError::Protocol(
        protocol::ProtocolEffectError::WrongAction,
    ))
}

fn render_create_error(error: CreateCanisterError) -> String {
    match error {
        CreateCanisterError::CreatedInFuture { ledger_time } => {
            format!("created in future of ledger time {ledger_time}")
        }
        CreateCanisterError::Duplicate { duplicate_of, .. } => {
            format!("duplicate block {duplicate_of}")
        }
        CreateCanisterError::FailedToCreate {
            error,
            fee_block,
            refund_block,
        } => format!("failed: {error}; fee_block={fee_block:?}; refund_block={refund_block:?}"),
        CreateCanisterError::GenericError {
            error_code,
            message,
        } => format!("error {error_code}: {message}"),
        CreateCanisterError::InsufficientFunds { balance } => {
            format!("insufficient funds: balance={balance}")
        }
        CreateCanisterError::TemporarilyUnavailable => "temporarily unavailable".to_string(),
        CreateCanisterError::TooOld => "request is too old".to_string(),
    }
}

fn render_withdraw_error(error: WithdrawError) -> String {
    match error {
        WithdrawError::BadFee { expected_fee } => format!("bad fee; expected {expected_fee}"),
        WithdrawError::CreatedInFuture { ledger_time } => {
            format!("created in future of ledger time {ledger_time}")
        }
        WithdrawError::Duplicate { duplicate_of } => format!("duplicate block {duplicate_of}"),
        WithdrawError::FailedToWithdraw {
            fee_block,
            rejection_reason,
            ..
        } => format!("withdraw failed: {rejection_reason}; fee_block={fee_block:?}"),
        WithdrawError::GenericError {
            error_code,
            message,
        } => format!("error {error_code}: {message}"),
        WithdrawError::InsufficientFunds { balance } => {
            format!("insufficient funds: balance={balance}")
        }
        WithdrawError::InvalidReceiver { receiver } => format!("invalid receiver {receiver}"),
        WithdrawError::TemporarilyUnavailable => "temporarily unavailable".to_string(),
        WithdrawError::TooOld => "request is too old".to_string(),
    }
}

fn render_ledger_transfer_error(error: CyclesLedgerTransferError) -> String {
    match error {
        CyclesLedgerTransferError::BadBurn { min_burn_amount } => {
            format!("bad burn; minimum {min_burn_amount}")
        }
        CyclesLedgerTransferError::BadFee { expected_fee } => {
            format!("bad fee; expected {expected_fee}")
        }
        CyclesLedgerTransferError::CreatedInFuture { ledger_time } => {
            format!("created in future of ledger time {ledger_time}")
        }
        CyclesLedgerTransferError::Duplicate { duplicate_of } => {
            format!("duplicate block {duplicate_of}")
        }
        CyclesLedgerTransferError::GenericError {
            error_code,
            message,
        } => format!("error {error_code}: {message}"),
        CyclesLedgerTransferError::InsufficientFunds { balance } => {
            format!("insufficient funds: balance={balance}")
        }
        CyclesLedgerTransferError::TemporarilyUnavailable => "temporarily unavailable".to_string(),
        CyclesLedgerTransferError::TooOld => "request is too old".to_string(),
    }
}

#[expect(
    dead_code,
    reason = "Candid variant is retained for exact response decoding"
)]
const fn rejection_code_name(code: RejectionCode) -> &'static str {
    match code {
        RejectionCode::CanisterError => "canister_error",
        RejectionCode::CanisterReject => "canister_reject",
        RejectionCode::DestinationInvalid => "destination_invalid",
        RejectionCode::NoError => "no_error",
        RejectionCode::SysFatal => "sys_fatal",
        RejectionCode::SysTransient => "sys_transient",
        RejectionCode::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::dto::pool::CanisterPoolAssetStatus;

    #[test]
    fn protocol_observation_delay_uses_bounded_exponential_backoff() {
        let production_cap = MAXIMUM_PROTOCOL_OBSERVATION_DELAY;
        assert_eq!(
            protocol_observation_delay(0, INITIAL_PROTOCOL_OBSERVATION_DELAY, production_cap),
            Duration::from_millis(250)
        );
        assert_eq!(
            protocol_observation_delay(1, INITIAL_PROTOCOL_OBSERVATION_DELAY, production_cap),
            Duration::from_millis(250)
        );
        assert_eq!(
            protocol_observation_delay(2, INITIAL_PROTOCOL_OBSERVATION_DELAY, production_cap),
            Duration::from_millis(500)
        );
        assert_eq!(
            protocol_observation_delay(5, INITIAL_PROTOCOL_OBSERVATION_DELAY, production_cap),
            Duration::from_secs(4)
        );
        assert_eq!(
            protocol_observation_delay(6, INITIAL_PROTOCOL_OBSERVATION_DELAY, production_cap),
            Duration::from_secs(5)
        );
        assert_eq!(
            protocol_observation_delay(
                u32::MAX,
                INITIAL_PROTOCOL_OBSERVATION_DELAY,
                production_cap
            ),
            Duration::from_secs(5)
        );
        assert_eq!(
            protocol_observation_delay(u32::MAX, Duration::from_secs(1), Duration::from_secs(1)),
            Duration::from_secs(1)
        );
    }

    #[cfg(unix)]
    #[test]
    fn icp_1_3_public_non_controller_status_is_typed_unavailable_evidence() {
        use std::{fs, os::unix::fs::PermissionsExt};

        let canister = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let root = crate::test_support::temp_dir("canic-public-status-projection");
        fs::create_dir_all(&root).expect("create public status fixture");
        let executable = root.join("icp");
        let commands = root.join("commands.log");
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo 'icp 1.3.0'; exit 0; fi\n\
             printf '%s\\n' \"$*\" >> '{}'\n\
             printf '%s\\n' '{{\"id\":\"{canister}\",\"controllers\":[\"rdmx6-jaaaa-aaaaa-aaadq-cai\"],\"module_hash\":null}}'\n",
            commands.display(),
        );
        fs::write(&executable, script).expect("write fake ICP 1.3.0");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("make fake ICP executable");
        let desired = DesiredFleet {
            bootstrap: None,
            canisters: Vec::new(),
            cycles_ledger: "um5iw-rqaaa-aaaaq-qaaba-cai".to_string(),
            environment: "local".to_string(),
            fleet: "public-status".to_string(),
            ledger_fee_cycles: "100M".to_string(),
            management_creation_fee_cycles: "500B".to_string(),
            material_cycle_threshold: "1B".to_string(),
            maximum_observation_burn_cycles: "1B".to_string(),
            maximum_stalled_observations: 4,
            maximum_update_burn_cycles: "1B".to_string(),
            operator: "rdmx6-jaaaa-aaaaa-aaadq-cai".to_string(),
            protocol: None,
            protocol_steps: Vec::new(),
            schema_version: 1,
            treasury: "treasury".to_string(),
        };
        let platform =
            IcpEnsurePlatform::new(desired, executable.to_string_lossy().as_ref(), &root);

        assert!(matches!(
            platform.status_optional(canister),
            Err(IcpEnsurePlatformError::PublicCanisterStatusUnavailable {
                canister: unavailable,
            }) if unavailable == canister
        ));
        assert!(
            fs::read_to_string(commands)
                .expect("read fake ICP commands")
                .contains("canister status rrkah-fqaaa-aaaaa-aaaaq-cai --json")
        );
    }

    #[test]
    fn retained_predecessor_modules_are_observed_and_optionally_cross_checked() {
        let root_controllers = vec!["root-controller".to_string()];
        let store_controllers = vec!["root-controller".to_string(), "operator".to_string()];
        let observed = RetainedStoreControlLiveBinding {
            root_controllers: root_controllers.clone(),
            root_module_sha256: Some("root-predecessor".to_string()),
            store_controllers: store_controllers.clone(),
            store_module_sha256: Some("store-predecessor".to_string()),
        };

        assert!(retained_store_control_live_is_exact(
            &observed,
            &root_controllers,
            &store_controllers,
            None,
            None,
            "root-successor",
            "store-successor",
        ));
        assert!(retained_store_control_live_is_exact(
            &observed,
            &root_controllers,
            &store_controllers,
            Some("root-predecessor"),
            Some("store-predecessor"),
            "root-successor",
            "store-successor",
        ));
        assert!(!retained_store_control_live_is_exact(
            &observed,
            &root_controllers,
            &store_controllers,
            Some("wrong-root-predecessor"),
            Some("store-predecessor"),
            "root-successor",
            "store-successor",
        ));

        let missing_live_module = RetainedStoreControlLiveBinding {
            root_controllers: root_controllers.clone(),
            root_module_sha256: None,
            store_controllers: store_controllers.clone(),
            store_module_sha256: Some("store-predecessor".to_string()),
        };
        assert!(!retained_store_control_live_is_exact(
            &missing_live_module,
            &root_controllers,
            &store_controllers,
            None,
            None,
            "root-successor",
            "store-successor",
        ));

        let mut successor_is_already_live = observed;
        successor_is_already_live.store_module_sha256 = Some("store-successor".to_string());
        assert!(!retained_store_control_live_is_exact(
            &successor_is_already_live,
            &root_controllers,
            &store_controllers,
            None,
            None,
            "root-successor",
            "store-successor",
        ));
    }

    #[cfg(unix)]
    #[test]
    fn versionless_icp_status_uses_exact_typed_management_version() {
        #[derive(CandidType)]
        struct CanonicalManagementCanisterStatusFixture {
            version: u64,
            module_hash: Option<Vec<u8>>,
        }

        use std::{fs, os::unix::fs::PermissionsExt};

        let root = crate::test_support::temp_dir("canic-install-version-fallback");
        fs::create_dir_all(&root).expect("create version fallback fixture");
        let executable = root.join("icp");
        let commands = root.join("commands.log");
        let canister = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let status_json = format!(
            r#"{{"id":"{canister}","name":"coordinator","status":"Running","settings":{{"controllers":["rdmx6-jaaaa-aaaaa-aaadq-cai"]}},"module_hash":"0x{}","memory_size":"1","cycles":"1000000000000","query_stats":{{}}}}"#,
            "11".repeat(32),
        );
        let response_bytes = candid::encode_one(CanonicalManagementCanisterStatusFixture {
            version: 42,
            module_hash: Some(vec![0x22; 32]),
        })
        .expect("encode independently modelled management status");
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then printf '%s\\n' 'icp 1.3.0'; exit 0; fi\n\
             printf '%s\\n' \"$*\" >> '{}'\n\
             case \"$*\" in\n\
               *\"canister status {canister}\"*) printf '%s\\n' '{}' ;;\n\
               *) printf '%s\\n' 'unexpected fake ICP command' >&2; exit 23 ;;\n\
             esac\n",
            commands.display(),
            status_json,
        );
        fs::write(&executable, script).expect("write fake ICP 1.3.0");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("make fake ICP executable");
        let icp = IcpCli::new(executable.to_string_lossy(), Some("ic".to_string()));

        let projected = icp
            .canister_status_report(canister)
            .expect("read ICP 1.3.0 status projection");
        assert_eq!(projected.canister_version, None);
        let exact = exact_install_canister_status_with(
            canister,
            projected.canister_version,
            projected.module_hash.map(|hash| normalize_hash(&hash)),
            |effective_canister_id| {
                assert_eq!(
                    effective_canister_id,
                    canister.parse().expect("target Principal")
                );
                candid::decode_one(&response_bytes).map_err(IcpManagementCallError::CandidResponse)
            },
        )
        .expect("obtain exact typed management status");
        assert_eq!(exact.canister_version, 42);
        assert_eq!(exact.module_sha256, Some("22".repeat(32)));
        let commands = fs::read_to_string(&commands).expect("read fake ICP commands");
        assert!(commands.contains("canister status rrkah-fqaaa-aaaaa-aaaaq-cai"));
        assert!(!commands.contains("canister call aaaaa-aa canister_status"));

        fs::write(
            &executable,
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'icp 1.3.0'; exit 0; fi\necho '{\"not_response_bytes\":true}'\n",
        )
        .expect("replace unavailable management fixture");
        let error = exact_install_canister_status_with(canister, None, None, |_| {
            Err(IcpManagementCallError::MissingEnvironment)
        })
        .expect_err("missing projected and typed version must fail closed");
        assert!(matches!(
            error,
            IcpEnsurePlatformError::InstallVersionProofUnavailable {
                canister: observed_canister,
                source,
            } if observed_canister == canister
                && matches!(source.as_ref(), IcpManagementCallError::MissingEnvironment)
        ));

        fs::remove_dir_all(root).expect("remove version fallback fixture");
    }

    #[test]
    fn root_owned_observation_classifies_bootstrap_and_workload_lifecycle() {
        assert_eq!(
            root_owned_lifecycle(DesiredCanisterKind::Store, &CanisterPoolAssetStatus::Store),
            Some(RootOwnedCanisterLifecycle::Store)
        );
        assert_eq!(
            root_owned_lifecycle(DesiredCanisterKind::Pool, &CanisterPoolAssetStatus::Ready),
            Some(RootOwnedCanisterLifecycle::Idle)
        );
        assert_eq!(
            root_owned_lifecycle(
                DesiredCanisterKind::Pool,
                &CanisterPoolAssetStatus::Claimed {
                    claim: canic_core::dto::pool::CanisterPoolClaim {
                        component: canic_core::ids::ComponentInstanceId::from_generated_bytes(
                            [1; 32]
                        ),
                        operation_id: [2; 32],
                    },
                }
            ),
            Some(RootOwnedCanisterLifecycle::Claimed)
        );
        assert_eq!(
            root_owned_lifecycle(
                DesiredCanisterKind::Pool,
                &CanisterPoolAssetStatus::Workload {
                    claim: canic_core::dto::pool::CanisterPoolClaim {
                        component: canic_core::ids::ComponentInstanceId::from_generated_bytes(
                            [1; 32]
                        ),
                        operation_id: [2; 32],
                    },
                }
            ),
            Some(RootOwnedCanisterLifecycle::Workload)
        );
        assert_eq!(
            root_owned_lifecycle(
                DesiredCanisterKind::Pool,
                &CanisterPoolAssetStatus::HandingOff {
                    recipient: Principal::anonymous(),
                }
            ),
            None
        );
        assert_eq!(
            root_owned_lifecycle(
                DesiredCanisterKind::Store,
                &CanisterPoolAssetStatus::PendingReset
            ),
            None
        );
    }

    #[test]
    fn estate_pool_inventory_reconciles_every_lifecycle_and_declared_total() {
        let claim = canic_core::dto::pool::CanisterPoolClaim {
            component: canic_core::ids::ComponentInstanceId::from_generated_bytes([1; 32]),
            operation_id: [2; 32],
        };
        let statuses = [
            CanisterPoolAssetStatus::Store,
            CanisterPoolAssetStatus::StoreDeletionPending {
                operation_id: [3; 32],
            },
            CanisterPoolAssetStatus::PendingReset,
            CanisterPoolAssetStatus::Ready,
            CanisterPoolAssetStatus::Claimed {
                claim: claim.clone(),
            },
            CanisterPoolAssetStatus::Workload {
                claim: claim.clone(),
            },
            CanisterPoolAssetStatus::Recycling {
                claim,
                reset: canic_core::dto::pool::CanisterPoolRecycleReset::Pending,
            },
            CanisterPoolAssetStatus::HandingOff {
                recipient: Principal::anonymous(),
            },
            CanisterPoolAssetStatus::Failed {
                reason: "retained failure".to_string(),
            },
        ];
        let mut observed = EstatePoolLifecycleCounts::default();
        for status in &statuses {
            observed.observe(status).expect("bounded lifecycle count");
        }
        let authority = EstatePoolPageAuthority {
            completed_handoffs: 0,
            counts: observed,
            pending_handoff: None,
            pooled: 4,
            surplus: 1,
        };

        assert!(authority.matches_complete_inventory(observed, 3));
        let mut incomplete = observed;
        incomplete.failed -= 1;
        incomplete.tracked -= 1;
        assert!(!authority.matches_complete_inventory(incomplete, 3));

        assert_eq!(estate_pool_lifecycle(&statuses[0]), None);
        assert_eq!(estate_pool_lifecycle(&statuses[1]), None);
        assert_eq!(
            statuses[2..]
                .iter()
                .map(estate_pool_lifecycle)
                .collect::<Vec<_>>(),
            vec![
                Some(EstatePoolAssetLifecycle::PendingReset),
                Some(EstatePoolAssetLifecycle::Ready),
                Some(EstatePoolAssetLifecycle::Claimed),
                Some(EstatePoolAssetLifecycle::Workload),
                Some(EstatePoolAssetLifecycle::Recycling),
                Some(EstatePoolAssetLifecycle::HandingOff),
                Some(EstatePoolAssetLifecycle::Failed),
            ]
        );
    }

    #[test]
    fn estate_funding_requires_exact_source_debit_and_destination_credit() {
        let exact = EstateFundingObservation {
            amount: 60,
            destination_after: 100,
            destination_before: Some(40),
            expected_destination_after: 100,
            ledger_fee_cycles: 5,
            source_after: 135,
            source_before: Some(200),
        };
        assert!(estate_funding_applied(exact));

        for drifted in [
            EstateFundingObservation {
                source_after: 136,
                ..exact
            },
            EstateFundingObservation {
                destination_after: 99,
                ..exact
            },
            EstateFundingObservation {
                source_before: None,
                ..exact
            },
            EstateFundingObservation {
                destination_before: None,
                ..exact
            },
        ] {
            assert!(!estate_funding_applied(drifted));
        }
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "the binding regression mutates each authority field independently"
    )]
    fn retained_root_owned_topology_requires_exact_child_root_and_parent_binding() {
        let mut state = FleetEnsureStateRecord {
            active_registry: None,
            completed_reinstall_action_sha256: BTreeMap::new(),
            completed_reinstall_operation_id: None,
            completed_reinstalls: BTreeMap::new(),
            fleet: "fleet".to_string(),
            pending_principals: BTreeMap::new(),
            principals: BTreeMap::from([
                ("root".to_string(), "root-principal".to_string()),
                ("store".to_string(), "store-principal".to_string()),
            ]),
            retained_cycles_by_principal: BTreeMap::new(),
            schema_version: crate::fleet_ensure::model::FLEET_ENSURE_SCHEMA_VERSION,
            topology: BTreeMap::from([(
                "store".to_string(),
                crate::fleet_ensure::model::FleetEnsureTopologyRecord {
                    kind: DesiredCanisterKind::Store,
                    module_hash: Some("11".repeat(32)),
                    parent: Some("root".to_string()),
                    protocol_binding: None,
                    role: None,
                },
            )]),
        };

        assert!(
            exact_retained_root_owned_topology(
                &state,
                "store",
                DesiredCanisterKind::Store,
                "root",
                "store-principal",
                "root-principal",
            )
            .is_some()
        );
        assert!(
            exact_retained_root_owned_topology(
                &state,
                "store",
                DesiredCanisterKind::Store,
                "root",
                "foreign-store",
                "root-principal",
            )
            .is_none()
        );
        state
            .principals
            .insert("store".to_string(), "store-principal".to_string());
        state
            .principals
            .insert("root".to_string(), "foreign-root".to_string());
        assert!(
            exact_retained_root_owned_topology(
                &state,
                "store",
                DesiredCanisterKind::Store,
                "root",
                "store-principal",
                "root-principal",
            )
            .is_none()
        );
        state
            .principals
            .insert("root".to_string(), "root-principal".to_string());
        state
            .topology
            .get_mut("store")
            .expect("Store topology")
            .kind = DesiredCanisterKind::Pool;
        assert!(
            exact_retained_root_owned_topology(
                &state,
                "store",
                DesiredCanisterKind::Store,
                "root",
                "store-principal",
                "root-principal",
            )
            .is_none()
        );
        state
            .topology
            .get_mut("store")
            .expect("Store topology")
            .kind = DesiredCanisterKind::Store;
        state
            .topology
            .get_mut("store")
            .expect("Store topology")
            .parent = Some("foreign-root".to_string());
        assert!(
            exact_retained_root_owned_topology(
                &state,
                "store",
                DesiredCanisterKind::Store,
                "root",
                "store-principal",
                "root-principal",
            )
            .is_none()
        );
        state
            .topology
            .get_mut("store")
            .expect("Store topology")
            .parent = Some("root".to_string());
        state.principals.clear();
        state.pending_principals = BTreeMap::from([
            ("root".to_string(), "root-principal".to_string()),
            ("store".to_string(), "store-principal".to_string()),
        ]);
        assert!(
            exact_retained_root_owned_topology(
                &state,
                "store",
                DesiredCanisterKind::Store,
                "root",
                "store-principal",
                "root-principal",
            )
            .is_some()
        );
        state
            .principals
            .insert("store".to_string(), "foreign-store".to_string());
        assert!(
            exact_retained_root_owned_topology(
                &state,
                "store",
                DesiredCanisterKind::Store,
                "root",
                "store-principal",
                "root-principal",
            )
            .is_none()
        );
    }

    #[test]
    fn completed_reinstall_requires_exact_continuity_when_ordinary_status_has_no_version() {
        let desired = "11".repeat(32);
        assert_eq!(
            completed_reinstall_continuity(7, true, true, Some(&desired), &desired, None,),
            Ok(())
        );
        assert_eq!(
            completed_reinstall_continuity(7, true, true, Some(&desired), &desired, Some(8),),
            Ok(())
        );
        for (rejected, field) in [
            (
                completed_reinstall_continuity(7, false, true, Some(&desired), &desired, None),
                "Principal",
            ),
            (
                completed_reinstall_continuity(7, true, false, Some(&desired), &desired, None),
                "Root/parent/kind topology",
            ),
            (
                completed_reinstall_continuity(
                    7,
                    true,
                    true,
                    Some(&"22".repeat(32)),
                    &desired,
                    None,
                ),
                "module SHA-256",
            ),
            (
                completed_reinstall_continuity(7, true, true, Some(&desired), &desired, Some(7)),
                "canister version",
            ),
        ] {
            assert_eq!(rejected, Err(field));
        }
    }

    #[test]
    fn toko_fresh_fleet_create_responses_retain_the_exact_requested_balance() {
        let canister = Principal::from_slice(&[9; 29]);
        let canister_text = canister.to_text();
        for (response, receipt) in [
            (
                Ok(CreateCanisterSuccess {
                    block_id: Nat::from(41_u8),
                    canister_id: canister,
                }),
                "41",
            ),
            (
                Err(CreateCanisterError::Duplicate {
                    duplicate_of: Nat::from(41_u8),
                    canister_id: Some(canister),
                }),
                "41",
            ),
        ] {
            let outcome = create_canister_response_outcome(response, 5_000)
                .expect("successful or duplicate-with-Principal Create response");
            assert_eq!(
                outcome.created_principal.as_deref(),
                Some(canister_text.as_str())
            );
            assert_eq!(outcome.post_cycles, Some(5_000));
            assert_eq!(outcome.receipt.as_deref(), Some(receipt));
        }
    }

    #[test]
    fn root_funding_inspection_requires_exact_controller_and_pool_shape() {
        let root = Principal::from_slice(&[7; 29]);
        let root_text = root.to_text();
        let cycles = Nat::from(2_898_749_313_788_u128);
        assert_eq!(
            validate_root_funding_inspection(
                "pool-0",
                DesiredCanisterKind::Pool,
                &root_text,
                &[root],
                None,
                &cycles,
            )
            .expect("exact Root-authorized pool inspection"),
            2_898_749_313_788,
        );

        let foreign = Principal::from_slice(&[8; 29]);
        for rejected in [
            validate_root_funding_inspection(
                "pool-0",
                DesiredCanisterKind::Pool,
                &root_text,
                &[foreign],
                None,
                &cycles,
            ),
            validate_root_funding_inspection(
                "pool-0",
                DesiredCanisterKind::Pool,
                &root_text,
                &[root],
                Some(&[1]),
                &cycles,
            ),
        ] {
            assert!(matches!(
                rejected,
                Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict { .. })
            ));
        }
    }
}
