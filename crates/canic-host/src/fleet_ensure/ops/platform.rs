//! Module: fleet_ensure::ops::platform
//!
//! Responsibility: mechanically observe and mutate the exact current Fleet through ICP CLI.
//! Does not own: effect ordering, durable intent, retry policy, or plan approval.
//! Boundary: the workflow calls one method only after persisting its exact action identity.

use crate::{
    canister_protocol::{CanisterProtocolError, call_with_candid, query_with_candid},
    fleet_ensure::{
        dto::{FleetEnsureProgress, FleetObservationStage, FleetObservationTiming},
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
            EffectObservation, EffectOutcome, EffectRetry, EnsurePlatform, EnsureStateError,
            TerminalFleetInventory, canic_init, current_protocol, protocol, root_owned_lifecycle,
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

// Inventory is not callback-settlement evidence: an expired maintenance lease can be taken over.
fn inactive_source_pool_has_full_inventory(
    pool: &CanisterPoolResponse,
    root: &crate::fleet_ensure::model::RootActivationResetRecord,
) -> bool {
    [
        pool.config.maximum_size > 0,
        pool.ready.checked_add(pool.workload) == Some(pool.config.maximum_size),
        pool.config.maximum_size.checked_add(1) == Some(pool.tracked),
        pool.ready >= pool.config.minimum_size,
        root.component_count.checked_add(root.managed_descendants) == Some(pool.workload),
        pool.store == 1,
        pool.store_deletion_pending == 0,
        pool.pending_reset == 0,
        pool.claimed == 0,
        pool.recycling == 0,
        pool.handing_off == 0,
        pool.failed == 0,
        pool.pending_creation.is_none(),
        pool.pending_handoff.is_none(),
    ]
    .into_iter()
    .all(|fact| fact)
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

/// Minimal protected management projection needed to authorize native pool funding.
#[derive(CandidType, Deserialize)]
enum RootFundingInspectionResponse {
    InspectCanister(RootFundingInspectionStatus),
}

#[derive(CandidType, Deserialize)]
struct RootFundingInspectionStatus {
    status: canic_core::dto::canister::CanisterStatusType,
    settings: ManagementCanisterObservationSettings,
    module_hash: Option<Vec<u8>>,
    cycles: Nat,
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

    #[error(
        "management history does not prove the issued reinstall of {canister}; refusing another reset"
    )]
    ReinstallHistoryConflict { canister: String },

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

    #[error(
        "Root {root} pool policy differs from desired input; explicitly review infrastructure reinstall before resetting it"
    )]
    PoolPolicyReinstallReviewRequired { root: String },

    #[error("Cycles Ledger withdraw failed: {0}")]
    LedgerWithdraw(String),

    #[error("completed withdrawal balance conflicts with reviewed funding bounds: {observation:?}")]
    NativeFundingBalanceDrift {
        observation: Box<NativeFundingObservation>,
    },

    #[error("Cycles Ledger estate funding transfer failed: {0}")]
    LedgerTransfer(String),

    #[error(
        "Cycles Ledger transfer fee changed from reviewed {reviewed_fee_cycles} to {expected_fee_cycles}; no transfer was accepted"
    )]
    LedgerTransferFeeChanged {
        expected_fee_cycles: u128,
        reviewed_fee_cycles: u128,
    },

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
}

/// Read evidence shared only by projections of one Fleet observation.
#[derive(Default)]
struct FleetObservationSnapshot {
    pool_pages: BTreeMap<(Principal, Option<Principal>), CanisterPoolResponse>,
    statuses: BTreeMap<String, Option<LiveCanister>>,
}

/// Distinguish an unobserved identity from an observed missing canister.
enum StatusCacheEntry {
    Miss,
    Hit(Option<LiveCanister>),
}

/// Production ICP adapter for the current desired Fleet.
pub struct IcpEnsurePlatform {
    desired: DesiredFleet,
    icp: IcpCli,
    initial_observation_delay: Duration,
    maximum_observation_delay: Duration,
    observation_snapshot: RefCell<Option<FleetObservationSnapshot>>,
    progress_handler: Option<Box<dyn FnMut(FleetEnsureProgress)>>,
    observation_handler: Option<Box<dyn FnMut(FleetObservationTiming)>>,
    estate_observations: BTreeMap<String, EstateFundingDomainObservation>,
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
            observation_snapshot: RefCell::new(None),
            progress_handler: None,
            observation_handler: None,
            estate_observations: BTreeMap::new(),
            root: root.to_path_buf(),
        }
    }

    /// Attach an informational progress sink to this operation.
    #[must_use]
    pub fn with_progress_handler(
        mut self,
        handler: impl FnMut(FleetEnsureProgress) + 'static,
    ) -> Self {
        self.progress_handler = Some(Box::new(handler));
        self
    }

    /// Attach informational stage timings without retaining observations across boundaries.
    #[must_use]
    pub fn with_observation_handler(
        mut self,
        handler: impl FnMut(FleetObservationTiming) + 'static,
    ) -> Self {
        self.observation_handler = Some(Box::new(handler));
        self
    }

    fn finish_observation_timing(
        &mut self,
        stage: FleetObservationStage,
        started: std::time::Instant,
        calls_before: u64,
        succeeded: bool,
    ) {
        let timing = FleetObservationTiming {
            stage,
            elapsed_millis: started.elapsed().as_millis(),
            remote_call_attempts: self.icp.remote_call_count().saturating_sub(calls_before),
            succeeded,
        };
        if let Some(handler) = &mut self.observation_handler {
            handler(timing);
        }
    }

    fn timed_observation<T>(
        &mut self,
        stage: FleetObservationStage,
        observe: impl FnOnce(&mut Self) -> Result<T, IcpEnsurePlatformError>,
    ) -> Result<T, IcpEnsurePlatformError> {
        let started = std::time::Instant::now();
        let calls = self.icp.remote_call_count();
        let result = observe(self);
        self.finish_observation_timing(stage, started, calls, result.is_ok());
        result
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

    // The scope ends on every Result path before workflow can issue an effect.
    // Action observations and subsequent retries therefore always read live state.
    fn with_observation_snapshot<T>(
        &mut self,
        observe: impl FnOnce(&mut Self) -> Result<T, IcpEnsurePlatformError>,
    ) -> Result<T, IcpEnsurePlatformError> {
        self.observation_snapshot
            .replace(Some(FleetObservationSnapshot::default()));
        let result = observe(self);
        self.observation_snapshot.take();
        result
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

    fn current_protocol_owners_are_ready(
        &self,
        state: &FleetEnsureStateRecord,
    ) -> Result<bool, IcpEnsurePlatformError> {
        let mut owners = Vec::new();
        for configured in self.desired.canisters.iter().filter(|configured| {
            configured.presence == crate::fleet_ensure::model::DesiredPresence::Present
                && matches!(
                    configured.kind,
                    DesiredCanisterKind::Coordinator
                        | DesiredCanisterKind::Root
                        | DesiredCanisterKind::Store
                )
        }) {
            let live = self
                .current_principal(state, &configured.name)
                .map(|principal| self.status_optional(principal))
                .transpose()?
                .flatten();
            if live
                .as_ref()
                .is_some_and(|live| live.status != CanisterRuntimeStatus::Running)
            {
                return Ok(false);
            }
            owners.push((configured, live));
        }
        // Check running state and module identity from the same fresh status read.
        // Keep missing-owner/module decisions after the status scan so transport
        // failures retain their existing precedence.
        for (configured, live) in owners {
            let Some(live) = live else {
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

    fn refresh_estate_funding_observation(
        &self,
        observation: &mut EffectObservation,
        state: &FleetEnsureStateRecord,
    ) -> Result<(), IcpEnsurePlatformError> {
        let Some(required) = observation.estate_funding_required.as_mut() else {
            return Ok(());
        };
        let root = required.root.to_text();
        let known_root = self.desired.canisters.iter().any(|canister| {
            canister.kind == DesiredCanisterKind::Root
                && self.current_principal(state, &canister.name) == Some(root.as_str())
        });
        if !known_root || required.cycles_ledger.to_text() != self.desired.cycles_ledger {
            return Err(current_protocol::CurrentProtocolError::ResponseMismatch.into());
        }
        // Root retains the balance from its last creation attempt until its next
        // retry. A completed host transfer can therefore precede this status.
        let available = self.cycles_ledger_balance(&root)?;
        let debit = required.required.to_u128();
        if available >= debit {
            observation.estate_funding_required = None;
        } else {
            required.available = Cycles::new(available);
            required.shortfall = Cycles::new(debit - available);
        }
        Ok(())
    }

    fn require_current_pool_policy(
        &self,
        domains: &BTreeMap<String, EstateFundingDomainObservation>,
    ) -> Result<(), IcpEnsurePlatformError> {
        let Some(bootstrap) = self.desired.bootstrap.as_ref() else {
            return Ok(());
        };
        for root in &bootstrap.roots {
            let Some(pool) = domains
                .get(&root.root)
                .and_then(|domain| domain.pool.as_ref())
            else {
                continue;
            };
            if !pool_policy_is_current(pool, &root.limits.canister_pool) {
                return Err(IcpEnsurePlatformError::PoolPolicyReinstallReviewRequired {
                    root: root.root.clone(),
                });
            }
        }
        Ok(())
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
        let candid = self.root_protocol_candid()?;
        let root_principal = parse_principal("Fleet Subnet Root", root)?;
        let mut start_after = None;
        let mut inventory = EstatePoolInventoryAccumulator::default();
        loop {
            let page = self.query_estate_pool_page(&candid, root_principal, start_after)?;
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
        candid: &Path,
        root: Principal,
        start_after: Option<Principal>,
    ) -> Result<CanisterPoolResponse, IcpEnsurePlatformError> {
        let key = (root, start_after);
        if let Some(page) = self
            .observation_snapshot
            .borrow()
            .as_ref()
            .and_then(|snapshot| snapshot.pool_pages.get(&key).cloned())
        {
            return Ok(page);
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
        if let Some(snapshot) = self.observation_snapshot.borrow_mut().as_mut() {
            snapshot.pool_pages.insert(key, (*page).clone());
        }
        Ok(*page)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one inspection binds each source asset to exact management authority and cycles"
    )]
    fn inspect_reinstall_assets(
        &self,
        state: &FleetEnsureStateRecord,
        authorities: &BTreeMap<String, RootManagementCanisterObservation>,
        observation: &mut FleetObservation,
        candid_by_root: &BTreeMap<String, PathBuf>,
        allow_pending_reset: bool,
    ) -> Result<Vec<crate::fleet_ensure::model::FleetReinstallAssetRecord>, IcpEnsurePlatformError>
    {
        let mut assets = Vec::new();
        let mut seen = BTreeSet::new();
        for (root_name, domain) in &observation.estate_funding_domains {
            let root = domain.root_principal.as_deref().ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement("missing Root principal".to_string())
            })?;
            let pool = domain.pool.as_ref().ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement("missing complete pool".to_string())
            })?;
            if pool.pending_creation.is_some() {
                return Err(IcpEnsurePlatformError::RootManagement(
                    "pool creation is still in flight".to_string(),
                ));
            }
            let root_authority = authorities.get(root_name).ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement("missing Root binding".to_string())
            })?;
            for asset in &pool.assets {
                if !seen.insert(asset.principal.clone())
                    || !(matches!(
                        asset.lifecycle,
                        EstatePoolAssetLifecycle::Ready | EstatePoolAssetLifecycle::Workload
                    ) || (allow_pending_reset
                        && matches!(
                            asset.lifecycle,
                            EstatePoolAssetLifecycle::PendingReset
                                | EstatePoolAssetLifecycle::Failed
                        )))
                {
                    return Err(IcpEnsurePlatformError::RootManagement(
                        "pool asset is duplicated or not terminal".to_string(),
                    ));
                }
                let target = parse_principal("reset pool asset", &asset.principal)?;
                let RootFundingInspectionResponse::InspectCanister(response) = call_with_candid(
                    &self.icp,
                    candid_by_root.get(root_name).ok_or_else(|| {
                        pool_configuration_error("missing reset read contract".to_string())
                    })?,
                    parse_principal("reset Root", root)?,
                    canic_protocol::CANIC_ROOT_COMMAND,
                    &RootInspectionCommand::InspectCanister(CanisterInspectionRequest {
                        canister_id: target,
                    }),
                )
                .map_err(current_protocol::CurrentProtocolError::from)?;
                if !response
                    .settings
                    .controllers
                    .iter()
                    .any(|controller| controller.to_text() == root)
                {
                    return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        canister: asset.principal.clone(),
                        field: "Root controller",
                    });
                }
                let cycles = validate_inspected_cycles(
                    &asset.principal,
                    InspectedModule::Any,
                    response.module_hash.as_deref(),
                    &response.cycles,
                )?;
                let mut controllers = response
                    .settings
                    .controllers
                    .iter()
                    .map(Principal::to_text)
                    .collect::<Vec<_>>();
                controllers.sort();
                if controllers != [root.to_string()] {
                    return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        canister: asset.principal.clone(),
                        field: "exact Root controllers",
                    });
                }

                let module_sha256 = response
                    .module_hash
                    .as_ref()
                    .map(canic_core::cdk::utils::hash::hex_bytes);
                let configured_name = self
                    .desired
                    .canisters
                    .iter()
                    .find(|c| {
                        c.principal.as_deref() == Some(&asset.principal)
                            || self.current_principal(state, &c.name) == Some(&asset.principal)
                    })
                    .map(|c| c.name.clone());
                let name = configured_name
                    .unwrap_or_else(|| format!("reinstall-pool-{}", asset.principal));
                observation.canisters.insert(
                    name,
                    Some(LiveCanister {
                        canister_version: None,
                        controllers: controllers.clone(),
                        cycles,
                        module_sha256: module_sha256.clone(),
                        principal: asset.principal.clone(),
                        reinstall_required: false,
                        root_owned_lifecycle: Some(match asset.lifecycle {
                            EstatePoolAssetLifecycle::Ready => RootOwnedCanisterLifecycle::Idle,
                            EstatePoolAssetLifecycle::Workload => {
                                RootOwnedCanisterLifecycle::Workload
                            }
                            _ => RootOwnedCanisterLifecycle::Retained,
                        }),
                        status: match response.status {
                            canic_core::dto::canister::CanisterStatusType::Running => {
                                CanisterRuntimeStatus::Running
                            }
                            canic_core::dto::canister::CanisterStatusType::Stopped => {
                                CanisterRuntimeStatus::Stopped
                            }
                            canic_core::dto::canister::CanisterStatusType::Stopping => {
                                CanisterRuntimeStatus::Stopping
                            }
                        },
                    }),
                );
                observation
                    .additional_controlled_cycles
                    .remove(&asset.principal);
                assets.push(crate::fleet_ensure::model::FleetReinstallAssetRecord {
                    controllers,
                    module_sha256,
                    principal: asset.principal.clone(),
                    root: root_name.clone(),
                    subnet: root_authority.subnet.clone(),
                });
            }
        }
        assets.sort_by(|a, b| a.principal.cmp(&b.principal));
        Ok(assets)
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
        if let Some(cached) = self
            .observation_snapshot
            .borrow()
            .as_ref()
            .and_then(|snapshot| snapshot.statuses.get(principal).cloned())
        {
            return Ok(cached);
        }
        let observed = self.read_status_optional(principal)?;
        if let Some(snapshot) = self.observation_snapshot.borrow_mut().as_mut() {
            snapshot
                .statuses
                .insert(principal.to_string(), observed.clone());
        }
        Ok(observed)
    }

    fn read_status_optional(
        &self,
        principal: &str,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        Self::read_status_with(&self.icp, principal)
    }

    fn read_status_with(
        icp: &IcpCli,
        principal: &str,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        if icp.uses_direct_local_replica() {
            return Self::direct_local_status_optional(icp, principal);
        }
        let report = match icp.canister_status_report(principal) {
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
        icp: &IcpCli,
        principal: &str,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let canister_id = parse_principal("local canister status target", principal)?;
        let response = icp
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
        let cycles = validate_root_controlled_inspection(
            name,
            InspectedModule::Empty,
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

    fn observe_configured_canister(
        &self,
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        if configured.kind == DesiredCanisterKind::Pool {
            return self.observe_root_owned_canister(configured, principal, state);
        }
        let status = self.status_optional(principal);
        self.configured_status_or_root(configured, principal, state, status)
    }

    fn configured_status_or_root(
        &self,
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        state: &FleetEnsureStateRecord,
        status: Result<Option<LiveCanister>, IcpEnsurePlatformError>,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        match status {
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
        if self.required_root_status(&configured.name, root)? == CanisterRuntimeStatus::Stopped {
            return self
                .retained_root_owned_observation(configured, principal, parent, root, state);
        }
        let target = parse_principal("Root-owned canister", principal)?;
        let mut start_after = None;
        loop {
            let page = self.query_estate_pool_page(
                &candid,
                parse_principal("Fleet Subnet Root", root)?,
                start_after,
            )?;
            if let Some(mut asset) = page
                .entries
                .into_iter()
                .find(|asset| asset.canister_id == target)
            {
                if matches!(
                    asset.status,
                    canic_core::dto::pool::CanisterPoolAssetStatus::PendingReset
                ) && asset.cycles.to_u128() == 0
                {
                    let cycles = self.inspect_pending_pool_balance(configured, principal, state)?;
                    asset.cycles = Cycles::new(cycles);
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

    fn inspect_pending_pool_balance(
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
        let pending_fresh_pool = self
            .desired
            .bootstrap
            .as_ref()
            .is_some_and(|bootstrap| bootstrap.fresh_estate)
            && configured.principal.is_none()
            && state
                .pending_principals
                .get(&configured.name)
                .map(String::as_str)
                == Some(principal);
        if pending_fresh_pool {
            let response = self
                .inspect_root_owned_canister(configured, principal, state)?
                .ok_or_else(
                    || IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        canister: configured.name.clone(),
                        field: "installed Root",
                    },
                )?;
            return validate_pending_fresh_pool_inspection(
                &configured.name,
                root,
                &self.desired.operator,
                &response.settings.controllers,
                response.module_hash.as_deref(),
                &response.cycles,
            );
        }
        self.inspect_pool_balance(&configured.name, root, principal, InspectedModule::Any)
    }

    fn inspect_pool_balance(
        &self,
        name: &str,
        root: &str,
        principal: &str,
        module: InspectedModule,
    ) -> Result<u128, IcpEnsurePlatformError> {
        if self.required_root_status(name, root)? != CanisterRuntimeStatus::Running {
            return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                canister: name.to_string(),
                field: "running Root",
            });
        }
        self.require_operator()?;
        let target = parse_principal("Root-owned funding target", principal)?;
        let response: RootFundingInspectionResponse = call_with_candid(
            &self.icp,
            &self.root_protocol_candid()?,
            parse_principal("Fleet Subnet Root", root)?,
            canic_protocol::CANIC_ROOT_COMMAND,
            &RootInspectionCommand::InspectCanister(CanisterInspectionRequest {
                canister_id: target,
            }),
        )
        .map_err(current_protocol::CurrentProtocolError::from)?;
        let RootFundingInspectionResponse::InspectCanister(response) = response;
        validate_root_controlled_inspection(
            name,
            module,
            root,
            &response.settings.controllers,
            response.module_hash.as_deref(),
            &response.cycles,
        )
    }

    fn pool_funding_module(
        &self,
        name: &str,
        authority: &crate::fleet_ensure::model::PoolFundingAuthority,
        principal: &str,
    ) -> Result<InspectedModule, IcpEnsurePlatformError> {
        let candid = self.root_protocol_candid()?;
        let root = parse_principal("Fleet Subnet Root", &authority.root)?;
        let target = parse_principal("pool funding target", principal)?;
        let mut start_after = None;
        loop {
            let page = self.query_estate_pool_page(&candid, root, start_after)?;
            if let Some(asset) = page
                .entries
                .iter()
                .find(|asset| asset.canister_id == target)
            {
                return pool_funding_inspected_module(name, authority.lifecycle, &asset.status);
            }
            if page.next_start_after.is_none() || page.next_start_after == start_after {
                return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                    canister: name.to_string(),
                    field: "reviewed pool membership",
                });
            }
            start_after = page.next_start_after;
        }
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
                canic_protocol::CANIC_OBSERVABILITY,
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
        reviewed_fee_cycles: u128,
    ) -> Result<EffectOutcome, IcpEnsurePlatformError> {
        let request = CyclesLedgerTransferArgs {
            amount: Nat::from(amount),
            created_at_time: Some(created_at_time),
            fee: Some(Nat::from(reviewed_fee_cycles)),
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
            Err(CyclesLedgerTransferError::BadFee { expected_fee }) => {
                Err(IcpEnsurePlatformError::LedgerTransferFeeChanged {
                    expected_fee_cycles: ledger_fee_cycles(expected_fee)?,
                    reviewed_fee_cycles,
                })
            }
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

    fn observe_fleet_snapshot(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, IcpEnsurePlatformError> {
        self.require_operator()?;
        let mut canisters = self
            .timed_observation(FleetObservationStage::ConfiguredCanisters, |platform| {
                platform.observe_configured_canisters(state)
            })?;
        self.reconcile_completed_reinstalls(state, &mut canisters)?;
        let mut estate_funding_domains = self
            .timed_observation(FleetObservationStage::EstateFunding, |platform| {
                platform.observe_estate_funding_domains(state)
            })?;
        self.timed_observation(FleetObservationStage::PoolBalances, |platform| {
            for domain in estate_funding_domains.values_mut() {
                if let (Some(root), Some(pool)) = (&domain.root_principal, &mut domain.pool) {
                    for asset in &mut pool.assets {
                        if matches!(
                            asset.lifecycle,
                            EstatePoolAssetLifecycle::PendingReset
                                | EstatePoolAssetLifecycle::Failed
                        ) {
                            asset.cycles = platform.inspect_pool_balance(
                                &asset.principal,
                                root,
                                &asset.principal,
                                InspectedModule::Any,
                            )?;
                            for live in canisters.values_mut().filter_map(Option::as_mut) {
                                if live.principal == asset.principal {
                                    live.cycles = asset.cycles;
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        })?;
        self.estate_observations.clone_from(&estate_funding_domains);
        self.require_current_pool_policy(&estate_funding_domains)?;
        self.reconcile_completed_reinstalls(state, &mut canisters)?;
        let additional_controlled_cycles =
            Self::additional_pool_cycles(&canisters, &estate_funding_domains)?;
        let protocol_ready = self
            .timed_observation(FleetObservationStage::ProtocolReadiness, |platform| {
                platform.observe_protocol_readiness(operation_id, state, &canisters)
            })?;
        let ledger_fee_cycles =
            self.timed_observation(FleetObservationStage::LedgerFee, |platform| {
                ledger_fee_cycles(platform.icp.canister_query_candid(
                    &platform.desired.cycles_ledger,
                    "icrc1_fee",
                    &(),
                    None,
                )?)
            })?;
        let operator_cycles =
            self.timed_observation(FleetObservationStage::OperatorBalance, |platform| {
                platform
                    .icp
                    .identity_cycles_balance()
                    .map_err(|error| IcpEnsurePlatformError::LedgerWithdraw(error.to_string()))
            })?;
        Ok(FleetObservation {
            additional_controlled_cycles,
            canisters,
            estate_funding_domains,
            ledger_fee_cycles,
            operator_cycles,
            protocol_ready,
        })
    }

    fn observe_configured_canisters(
        &self,
        state: &FleetEnsureStateRecord,
    ) -> Result<BTreeMap<String, Option<LiveCanister>>, IcpEnsurePlatformError> {
        let configured = &self.desired.canisters;
        let mut observed = BTreeMap::new();
        let mut cursor = 0;
        while cursor < configured.len() {
            if configured[cursor].kind == DesiredCanisterKind::Pool {
                let canister = &configured[cursor];
                let live = self
                    .current_principal(state, &canister.name)
                    .map(|principal| self.observe_root_owned_canister(canister, principal, state))
                    .transpose()?
                    .flatten();
                observed.insert(canister.name.clone(), live);
                cursor += 1;
                continue;
            }
            // Only consecutive independent management reads may overlap. Pool dependencies
            // remain ordered, and every started batch drains before exposing an error.
            let end = configured[cursor..]
                .iter()
                .take(super::bounded_observations::MAX_IN_FLIGHT)
                .take_while(|canister| canister.kind != DesiredCanisterKind::Pool)
                .count()
                + cursor;
            self.observe_configured_status_batch(&configured[cursor..end], state, &mut observed)?;
            cursor = end;
        }
        Ok(observed)
    }

    fn observe_configured_status_batch(
        &self,
        configured: &[crate::fleet_ensure::model::DesiredCanister],
        state: &FleetEnsureStateRecord,
        observed: &mut BTreeMap<String, Option<LiveCanister>>,
    ) -> Result<(), IcpEnsurePlatformError> {
        let mut seen = BTreeSet::new();
        let requests = configured
            .iter()
            .filter_map(|canister| self.current_principal(state, &canister.name))
            .filter(|principal| seen.insert(*principal))
            .filter(|principal| matches!(self.cached_status(principal), StatusCacheEntry::Miss))
            .collect::<Vec<_>>();
        let icp = &self.icp;
        let mut results = super::bounded_observations::collect(&requests, |principal| {
            Ok::<_, std::convert::Infallible>((*principal, Self::read_status_with(icp, principal)))
        })
        .expect("read outcomes are retained for ordered authority decisions")
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        for canister in configured {
            let Some(principal) = self.current_principal(state, &canister.name) else {
                observed.insert(canister.name.clone(), None);
                continue;
            };
            let status = if let StatusCacheEntry::Hit(cached) = self.cached_status(principal) {
                Ok(cached)
            } else {
                results
                    .remove(principal)
                    .unwrap_or_else(|| self.status_optional(principal))
            };
            if let Ok(live) = &status
                && let Some(snapshot) = self.observation_snapshot.borrow_mut().as_mut()
            {
                snapshot
                    .statuses
                    .insert(principal.to_string(), live.clone());
            }
            let live = self.configured_status_or_root(canister, principal, state, status)?;
            observed.insert(canister.name.clone(), live);
        }
        Ok(())
    }

    fn cached_status(&self, principal: &str) -> StatusCacheEntry {
        self.observation_snapshot
            .borrow()
            .as_ref()
            .and_then(|snapshot| snapshot.statuses.get(principal).cloned())
            .map_or(StatusCacheEntry::Miss, StatusCacheEntry::Hit)
    }

    fn reconcile_completed_reinstalls(
        &self,
        state: &FleetEnsureStateRecord,
        canisters: &mut BTreeMap<String, Option<LiveCanister>>,
    ) -> Result<(), IcpEnsurePlatformError> {
        for (name, live) in canisters.iter_mut() {
            let Some(live) = live.as_mut() else {
                continue;
            };
            let completed = self.completed_reinstall_is_current(state, name, live)?;
            let store_module_unobserved = live.module_sha256.is_none()
                && live.root_owned_lifecycle == Some(RootOwnedCanisterLifecycle::Store);
            if completed && store_module_unobserved {
                live.module_sha256 = state
                    .topology
                    .get(name)
                    .and_then(|topology| topology.module_hash.clone());
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

    fn management_roots_are_current(
        &self,
        roots: &[(&crate::fleet_ensure::model::DesiredCanister, LiveCanister)],
        reviewed_targets: &BTreeSet<String>,
    ) -> Result<bool, IcpEnsurePlatformError> {
        let mut all_current = true;
        for (configured, live) in roots {
            if live.status != CanisterRuntimeStatus::Running || !reviewed_targets.is_empty() {
                all_current = false;
                continue;
            }
            if live.module_sha256.is_none()
                && is_unallocated_fresh_root(&self.desired, configured, reviewed_targets)
            {
                continue;
            }
            if let Some(wasm) = &configured.wasm {
                let expected = artifact_hash(&resolve_path(&self.root, wasm))?;
                if live.module_sha256.as_deref() != Some(expected.as_str()) {
                    all_current = false;
                }
            }
        }
        Ok(all_current)
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

    fn report_progress(&mut self, progress: FleetEnsureProgress) {
        if let Some(handler) = self.progress_handler.as_mut() {
            handler(progress);
        }
    }

    fn require_retained_root_activation(
        &mut self,
        operation_id: &str,
        root: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<(), Self::Error> {
        let authorities = canic_init::compile_root_authorities(
            &self.root,
            &self.desired,
            &self.protocol_principals(state),
        )?;
        let (_, authority) = authorities
            .iter()
            .find(|(name, _)| name == root)
            .ok_or_else(|| canic_init::CanicInitError::MissingRoot(root.to_string()))?;
        current_protocol::require_store_installation_binding(
            &self.icp,
            &self.root_protocol_candid()?,
            root,
            authority,
            operation_id,
        )?;
        Ok(())
    }

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

    fn authority_sealed(
        &mut self,
        operation_id: &str,
        action: &EnsureAction,
    ) -> Result<bool, Self::Error> {
        Ok(super::authority_seal::observe(&self.icp, &self.root, operation_id, action)?.applied)
    }

    fn reinstall_assets_match(
        &mut self,
        intent: &crate::fleet_ensure::model::FleetReinstallRecord,
        root_name: &str,
        check: super::ReinstallAssetCheck,
    ) -> Result<bool, Self::Error> {
        let binding = intent
            .authorities
            .iter()
            .find(|binding| binding.name == root_name)
            .ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement("missing reset Root".to_string())
            })?;
        let principal = parse_principal("reset Root", &binding.principal)?;
        let candid = self.root_protocol_candid()?;
        for asset in intent.assets.iter().filter(|asset| asset.root == root_name) {
            let RootFundingInspectionResponse::InspectCanister(status) = call_with_candid(
                &self.icp,
                &candid,
                principal,
                canic_protocol::CANIC_ROOT_COMMAND,
                &RootInspectionCommand::InspectCanister(CanisterInspectionRequest {
                    canister_id: parse_principal("reset asset", &asset.principal)?,
                }),
            )
            .map_err(current_protocol::CurrentProtocolError::from)?;
            let mut controllers = status
                .settings
                .controllers
                .iter()
                .map(Principal::to_text)
                .collect::<Vec<_>>();
            controllers.sort();
            let module = status
                .module_hash
                .as_ref()
                .map(canic_core::cdk::utils::hash::hex_bytes);
            let module_matches =
                check == super::ReinstallAssetCheck::Terminal || module == asset.module_sha256;
            if controllers != asset.controllers || !module_matches {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn reinstall_authorities(
        &mut self,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<BTreeMap<String, RootManagementCanisterObservation>>, Self::Error> {
        self.require_operator()?;
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
        let mut authorities = BTreeMap::new();
        let root_targets = self
            .desired
            .canisters
            .iter()
            .filter(|c| c.kind == DesiredCanisterKind::Root)
            .map(|c| c.name.clone())
            .collect();
        let roots = self
            .observe_root_management(state, &root_targets)?
            .ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement("missing Root authority".to_string())
            })?;
        for configured in &self.desired.canisters {
            if configured.kind == DesiredCanisterKind::Pool {
                continue;
            }
            let principal = self
                .current_principal(state, &configured.name)
                .ok_or_else(|| {
                    IcpEnsurePlatformError::UnresolvedCreated(configured.name.clone())
                })?;
            let live = self.install_status_optional(principal)?.ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement("missing infrastructure".to_string())
            })?;
            let subnet = if let Some(root) = roots.roots.get(&configured.name) {
                root.subnet.clone()
            } else if let Some(catalog) = &catalog {
                catalog
                    .catalog
                    .resolve_canister_route(principal)
                    .map(|route| route.subnet.to_text())
                    .map_err(|error| IcpEnsurePlatformError::RootManagement(error.to_string()))?
            } else {
                configured.subnet.clone()
            };
            authorities.insert(
                configured.name.clone(),
                RootManagementCanisterObservation {
                    live,
                    name: configured.name.clone(),
                    subnet,
                },
            );
        }
        Ok(Some(authorities))
    }

    fn reinstall_inventory(
        &mut self,
        source_operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<super::FleetReinstallObservation>, Self::Error> {
        let authorities = self.reinstall_authorities(state)?.ok_or_else(|| {
            IcpEnsurePlatformError::RootManagement("missing infrastructure".to_string())
        })?;
        let terminal = self.terminal_inventory(source_operation_id, state)?;
        let mut observation = self.observe(source_operation_id, state)?;
        for (name, observed) in &authorities {
            observation
                .canisters
                .insert(name.clone(), Some(observed.live.clone()));
        }
        let candid = self.root_protocol_candid()?;
        let candid_by_root = observation
            .estate_funding_domains
            .keys()
            .map(|name| (name.clone(), candid.clone()))
            .collect();
        let assets = self.inspect_reinstall_assets(
            state,
            &authorities,
            &mut observation,
            &candid_by_root,
            false,
        )?;
        let seen = assets
            .iter()
            .map(|asset| asset.principal.clone())
            .collect::<BTreeSet<_>>();
        let infrastructure = authorities
            .values()
            .map(|a| a.live.principal.as_str())
            .collect::<BTreeSet<_>>();
        if terminal
            .controlled_cycles_by_principal
            .keys()
            .any(|pid| !infrastructure.contains(pid.as_str()) && !seen.contains(pid))
        {
            return Err(IcpEnsurePlatformError::RootManagement(
                "terminal inventory escapes the reviewed reset estate".to_string(),
            ));
        }
        Ok(Some(super::FleetReinstallObservation {
            authorities,
            assets,
            observation,
        }))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one source observation binds old protocol receipts, full pagination and physical management inspection"
    )]
    fn activation_reset_inventory(
        &mut self,
        source: &crate::fleet_ensure::model::FleetActivationSourceRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<super::FleetActivationResetObservation>, Self::Error> {
        if source.operator != self.desired.operator
            || source.cycles_ledger != self.desired.cycles_ledger
        {
            return Err(pool_configuration_error(
                "source operator or Cycles Ledger differs from the requested reset".to_string(),
            ));
        }
        let authorities = self
            .reinstall_authorities(state)?
            .ok_or_else(|| pool_configuration_error("missing source infrastructure".to_string()))?;
        let mut observation = FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            canisters: authorities
                .iter()
                .map(|(name, observed)| (name.clone(), Some(observed.live.clone())))
                .collect(),
            estate_funding_domains: BTreeMap::new(),
            ledger_fee_cycles: self
                .desired
                .ledger_fee_cycles
                .parse::<Cycles>()
                .map_err(|_| pool_configuration_error("invalid source fee bound".to_string()))?
                .to_u128(),
            operator_cycles: 0,
            protocol_ready: BTreeMap::new(),
        };
        let mut roots = Vec::new();
        let mut candid_by_root = BTreeMap::new();
        for configured in &self.desired.canisters {
            if configured.kind != DesiredCanisterKind::Root {
                continue;
            }
            let live = &authorities
                .get(&configured.name)
                .ok_or_else(|| pool_configuration_error("missing source Root".to_string()))?
                .live;
            let root = parse_principal("source Root", &live.principal)?;
            let evidence = current_protocol::observe_inactive_activation(
                &self.icp,
                &self.root,
                source,
                &configured.name,
                root,
            )?;
            let candid = source
                .registry_preparations
                .iter()
                .find_map(|action| match action {
                    EnsureAction::FleetProtocol {
                        principal, candid, ..
                    } if principal == &live.principal => Some(self.root.join(candid)),
                    _ => None,
                })
                .ok_or_else(|| {
                    pool_configuration_error("missing source Root read contract".to_string())
                })?;
            let mut inventory = EstatePoolInventoryAccumulator::default();
            let mut cursor = None;
            loop {
                let page = self.query_estate_pool_page(&candid, root, cursor)?;
                if !inactive_source_pool_has_full_inventory(&page, &evidence) {
                    return Err(pool_configuration_error(
                        "source pool has unsettled work or spare creation capacity".to_string(),
                    ));
                }
                let next = inventory.observe_page(&configured.name, page)?;
                if next.is_none() {
                    break;
                }
                if next == cursor {
                    return Err(pool_configuration_error(
                        "source pool cursor did not advance".to_string(),
                    ));
                }
                cursor = next;
            }
            observation.estate_funding_domains.insert(
                configured.name.clone(),
                EstateFundingDomainObservation {
                    balance_cycles: Some(self.cycles_ledger_balance(&live.principal)?),
                    cycles_ledger: self.desired.cycles_ledger.clone(),
                    pool: Some(inventory.finish(&configured.name)?),
                    root_principal: Some(live.principal.clone()),
                },
            );
            candid_by_root.insert(configured.name.clone(), candid);
            roots.push(evidence);
        }
        if roots.is_empty() || roots.len() != source.registry_preparations.len() {
            return Err(pool_configuration_error(
                "source Root inventory is incomplete".to_string(),
            ));
        }
        let assets = self.inspect_reinstall_assets(
            state,
            &authorities,
            &mut observation,
            &candid_by_root,
            false,
        )?;
        Ok(Some(super::FleetActivationResetObservation {
            inventory: super::FleetReinstallObservation {
                authorities,
                assets,
                observation,
            },
            roots,
        }))
    }

    fn activation_reset_inventory_after_reset(
        &mut self,
        intent: &crate::fleet_ensure::model::FleetReinstallRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<super::FleetReinstallObservation>, Self::Error> {
        let authorities = self
            .reinstall_authorities(state)?
            .ok_or_else(|| pool_configuration_error("missing reset infrastructure".to_string()))?;
        let mut observation = FleetObservation {
            canisters: authorities
                .iter()
                .map(|(name, entry)| (name.clone(), Some(entry.live.clone())))
                .collect(),
            additional_controlled_cycles: BTreeMap::new(),
            estate_funding_domains: BTreeMap::new(),
            ledger_fee_cycles: self
                .desired
                .ledger_fee_cycles
                .parse::<Cycles>()
                .map_err(|_| pool_configuration_error("invalid reset Ledger fee".to_string()))?
                .to_u128(),
            operator_cycles: 0,
            protocol_ready: BTreeMap::new(),
        };
        let candid = self.root_protocol_candid()?;
        let mut candid_by_root = BTreeMap::new();
        for configured in &self.desired.canisters {
            if configured.kind != DesiredCanisterKind::Root {
                continue;
            }
            let binding = authorities
                .get(&configured.name)
                .ok_or_else(|| pool_configuration_error("missing reset Root".to_string()))?;
            let root = parse_principal("reset Root", &binding.live.principal)?;
            let mut inventory = EstatePoolInventoryAccumulator::default();
            let mut cursor = None;
            loop {
                let page = self.query_estate_pool_page(&candid, root, cursor)?;
                let next = inventory.observe_page(&configured.name, page)?;
                if next.is_none() {
                    break;
                }
                if next == cursor {
                    return Err(pool_configuration_error(
                        "reset inventory cursor did not advance".to_string(),
                    ));
                }
                cursor = next;
            }
            let pool = inventory.finish(&configured.name)?;
            let expected = intent
                .assets
                .iter()
                .filter(|asset| asset.root == configured.name)
                .map(|asset| asset.principal.as_str())
                .collect::<BTreeSet<_>>();
            let observed = pool
                .assets
                .iter()
                .map(|asset| asset.principal.as_str())
                .collect::<BTreeSet<_>>();
            if observed != expected || pool.pending_creation.is_some() {
                return Err(pool_configuration_error(
                    "reset changed the complete physical estate".to_string(),
                ));
            }
            observation.estate_funding_domains.insert(
                configured.name.clone(),
                EstateFundingDomainObservation {
                    balance_cycles: Some(self.cycles_ledger_balance(&binding.live.principal)?),
                    cycles_ledger: self.desired.cycles_ledger.clone(),
                    pool: Some(pool),
                    root_principal: Some(binding.live.principal.clone()),
                },
            );
            candid_by_root.insert(configured.name.clone(), candid.clone());
        }
        let assets = self.inspect_reinstall_assets(
            state,
            &authorities,
            &mut observation,
            &candid_by_root,
            true,
        )?;
        Ok(Some(super::FleetReinstallObservation {
            authorities,
            assets,
            observation,
        }))
    }

    fn observe_root_management(
        &mut self,
        state: &FleetEnsureStateRecord,
        reviewed_targets: &BTreeSet<String>,
    ) -> Result<Option<RootManagementObservation>, Self::Error> {
        self.timed_observation(FleetObservationStage::RootManagement, |platform| {
            let configured_roots = platform
                .desired
                .canisters
                .iter()
                .filter(|configured| {
                    configured.kind == DesiredCanisterKind::Root
                        && configured.presence
                            == crate::fleet_ensure::model::DesiredPresence::Present
                })
                .collect::<Vec<_>>();
            if configured_roots.is_empty() {
                return Ok(None);
            }
            platform.require_operator()?;
            let mut observed_roots = Vec::new();
            for configured in configured_roots {
                let Some(principal) = platform.current_principal(state, &configured.name) else {
                    if is_unallocated_fresh_root(&platform.desired, configured, reviewed_targets) {
                        continue;
                    }
                    return Err(IcpEnsurePlatformError::RootManagement(format!(
                        "configured Root {} has no exact Principal",
                        configured.name
                    )));
                };
                let live = platform.status_optional(principal)?.ok_or_else(|| {
                    IcpEnsurePlatformError::RootManagement(format!(
                        "configured Root {} is unavailable",
                        configured.name
                    ))
                })?;
                observed_roots.push((configured, live));
            }
            let all_current =
                platform.management_roots_are_current(&observed_roots, reviewed_targets)?;
            if reviewed_targets.is_empty() && all_current {
                return Ok(None);
            }
            let network =
                resolve_icp_build_network_from_root(&platform.root, &platform.desired.environment)
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
                    load_mainnet_subnet_catalog(&platform.root, now).map_err(|error| {
                        IcpEnsurePlatformError::RootManagement(error.to_string())
                    })?,
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
                            .map_err(|error| {
                                IcpEnsurePlatformError::RootManagement(error.to_string())
                            })
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
            let operator_cycles = platform
                .icp
                .identity_cycles_balance()
                .map_err(|error| IcpEnsurePlatformError::LedgerWithdraw(error.to_string()))?;
            Ok(Some(RootManagementObservation {
                operator_cycles,
                roots,
            }))
        })
    }

    fn observe(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        self.with_observation_snapshot(|platform| {
            platform.observe_fleet_snapshot(operation_id, state)
        })
    }

    fn protocol_actions(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Vec<EnsureAction>, Self::Error> {
        self.timed_observation(FleetObservationStage::ProtocolActions, |platform| {
            if platform.desired.protocol.is_none() {
                return Ok(Vec::new());
            }
            current_protocol::validate_component_pool_capacity(&platform.root, &platform.desired)?;
            if !platform.current_protocol_owners_are_ready(state)? {
                return Ok(Vec::new());
            }
            let reconciliation = current_protocol::compile_pool_reconciliation(
                &platform.root,
                &platform.desired,
                state,
                &platform.estate_observations,
            )?;
            if !reconciliation.is_empty() {
                return Ok(reconciliation);
            }
            current_protocol::compile(
                &platform.icp,
                &platform.root,
                &platform.desired,
                operation_id,
                state,
            )
            .map_err(Into::into)
        })
    }

    fn fresh_protocol_actions(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Vec<EnsureAction>, Self::Error> {
        current_protocol::compile_fresh_protocol(&self.root, &self.desired, state, operation_id)
            .map_err(Into::into)
    }

    fn terminal_inventory(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<TerminalFleetInventory, Self::Error> {
        self.timed_observation(FleetObservationStage::TerminalInventory, |platform| {
            if platform.desired.protocol.is_none() {
                if state.active_registry.is_some() {
                    return Err(current_protocol::CurrentProtocolError::Configuration(
                        "managed terminal Fleet cannot drop its typed protocol intent".to_string(),
                    )
                    .into());
                }
                return Ok(TerminalFleetInventory::default());
            }
            super::current_inventory::terminal_inventory(
                &platform.icp,
                &platform.root,
                &platform.desired,
                operation_id,
                state,
            )
            .map_err(Into::into)
        })
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
        if matches!(action, EnsureAction::SealAuthority { .. }) {
            return super::authority_seal::observe(&self.icp, &self.root, operation_id, action)
                .map_err(Into::into);
        }
        let mut retry = EffectRetry::None;
        let mut post_cycles = None;
        let (applied, progress_identity) = match action {
            EnsureAction::SealAuthority { .. } => {
                unreachable!("authority seals use their typed observer")
            }
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
                let maximum_observation_burn_cycles =
                    super::maximum_creation_observation_burn(&self.desired).ok_or(
                        IcpEnsurePlatformError::Arithmetic("Create execution and observation burn"),
                    )?;
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
                ..
            } => {
                let live_cycles = self.action_cycles(action, state)?;
                post_cycles = live_cycles;
                let observation = NativeFundingObservation {
                    amount: *amount,
                    expected_post_cycles: *expected_post_cycles,
                    funding_deficit_cycles: *funding_deficit_cycles,
                    funding_margin_cycles: *funding_margin_cycles,
                    live_cycles,
                    pre_cycles: record.pre_cycles,
                };
                (
                    native_funding_completion(observation, record.receipt.is_some())?,
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
                reinstall_witness,
                principal,
                wasm_sha256,
                ..
            } => {
                let live =
                    self.install_status_optional(Self::action_principal(state, principal)?)?;
                let applied = if let Some(witness) = reinstall_witness {
                    let current = self.reinstall_authorities(state)?;
                    let authority = current
                        .as_ref()
                        .and_then(|a| a.get(&witness.authority.name))
                        .and_then(super::reinstall::authority_binding);
                    if authority.as_ref() != Some(&witness.authority) {
                        return Err(IcpEnsurePlatformError::ReinstallHistoryConflict {
                            canister: witness.authority.principal.clone(),
                        });
                    }
                    let live = live.as_ref().ok_or_else(|| {
                        IcpEnsurePlatformError::ReinstallHistoryConflict {
                            canister: principal.clone(),
                        }
                    })?;
                    let before = record.pre_canister_version.ok_or_else(|| {
                        IcpEnsurePlatformError::ReinstallHistoryConflict {
                            canister: principal.clone(),
                        }
                    })?;
                    let history = super::install_history::observe(
                        &self.icp,
                        &self.root,
                        witness,
                        parse_principal("operator", &self.desired.operator)?,
                        before,
                        live,
                    )?;
                    match super::install_history::reconcile(
                        history,
                        wasm_sha256,
                        &witness.prior_module_sha256,
                        before,
                        live,
                    ) {
                        super::install_history::HistoryEffect::Applied => true,
                        super::install_history::HistoryEffect::NotApplied => false,
                        super::install_history::HistoryEffect::Conflict => {
                            return Err(IcpEnsurePlatformError::ReinstallHistoryConflict {
                                canister: live.principal.clone(),
                            });
                        }
                    }
                } else {
                    live.as_ref().is_some_and(|live| {
                        install_effect_applied(
                            *mode,
                            wasm_sha256,
                            live.module_sha256.as_deref(),
                            record.pre_canister_version,
                            live.canister_version,
                        )
                    })
                };
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
                let mut observation = match current_protocol::observe(&self.icp, &self.root, action)
                {
                    Ok(observation) => observation,
                    Err(error)
                        if matches!(
                            current_action.as_ref(),
                            crate::fleet_ensure::model::CurrentFleetProtocolAction::AdoptStore { .. }
                        ) && recoverable_current_protocol_error(&error) =>
                    {
                        EffectObservation {
                            provisioning_failure: None,
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
                self.refresh_estate_funding_observation(&mut observation, state)?;
                return Ok(observation);
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
            provisioning_failure: None,
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
            EnsureAction::SealAuthority { .. } => {
                super::authority_seal::apply(&self.icp, &self.root, operation_id, action)
                    .map_err(Into::into)
            }
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
                ledger_fee_cycles,
                principal,
                ..
            } => self.apply_estate_fund(
                *amount,
                *created_at_time,
                ledger,
                Self::action_principal(state, principal)?,
                *ledger_fee_cycles,
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
        if let EnsureAction::Fund {
            pool_funding: Some(authority),
            name,
            principal,
            ..
        } = action
        {
            let principal = Self::action_principal(state, principal)?;
            let module = self.pool_funding_module(name, authority, principal)?;
            return self
                .inspect_pool_balance(name, &authority.root, principal, module)
                .map(Some);
        }
        let (name, principal) = match action {
            EnsureAction::Create { .. } => return Ok(None),
            EnsureAction::SealAuthority {
                name, principal, ..
            }
            | EnsureAction::Delete {
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

/// Require the reviewed floor after all planning-to-completion burn within its margin.
pub const fn native_funding_applied(observation: NativeFundingObservation) -> bool {
    let Some(pre_cycles) = observation.pre_cycles else {
        return false;
    };
    let Some(live_cycles) = observation.live_cycles else {
        return false;
    };
    let Some(maximum_live_cycles) = pre_cycles.checked_add(observation.amount) else {
        return false;
    };
    let Some(reviewed_pre_cycles) = observation
        .expected_post_cycles
        .checked_sub(observation.amount)
    else {
        return false;
    };
    let Some(minimum_live_cycles) = observation
        .expected_post_cycles
        .checked_sub(observation.funding_margin_cycles)
    else {
        return false;
    };
    let Some(minimum_from_deficit) =
        reviewed_pre_cycles.checked_add(observation.funding_deficit_cycles)
    else {
        return false;
    };
    let reviewed_amount_matches =
        observation.funding_deficit_cycles > 0 && minimum_live_cycles == minimum_from_deficit;
    let observation_is_bounded = pre_cycles <= reviewed_pre_cycles
        && live_cycles >= minimum_live_cycles
        && live_cycles <= maximum_live_cycles;
    reviewed_amount_matches && observation_is_bounded
}

fn native_funding_completion(
    observation: NativeFundingObservation,
    has_receipt: bool,
) -> Result<bool, IcpEnsurePlatformError> {
    if !has_receipt || observation.live_cycles.is_none() {
        return Ok(false);
    }
    if !native_funding_applied(observation) {
        return Err(IcpEnsurePlatformError::NativeFundingBalanceDrift {
            observation: Box::new(observation),
        });
    }
    Ok(true)
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

/// Creation and Ready funding require empty modules; reviewed reset funding accepts installed assets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InspectedModule {
    Any,
    Empty,
}

fn pool_funding_inspected_module(
    canister: &str,
    reviewed: EstatePoolAssetLifecycle,
    observed: &CanisterPoolAssetStatus,
) -> Result<InspectedModule, IcpEnsurePlatformError> {
    match (reviewed, observed) {
        (EstatePoolAssetLifecycle::Ready, CanisterPoolAssetStatus::Ready) => {
            Ok(InspectedModule::Empty)
        }
        (EstatePoolAssetLifecycle::PendingReset, CanisterPoolAssetStatus::PendingReset)
        | (EstatePoolAssetLifecycle::Failed, CanisterPoolAssetStatus::Failed { .. }) => {
            Ok(InspectedModule::Any)
        }
        _ => Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
            canister: canister.to_string(),
            field: "reviewed pool lifecycle",
        }),
    }
}

fn validate_pending_fresh_pool_inspection(
    canister: &str,
    root: &str,
    operator: &str,
    controllers: &[Principal],
    module_hash: Option<&[u8]>,
    cycles: &Nat,
) -> Result<u128, IcpEnsurePlatformError> {
    let mut actual = controllers
        .iter()
        .map(Principal::to_text)
        .collect::<Vec<_>>();
    actual.sort();
    let mut temporary = vec![root.to_string(), operator.to_string()];
    temporary.sort();
    if actual != [root] && actual != temporary {
        return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
            canister: canister.to_string(),
            field: "reviewed fresh pool controllers",
        });
    }
    // Controller finalization is itself journalled. Until it completes, the exact
    // temporary operator remains a controller of this issued fresh Create only.
    validate_inspected_cycles(canister, InspectedModule::Empty, module_hash, cycles)
}

fn validate_root_controlled_inspection(
    canister: &str,
    module: InspectedModule,
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
    validate_inspected_cycles(canister, module, module_hash, cycles)
}

fn validate_inspected_cycles(
    canister: &str,
    module: InspectedModule,
    module_hash: Option<&[u8]>,
    cycles: &Nat,
) -> Result<u128, IcpEnsurePlatformError> {
    if matches!(module, InspectedModule::Empty) && module_hash.is_some() {
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
    fn inactive_source_inventory_rejects_spare_capacity_pending_work_and_omissions() {
        let root = crate::fleet_ensure::model::RootActivationResetRecord {
            root: "root".to_string(),
            activation_operation_id: [1; 32],
            inventory_hash: [2; 32],
            provisioning_receipt_hash: [3; 32],
            component_count: 1,
            managed_descendants: 2,
        };
        let page = inactive_source_pool_page();
        assert!(inactive_source_pool_has_full_inventory(&page, &root));
        let mut complete = EstatePoolInventoryAccumulator::default();
        assert_eq!(
            complete.observe_page("root", page.clone()).expect("page"),
            None
        );
        assert_eq!(complete.finish("root").expect("complete").assets.len(), 4);

        let mut spare = page.clone();
        spare.config.maximum_size += 1;
        let mut insufficient_reserve = page.clone();
        insufficient_reserve.config.minimum_size = 2;
        let mut pending = page.clone();
        pending.pending_reset = 1;
        let mut wrong_workload = page.clone();
        wrong_workload.workload -= 1;
        wrong_workload.ready += 1;
        let mut overflow = page.clone();
        overflow.workload = u32::MAX;
        for changed in [
            spare,
            insufficient_reserve,
            pending,
            wrong_workload,
            overflow,
        ] {
            assert!(!inactive_source_pool_has_full_inventory(&changed, &root));
        }

        let mut omitted = page;
        omitted.entries.pop();
        let mut inventory = EstatePoolInventoryAccumulator::default();
        inventory
            .observe_page("root", omitted)
            .expect("bounded page");
        assert!(matches!(
            inventory.finish("root"),
            Err(IcpEnsurePlatformError::CurrentProtocol(
                current_protocol::CurrentProtocolError::Configuration(_)
            ))
        ));
    }

    fn inactive_source_pool_page() -> CanisterPoolResponse {
        let claim = canic_core::dto::pool::CanisterPoolClaim {
            component: canic_core::ids::ComponentInstanceId::from_generated_bytes([4; 32]),
            operation_id: [5; 32],
        };
        let entries = (0..5)
            .map(|index| CanisterPoolAsset {
                canister_id: Principal::from_slice(&[index]),
                creation_receipt: None,
                cycles: Cycles::new(100),
                origin: if index == 0 {
                    CanisterPoolAssetOrigin::InfrastructureStore
                } else {
                    CanisterPoolAssetOrigin::Imported
                },
                status: match index {
                    0 => CanisterPoolAssetStatus::Store,
                    1 => CanisterPoolAssetStatus::Ready,
                    _ => CanisterPoolAssetStatus::Workload {
                        claim: claim.clone(),
                    },
                },
                added_at_ns: 1,
                updated_at_ns: 1,
            })
            .collect();
        CanisterPoolResponse {
            config: canic_core::ids::FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 4,
                canister_cycles: Cycles::new(100),
                creation_execution_margin: Cycles::new(1),
            },
            tracked: 5,
            store: 1,
            store_deletion_pending: 0,
            pooled: 1,
            workload: 3,
            surplus: 0,
            ready: 1,
            pending_reset: 0,
            claimed: 0,
            recycling: 0,
            handing_off: 0,
            failed: 0,
            completed_handoffs: 0,
            pending_creation: None,
            pending_handoff: None,
            entries,
            next_start_after: None,
        }
    }

    #[test]
    fn pool_funding_binds_reset_lifecycle_without_weakening_ready_inspection() {
        let failed = CanisterPoolAssetStatus::Failed {
            reason: "reset stopped".to_string(),
        };
        for (lifecycle, status, expected) in [
            (
                EstatePoolAssetLifecycle::Ready,
                CanisterPoolAssetStatus::Ready,
                InspectedModule::Empty,
            ),
            (
                EstatePoolAssetLifecycle::PendingReset,
                CanisterPoolAssetStatus::PendingReset,
                InspectedModule::Any,
            ),
            (
                EstatePoolAssetLifecycle::Failed,
                failed,
                InspectedModule::Any,
            ),
        ] {
            let module = pool_funding_inspected_module("pool", lifecycle, &status).unwrap();
            assert_eq!(module, expected);
            let installed =
                validate_inspected_cycles("pool", module, Some(&[1; 32]), &Nat::from(100_u8));
            assert_eq!(installed.is_ok(), module == InspectedModule::Any);
            for wrong in [
                EstatePoolAssetLifecycle::Ready,
                EstatePoolAssetLifecycle::PendingReset,
                EstatePoolAssetLifecycle::Failed,
                EstatePoolAssetLifecycle::Workload,
            ] {
                if wrong != lifecycle {
                    assert!(matches!(
                        pool_funding_inspected_module("pool", wrong, &status),
                        Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict { .. })
                    ));
                }
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn funding_pause_uses_fresh_ledger_balance_without_completing_protocol_work() {
        let mut fixture = ProtocolOwnersFixture::new();
        let root = Principal::from_slice(&[1]);
        let ledger = Principal::from_slice(&[2]);
        fixture.platform.desired.cycles_ledger = ledger.to_text();
        fixture.platform.desired.canisters[1].principal = Some(root.to_text());
        std::fs::write(
            fixture.root.join("icp"),
            format!(
                "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp 1.3.0'; exit 0; fi\ncat '{}'/balance.json\n",
                fixture.root.display(),
            ),
        )
        .unwrap();
        let pause = canic_core::dto::component_provisioning::RootEstateFundingRequired {
            available: Cycles::new(0),
            attempt_count: 0,
            creation_amount: Cycles::new(100),
            cycles_ledger: ledger,
            execution_margin: Cycles::new(10),
            last_attempt_at_ns: None,
            ledger_fee: Cycles::new(1),
            management_creation_fee: Cycles::new(20),
            operation_id: [1; 32],
            readiness_floor: Cycles::new(70),
            required: Cycles::new(101),
            retry_at_ns: 60,
            root,
            shortfall: Cycles::new(101),
        };
        for available in [0_u128, 40, 101, 404] {
            std::fs::write(
                fixture.root.join("balance.json"),
                serde_json::json!({
                    "response_bytes": hex_bytes(candid::encode_one(Nat::from(available)).unwrap()),
                })
                .to_string(),
            )
            .unwrap();
            let mut observed = EffectObservation {
                provisioning_failure: None,
                applied: false,
                estate_funding_required: Some(pause.clone()),
                post_cycles: None,
                progress_identity: "pending-creation".to_string(),
                retry: EffectRetry::ContinuePoolMaintenance,
            };
            fixture
                .platform
                .refresh_estate_funding_observation(&mut observed, &fixture.state)
                .unwrap();
            assert!(!observed.applied);
            assert_eq!(observed.retry, EffectRetry::ContinuePoolMaintenance);
            assert_eq!(observed.progress_identity, "pending-creation");
            let expected = (available < 101).then(|| {
                let mut expected = pause.clone();
                expected.available = Cycles::new(available);
                expected.shortfall = Cycles::new(101 - available);
                expected
            });
            assert_eq!(observed.estate_funding_required, expected);
        }
        for (root, ledger) in [
            (Principal::anonymous(), ledger),
            (root, Principal::anonymous()),
        ] {
            let mut changed = pause.clone();
            changed.root = root;
            changed.cycles_ledger = ledger;
            let mut observed = EffectObservation {
                provisioning_failure: None,
                applied: false,
                estate_funding_required: Some(changed),
                post_cycles: None,
                progress_identity: String::new(),
                retry: EffectRetry::None,
            };
            assert!(matches!(
                fixture
                    .platform
                    .refresh_estate_funding_observation(&mut observed, &fixture.state),
                Err(IcpEnsurePlatformError::CurrentProtocol(
                    current_protocol::CurrentProtocolError::ResponseMismatch
                ))
            ));
        }
        std::fs::remove_dir_all(&fixture.root).unwrap();
    }

    #[test]
    fn completed_withdrawal_outside_bounds_fails_without_endless_observation() {
        let observation = NativeFundingObservation {
            amount: 350,
            expected_post_cycles: 800,
            funding_deficit_cycles: 150,
            funding_margin_cycles: 200,
            live_cycles: Some(599),
            pre_cycles: Some(449),
        };
        assert!(!native_funding_completion(observation, false).unwrap());
        assert!(matches!(
            native_funding_completion(observation, true),
            Err(IcpEnsurePlatformError::NativeFundingBalanceDrift {
                observation: actual,
            }) if *actual == observation
        ));
        assert!(
            native_funding_completion(
                NativeFundingObservation {
                    live_cycles: Some(600),
                    ..observation
                },
                true
            )
            .unwrap()
        );
    }

    #[cfg(unix)]
    struct ProtocolOwnersFixture {
        root: PathBuf,
        platform: IcpEnsurePlatform,
        state: FleetEnsureStateRecord,
    }

    #[cfg(unix)]
    impl ProtocolOwnersFixture {
        fn new() -> Self {
            use std::{fs, os::unix::fs::PermissionsExt};

            let root = crate::test_support::temp_dir("canic-protocol-owner-observations");
            fs::create_dir_all(&root).unwrap();
            fs::write(root.join("owner.wasm"), b"current-owner-module").unwrap();
            let executable = root.join("icp");
            fs::write(&executable, format!(
                "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp 1.3.0'; exit 0; fi\nwhile [ \"$#\" -gt 0 ] && [ \"$1\" != status ]; do shift; done\nshift\nprintf '%s\\n' \"$1\" >> '{}'/calls\ncat '{}'/\"$1\".json\n",
                root.display(), root.display(),
            )).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
            let canisters = ["coordinator", "root", "store"].map(|name| {
                serde_json::json!({
                    "controller_canisters": [], "controllers": [], "drain": null,
                    "initial_cycles": "1T", "init_arg": null, "init_candid": null,
                    "kind": name, "minimum_cycles": "1T", "name": name,
                    "parent": null, "presence": "present", "principal": name,
                    "replace": false, "subnet": "subnet", "wasm": "owner.wasm",
                })
            });
            let desired = serde_json::from_value(serde_json::json!({
                "bootstrap": null, "canisters": canisters, "cycles_ledger": "ledger",
                "environment": "local", "fleet": "owners", "ledger_fee_cycles": "100M",
                "management_creation_fee_cycles": "500B", "material_cycle_threshold": "1B",
                "maximum_observation_burn_cycles": "1B", "maximum_stalled_observations": 4,
                "maximum_update_burn_cycles": "1B", "operator": "operator", "protocol": null,
                "schema_version": 1, "treasury": "treasury",
            }))
            .unwrap();
            let platform = IcpEnsurePlatform::new(desired, executable.to_str().unwrap(), &root);
            let paths = crate::fleet_ensure::ops::EnsurePaths::under(&root, "local", "owners");
            let state = crate::fleet_ensure::ops::read_state(&paths, "owners").unwrap();
            let fixture = Self {
                root,
                platform,
                state,
            };
            for name in ["coordinator", "root", "store"] {
                fixture.status(name, "Running", true);
            }
            fixture
        }

        fn status(&self, name: &str, status: &str, current_module: bool) {
            let module = if current_module {
                artifact_hash(&self.root.join("owner.wasm")).unwrap()
            } else {
                "ff".repeat(32)
            };
            std::fs::write(
                self.root.join(format!("{name}.json")),
                serde_json::json!({
                    "id": name, "status": status, "settings": { "controllers": [] },
                    "module_hash": module, "cycles": "1000000000000",
                })
                .to_string(),
            )
            .unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one measured transport fixture keeps equivalent retained inventories and call accounting together"
    )]
    fn configured_observation_benchmark_retains_snapshot_call_bound() {
        use std::{
            fs,
            os::unix::fs::PermissionsExt,
            sync::{Arc, Mutex},
        };
        for pool_count in [0_u32, 8, 24, 96] {
            let mut fixture = ProtocolOwnersFixture::new();
            let root_pid = Principal::from_slice(&[2]);
            for (index, canister) in fixture.platform.desired.canisters.iter_mut().enumerate() {
                let pid = Principal::from_slice(&[u8::try_from(index + 1).unwrap()]).to_text();
                let mut status: serde_json::Value = serde_json::from_slice(
                    &fs::read(fixture.root.join(format!("{}.json", canister.name))).unwrap(),
                )
                .unwrap();
                status["id"] = serde_json::Value::String(pid.clone());
                fs::write(fixture.root.join(format!("{pid}.json")), status.to_string()).unwrap();
                canister.principal = Some(pid);
            }
            fixture.platform.desired.protocol =
                Some(crate::fleet_ensure::model::DesiredFleetProtocol {
                    app_config: "unused.toml".to_string(),
                    component_group_placements: Vec::new(),
                    root_candid: "root.did".to_string(),
                    coordinator_candid: "unused.did".to_string(),
                    store_candid: "unused.did".to_string(),
                });
            fs::write(fixture.root.join("root.did"), "service : {}").unwrap();
            let mut entries = Vec::new();
            for index in 0..pool_count {
                let pid = Principal::from_slice(&[u8::try_from(index + 10).unwrap()]);
                let mut configured = fixture.platform.desired.canisters[0].clone();
                configured.name = format!("pool-{index:03}");
                configured.kind = DesiredCanisterKind::Pool;
                configured.principal = Some(pid.to_text());
                configured.parent = Some("root".to_string());
                fixture.platform.desired.canisters.push(configured);
                entries.push(CanisterPoolAsset {
                    canister_id: pid,
                    creation_receipt: None,
                    cycles: Cycles::new(1_000_000_000_000),
                    origin: CanisterPoolAssetOrigin::Imported,
                    status: CanisterPoolAssetStatus::Ready,
                    added_at_ns: 1,
                    updated_at_ns: 1,
                });
            }
            let page = CanisterPoolResponse {
                config: canic_core::ids::FleetSubnetCanisterPoolConfig {
                    minimum_size: 5,
                    maximum_size: 128,
                    canister_cycles: Cycles::new(1_000_000_000_000),
                    creation_execution_margin: Cycles::new(1_000_000_000),
                },
                tracked: pool_count,
                store: 0,
                store_deletion_pending: 0,
                pooled: pool_count,
                workload: 0,
                surplus: 0,
                ready: pool_count,
                pending_reset: 0,
                claimed: 0,
                recycling: 0,
                handing_off: 0,
                failed: 0,
                completed_handoffs: 0,
                pending_creation: None,
                pending_handoff: None,
                entries,
                next_start_after: None,
            };
            let response = Ok::<_, canic_core::dto::error::Error>(RootPoolStatusResponse::Pool(
                Box::new(page),
            ));
            fs::write(fixture.root.join("pool.json"), serde_json::json!({"response_bytes": hex_bytes(candid::encode_one(response).unwrap())}).to_string()).unwrap();
            let executable = fixture.root.join("icp");
            fs::write(&executable, format!(
                "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp 1.3.0'; exit 0; fi\nwhile [ \"$#\" -gt 0 ] && [ \"$1\" != canister ]; do shift; done\nshift\nsleep 0.05\ncase \"$1\" in\nstatus) shift; cat '{}'/\"$1\".json;;\ncall) cat '{}'/pool.json;;\n*) exit 1;;\nesac\n",
                fixture.root.display(), fixture.root.display())).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
            let timings = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&timings);
            fixture.platform = fixture
                .platform
                .with_observation_handler(move |timing| sink.lock().unwrap().push(timing));
            let observed = fixture
                .platform
                .with_observation_snapshot(|platform| {
                    platform
                        .timed_observation(FleetObservationStage::ConfiguredCanisters, |platform| {
                            platform.observe_configured_canisters(&fixture.state)
                        })
                })
                .unwrap();
            assert_eq!(observed.len(), usize::try_from(pool_count).unwrap() + 3);
            assert!(observed.values().all(Option::is_some));
            assert_eq!(
                observed["root"].as_ref().unwrap().principal,
                root_pid.to_text()
            );
            let timing = timings.lock().unwrap().pop().unwrap();
            assert_eq!(timing.remote_call_attempts, 3 + u64::from(pool_count > 0));
            assert!(timing.succeeded);
            println!(
                "configured_observation pool_assets={pool_count} elapsed_ms={} calls={}",
                timing.elapsed_millis, timing.remote_call_attempts
            );
            fs::remove_dir_all(fixture.root).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn configured_observation_failure_drains_batch_and_keeps_first_error() {
        use std::{
            fs,
            sync::{Arc, Mutex},
        };
        let mut fixture = ProtocolOwnersFixture::new();
        let timings = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&timings);
        fixture.platform = fixture
            .platform
            .with_observation_handler(move |timing| sink.lock().unwrap().push(timing));
        let mut first: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.root.join("coordinator.json")).unwrap())
                .unwrap();
        first["id"] = serde_json::json!("different-principal");
        fs::write(fixture.root.join("coordinator.json"), first.to_string()).unwrap();
        fs::remove_file(fixture.root.join("root.json")).unwrap();
        let failure = fixture.platform.with_observation_snapshot(|platform| {
            platform.timed_observation(FleetObservationStage::ConfiguredCanisters, |platform| {
                platform.observe_configured_canisters(&fixture.state)
            })
        });
        assert!(
            matches!(failure, Err(IcpEnsurePlatformError::StatusIdentityMismatch { expected, .. }) if expected == "coordinator")
        );
        let timing = timings.lock().unwrap().pop().unwrap();
        assert!(!timing.succeeded);
        assert_eq!(timing.remote_call_attempts, 3);
        assert!(fixture.platform.observation_snapshot.borrow().is_none());
        fixture.status("coordinator", "Running", true);
        fixture.status("root", "Running", true);
        assert_eq!(
            fixture
                .platform
                .with_observation_snapshot(
                    |platform| platform.observe_configured_canisters(&fixture.state)
                )
                .unwrap()
                .len(),
            3
        );
        assert_eq!(fixture.platform.icp.remote_call_count(), 6);
        fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn protocol_owner_observations_read_each_owner_once_and_refresh_between_calls() {
        let fixture = ProtocolOwnersFixture::new();
        assert!(
            fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        assert_eq!(
            std::fs::read_to_string(fixture.root.join("calls")).unwrap(),
            "coordinator\nroot\nstore\n"
        );
        fixture.status("store", "Stopping", true);
        assert!(
            !fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        fixture.status("store", "Stopped", true);
        assert!(
            !fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        fixture.status("store", "Running", false);
        assert!(
            !fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        fixture.status("store", "Running", true);
        assert!(
            fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        assert_eq!(
            std::fs::read_to_string(fixture.root.join("calls"))
                .unwrap()
                .lines()
                .count(),
            15
        );
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn protocol_owner_observations_preserve_missing_owner_and_transport_failures() {
        let mut fixture = ProtocolOwnersFixture::new();
        fixture.platform.desired.canisters[0].principal = None;
        assert!(
            !fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        std::fs::remove_file(fixture.root.join("store.json")).unwrap();
        assert!(matches!(
            fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state),
            Err(IcpEnsurePlatformError::Icp(_))
        ));
        fixture.platform.desired.canisters[0].principal = Some("coordinator".to_string());
        fixture.status("coordinator", "Running", false);
        assert!(matches!(
            fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state),
            Err(IcpEnsurePlatformError::Icp(_))
        ));
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one transport sequence proves snapshot reuse and expiry across success, failure and effects"
    )]
    fn observation_snapshot_expires_before_effects_and_after_failed_observation() {
        use std::{fs, os::unix::fs::PermissionsExt};

        let root = crate::test_support::temp_dir("canic-observation-snapshot");
        fs::create_dir_all(&root).expect("create observation fixture");
        let executable = root.join("icp");
        let commands = root.join("commands.log");
        let response = root.join("status.json");
        let canister = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let operator = "rdmx6-jaaaa-aaaaa-aaadq-cai";
        let status = |cycles: u128| {
            serde_json::json!({
                "id": canister,
                "status": "Running",
                "settings": { "controllers": [operator] },
                "module_hash": null,
                "cycles": cycles.to_string(),
            })
            .to_string()
        };
        fs::write(&response, status(1_000)).expect("write initial live balance");
        fs::write(&executable, format!(
            "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp 1.3.0'; exit 0; fi\nprintf '%s\\n' \"$*\" >> '{}'\ncat '{}'\n",
            commands.display(), response.display(),
        )).expect("write observation transport");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("make observation transport executable");
        let desired = DesiredFleet {
            bootstrap: None,
            canisters: Vec::new(),
            cycles_ledger: "um5iw-rqaaa-aaaaq-qaaba-cai".to_string(),
            environment: "local".to_string(),
            fleet: "observation-snapshot".to_string(),
            ledger_fee_cycles: "100M".to_string(),
            management_creation_fee_cycles: "500B".to_string(),
            material_cycle_threshold: "1B".to_string(),
            maximum_observation_burn_cycles: "1B".to_string(),
            maximum_stalled_observations: 4,
            maximum_update_burn_cycles: "1B".to_string(),
            operator: operator.to_string(),
            protocol: None,
            protocol_steps: Vec::new(),
            schema_version: 1,
            treasury: "treasury".to_string(),
        };
        let mut platform =
            IcpEnsurePlatform::new(desired, executable.to_str().expect("transport path"), &root);
        let failed: Result<(), IcpEnsurePlatformError> =
            platform.with_observation_snapshot(|platform| {
                assert_eq!(
                    platform
                        .status_optional(canister)?
                        .expect("live canister")
                        .cycles,
                    1_000
                );
                fs::write(&response, status(900)).expect("advance simulated live balance");
                assert_eq!(
                    platform
                        .status_optional(canister)?
                        .expect("same snapshot")
                        .cycles,
                    1_000
                );
                Err(IcpEnsurePlatformError::Arithmetic(
                    "injected observation failure",
                ))
            });
        assert!(matches!(failed, Err(IcpEnsurePlatformError::Arithmetic(_))));
        assert_eq!(
            fs::read_to_string(&commands)
                .expect("first observation calls")
                .lines()
                .count(),
            1
        );
        assert_eq!(
            platform
                .status_optional(canister)
                .expect("fresh action read")
                .expect("live canister")
                .cycles,
            900
        );
        platform
            .with_observation_snapshot(|platform| {
                assert_eq!(
                    platform
                        .status_optional(canister)?
                        .expect("next observation")
                        .cycles,
                    900
                );
                fs::write(&response, status(800)).expect("advance live balance again");
                assert_eq!(
                    platform
                        .status_optional(canister)?
                        .expect("next snapshot")
                        .cycles,
                    900
                );
                Ok(())
            })
            .expect("next observation succeeds");
        assert_eq!(
            platform
                .status_optional(canister)
                .expect("post-observation action read")
                .expect("live canister")
                .cycles,
            800
        );
        assert_eq!(
            fs::read_to_string(&commands)
                .expect("all observation calls")
                .lines()
                .count(),
            4
        );
        fs::remove_dir_all(root).expect("remove observation fixture");
    }

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
    fn fresh_fleet_create_responses_retain_the_exact_requested_balance() {
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
            validate_root_controlled_inspection(
                "pool-0",
                InspectedModule::Empty,
                &root_text,
                &[root],
                None,
                &cycles,
            )
            .expect("exact Root-authorized pool inspection"),
            2_898_749_313_788,
        );

        assert_eq!(
            validate_root_controlled_inspection(
                "pending-reset",
                InspectedModule::Any,
                &root_text,
                &[root],
                Some(&[1]),
                &cycles,
            )
            .expect("installed controlled asset remains part of conservation"),
            2_898_749_313_788,
        );
        let foreign = Principal::from_slice(&[8; 29]);
        for rejected in [
            validate_root_controlled_inspection(
                "pending-reset",
                InspectedModule::Any,
                &root_text,
                &[foreign],
                Some(&[1]),
                &cycles,
            ),
            validate_root_controlled_inspection(
                "pool-0",
                InspectedModule::Empty,
                &root_text,
                &[foreign],
                None,
                &cycles,
            ),
            validate_root_controlled_inspection(
                "pool-0",
                InspectedModule::Empty,
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

    #[test]
    fn pending_fresh_pool_observation_accepts_only_its_reviewed_controller_transition() {
        let root = Principal::from_slice(&[7; 29]);
        let operator = Principal::from_slice(&[8; 29]);
        let foreign = Principal::from_slice(&[9; 29]);
        for controllers in [vec![root, operator], vec![root]] {
            assert_eq!(
                validate_pending_fresh_pool_inspection(
                    "pool",
                    &root.to_text(),
                    &operator.to_text(),
                    &controllers,
                    None,
                    &Nat::from(17_u8),
                )
                .expect("issued fresh controller transition"),
                17,
            );
        }
        for controllers in [
            vec![root, foreign],
            vec![operator],
            vec![root, operator, foreign],
            vec![root, root],
        ] {
            assert!(matches!(
                validate_pending_fresh_pool_inspection(
                    "pool",
                    &root.to_text(),
                    &operator.to_text(),
                    &controllers,
                    None,
                    &Nat::from(17_u8),
                ),
                Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict { .. })
            ));
        }
    }
}
