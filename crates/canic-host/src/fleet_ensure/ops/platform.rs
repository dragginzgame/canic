//! Module: fleet_ensure::ops::platform
//!
//! Responsibility: mechanically observe and mutate the exact current Fleet through ICP CLI.
//! Does not own: effect ordering, durable intent, retry policy, or plan approval.
//! Boundary: the workflow calls one method only after persisting its exact action identity.

use crate::icp::cycles_ledger::{
    CanisterSettings, CmcCreateCanisterArgs, CreateCanisterArgs, CreateCanisterError,
    CreateCanisterSuccess, SubnetSelection,
};
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
    subnet_catalog::load_cached_mainnet_subnet_catalog,
};
use candid::{CandidType, Nat, Principal};
use canic_core::{
    cdk::{types::Cycles, utils::hash::hex_bytes},
    dto::canister::CanisterInspectionRequest,
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
    cell::{Cell, RefCell},
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

#[derive(CandidType, Clone, Deserialize)]
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

/// Protected management evidence shared by Root-owned observation and pool funding.
#[derive(CandidType, Deserialize)]
enum RootInspectionResponse {
    InspectCanister(RootInspectionStatus),
    InspectionReserveRequired(canic_core::dto::canister::CanisterInspectionReserveResponse),
}

impl RootInspectionResponse {
    fn into_status(
        self,
        root: Principal,
        target: Principal,
    ) -> Result<RootInspectionStatus, crate::CanisterProtocolError> {
        match self {
            Self::InspectCanister(status) => Ok(status),
            Self::InspectionReserveRequired(evidence) => Err(
                crate::CanisterProtocolError::inspection_reserve(root, target, evidence),
            ),
        }
    }
}

#[derive(CandidType, Clone, Deserialize)]
struct RootInspectionStatus {
    status: canic_core::dto::canister::CanisterStatusType,
    settings: ManagementCanisterObservationSettings,
    module_hash: Option<Vec<u8>>,
    cycles: Nat,
}

/// First input-ordered mismatch or read failure from a drained reset verification batch.
enum ReinstallAssetVerificationError {
    Mismatch,
    Read(Box<IcpEnsurePlatformError>),
}

/// Fresh management evidence validated for one retained reset asset.
struct ReinstallAssetStatus {
    response: RootInspectionStatus,
    controllers: Vec<String>,
    cycles: u128,
}

#[derive(CandidType)]
enum ManagedCanisterStatusRequest {
    CycleBalance,
}

#[derive(CandidType, Deserialize)]
enum ManagedCanisterStatusResponse {
    CycleBalance(canic_core::dto::role::CycleBalanceStatusResponse),
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
    #[error(transparent)]
    RetirementDebit(#[from] crate::fleet_ensure::ops::reinstall::debit::RetirementDebitError),
    #[error(transparent)]
    FundingObservation(
        #[from] crate::fleet_ensure::model::funding_observation::FundingObservationError,
    ),

    #[error("mainnet Subnet Catalog observation failed: {0}")]
    SubnetCatalog(#[source] Box<ic_query::subnet_catalog::SubnetCatalogLoadFailure>),

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
    pool_inspections: BTreeMap<PoolInspectionAuthority, RootInspectionStatus>,
    pool_pages: BTreeMap<(Principal, Option<Principal>), CanisterPoolResponse>,
    statuses: BTreeMap<String, Option<LiveCanister>>,
}

/// An inspection response belongs to the exact protected Root and requested asset.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct PoolInspectionAuthority {
    root: Principal,
    target: Principal,
}

impl PoolInspectionAuthority {
    fn parse(root: &str, target: &str) -> Result<Self, IcpEnsurePlatformError> {
        Ok(Self {
            root: parse_principal("Fleet Subnet Root", root)?,
            target: parse_principal("Root-owned inspection target", target)?,
        })
    }
}

/// Consumer requirements kept separate from the transport result shared by one batch.
enum PoolInspectionRequirement {
    PendingFresh,
    RootControlled(InspectedModule),
}

/// Prepared only after the caller's Root and operator checks succeed.
struct PreparedPoolInspection<'a> {
    authority: PoolInspectionAuthority,
    name: &'a str,
    root: &'a str,
    requirement: PoolInspectionRequirement,
}

/// A configured pending asset awaiting its independent management inspection.
struct PendingPoolObservation<'a> {
    configured: &'a crate::fleet_ensure::model::DesiredCanister,
    principal: &'a str,
    asset: CanisterPoolAsset,
    inspection: PreparedPoolInspection<'a>,
}

/// Pool-page and Root checks stay ordered before independent inspections overlap.
enum PreparedRootOwnedObservation<'a> {
    Observed(Option<LiveCanister>),
    Pending(Box<PendingPoolObservation<'a>>),
}

/// Completed transport outcomes local to one drained inspection batch.
#[derive(Default)]
struct PoolInspectionBatch {
    responses:
        BTreeMap<PoolInspectionAuthority, Result<RootInspectionStatus, IcpEnsurePlatformError>>,
}

/// Distinguish an unobserved identity from an observed missing canister.
enum StatusCacheEntry {
    Miss,
    Hit(Option<LiveCanister>),
}

/// Cumulative diagnostic counters sampled at observation boundaries.
struct ObservationCounters {
    remote_calls: u64,
    identity_lookups: u64,
    identity_lookup_millis: u64,
    cached_reads: u64,
}

/// Production ICP adapter for the current desired Fleet.
pub struct IcpEnsurePlatform {
    retirement_debit_block: Option<u64>,
    pub(super) desired: DesiredFleet,
    pub(super) icp: IcpCli,
    initial_observation_delay: Duration,
    maximum_observation_delay: Duration,
    observation_snapshot: RefCell<Option<FleetObservationSnapshot>>,
    staging_observations: Option<current_protocol::StoreStagingObservations>,
    cached_reads: Cell<u64>,
    observation_stages: Vec<(u64, FleetObservationStage)>,
    progress_handler: Option<Box<dyn FnMut(FleetEnsureProgress)>>,
    observation_handler: Option<Box<dyn FnMut(FleetObservationTiming)>>,
    estate_observations: BTreeMap<String, EstateFundingDomainObservation>,
    pub(super) root: PathBuf,
}

static NEXT_SPAN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

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
            retirement_debit_block: None,
            initial_observation_delay: INITIAL_PROTOCOL_OBSERVATION_DELAY,
            maximum_observation_delay: MAXIMUM_PROTOCOL_OBSERVATION_DELAY,
            observation_snapshot: RefCell::new(None),
            staging_observations: None,
            cached_reads: Cell::new(0),
            observation_stages: Vec::new(),
            progress_handler: None,
            observation_handler: None,
            estate_observations: BTreeMap::new(),
            root: root.to_path_buf(),
        }
    }

    /// Select one external debit for a fresh source retirement review, never an apply override.
    #[must_use]
    pub const fn with_retirement_debit(mut self, block: Option<u64>) -> Self {
        self.retirement_debit_block = block;
        self
    }

    /// Select the signer before the operation's existing Principal admission check.
    #[must_use]
    pub fn with_identity(mut self, identity: Option<&str>) -> Self {
        self.icp = self.icp.with_identity(identity);
        self
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

    /// Attach request timing to the same transport used by all worker clones.
    #[must_use]
    pub fn with_request_timing_handler(
        mut self,
        handler: impl Fn(crate::icp::IcpRequestTiming) + Send + Sync + 'static,
    ) -> Self {
        self.icp = self.icp.with_timing_handler(handler);
        self
    }

    fn timed_observation<T>(
        &mut self,
        stage: FleetObservationStage,
        observe: impl FnOnce(&mut Self) -> Result<T, IcpEnsurePlatformError>,
    ) -> Result<T, IcpEnsurePlatformError> {
        self.measure_observation(stage, observe)
    }

    fn measure_observation<T, E>(
        &mut self,
        stage: FleetObservationStage,
        observe: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        let started = std::time::Instant::now();
        let before = ObservationCounters {
            remote_calls: self.icp.remote_call_count(),
            identity_lookups: self.icp.identity_lookup_count(),
            identity_lookup_millis: self.icp.identity_lookup_millis(),
            cached_reads: self.cached_reads.get(),
        };
        let span_id = NEXT_SPAN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let parent = self.observation_stages.last().copied();
        let mut timing = FleetObservationTiming {
            span_id,
            parent_span_id: parent.map(|entry| entry.0),
            stage,
            parent_stage: parent.map(|entry| entry.1),
            elapsed_millis: 0,
            remote_call_attempts: 0,
            identity_lookup_attempts: 0,
            identity_lookup_millis: 0,
            cached_read_hits: 0,
            succeeded: None,
        };
        if let Some(handler) = &mut self.observation_handler {
            handler(timing.clone());
        }
        self.observation_stages.push((span_id, stage));
        let result = observe(self);
        self.observation_stages.pop();
        timing.elapsed_millis = started.elapsed().as_millis();
        timing.remote_call_attempts = self
            .icp
            .remote_call_count()
            .saturating_sub(before.remote_calls);
        timing.identity_lookup_attempts = self
            .icp
            .identity_lookup_count()
            .saturating_sub(before.identity_lookups);
        timing.identity_lookup_millis = self
            .icp
            .identity_lookup_millis()
            .saturating_sub(before.identity_lookup_millis);
        timing.cached_read_hits = self.cached_reads.get().saturating_sub(before.cached_reads);
        timing.succeeded = Some(result.is_ok());
        if let Some(handler) = &mut self.observation_handler {
            handler(timing);
        }
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

    // Nested stages share one explicit read-only scope. The owner expires it on
    // every Result path; retries and effects invalidate it independently.
    fn with_observation_snapshot<T>(
        &mut self,
        observe: impl FnOnce(&mut Self) -> Result<T, IcpEnsurePlatformError>,
    ) -> Result<T, IcpEnsurePlatformError> {
        self.with_readonly_snapshot(observe)
    }

    fn with_readonly_snapshot<T, E>(
        &mut self,
        observe: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        if self.observation_snapshot.borrow().is_some() {
            return observe(self);
        }
        self.observation_snapshot
            .replace(Some(FleetObservationSnapshot::default()));
        let result = observe(self);
        self.observation_snapshot.take();
        result
    }

    fn invalidate_observation_snapshot(&self) {
        if let Some(snapshot) = self.observation_snapshot.borrow_mut().as_mut() {
            *snapshot = FleetObservationSnapshot::default();
        }
    }

    fn record_cached_read(&self) {
        self.cached_reads
            .set(self.cached_reads.get().saturating_add(1));
    }

    pub(super) fn require_operator(&self) -> Result<(), IcpEnsurePlatformError> {
        self.icp.bind_selected_identity()?;
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
        let configured = self
            .desired
            .canisters
            .iter()
            .filter(|configured| {
                configured.presence == crate::fleet_ensure::model::DesiredPresence::Present
                    && matches!(
                        configured.kind,
                        DesiredCanisterKind::Coordinator
                            | DesiredCanisterKind::Root
                            | DesiredCanisterKind::Store
                    )
            })
            .collect::<Vec<_>>();
        let mut owners = Vec::with_capacity(configured.len());
        for batch in configured.chunks(super::bounded_observations::MAX_IN_FLIGHT) {
            let mut statuses = self.read_status_batch(
                batch
                    .iter()
                    .filter_map(|owner| self.current_principal(state, &owner.name)),
            );
            // Consume in configured order after draining this batch. A stopped owner
            // still precedes a later transport error; neither starts another batch.
            for configured in batch {
                let live = self
                    .current_principal(state, &configured.name)
                    .map(|principal| self.status_batch_response(&mut statuses, principal))
                    .transpose()?
                    .flatten();
                if live
                    .as_ref()
                    .is_some_and(|live| live.status != CanisterRuntimeStatus::Running)
                {
                    return Ok(false);
                }
                owners.push((*configured, live));
            }
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

    pub(super) fn query_estate_pool_page(
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
            self.record_cached_read();
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

    fn reinstall_assets_match_bound(
        icp: &IcpCli,
        candid: &Path,
        principal: Principal,
        assets: &[&crate::fleet_ensure::model::FleetReinstallAssetRecord],
        check: super::ReinstallAssetCheck,
    ) -> Result<bool, IcpEnsurePlatformError> {
        let result =
            super::bounded_observations::collect(
                assets,
                |asset| match Self::reinstall_asset_matches(icp, candid, principal, asset, check) {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(ReinstallAssetVerificationError::Mismatch),
                    Err(error) => Err(ReinstallAssetVerificationError::Read(Box::new(error))),
                },
            );
        match result {
            Ok(_) => Ok(true),
            Err(ReinstallAssetVerificationError::Mismatch) => Ok(false),
            Err(ReinstallAssetVerificationError::Read(error)) => Err(*error),
        }
    }

    fn reinstall_asset_matches(
        icp: &IcpCli,
        candid: &Path,
        principal: Principal,
        asset: &crate::fleet_ensure::model::FleetReinstallAssetRecord,
        check: super::ReinstallAssetCheck,
    ) -> Result<bool, IcpEnsurePlatformError> {
        let target = parse_principal("reset asset", &asset.principal)?;
        icp.measure_canister_inspection(principal, target, || {
            crate::canister_protocol::inspection::preflight_inspection(
                icp, candid, principal, target,
            )
            .map_err(current_protocol::CurrentProtocolError::from)?;
            let response: RootInspectionResponse = call_with_candid(
                icp,
                candid,
                principal,
                canic_protocol::CANIC_ROOT_COMMAND,
                &RootInspectionCommand::InspectCanister(CanisterInspectionRequest {
                    canister_id: target,
                }),
            )
            .map_err(current_protocol::CurrentProtocolError::from)?;
            let status = response
                .into_status(principal, target)
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
            Ok(controllers == asset.controllers && module_matches)
        })
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
            let recovery_controllers = self
                .desired
                .bootstrap
                .as_ref()
                .map_or(&[][..], |bootstrap| {
                    bootstrap.recovery_controllers.as_slice()
                });
            for batch in pool
                .assets
                .chunks(super::bounded_observations::MAX_IN_FLIGHT)
            {
                // Record uniqueness in inventory order, then drain each issued batch
                // before mutating the observation or scheduling another inspection.
                let requests = batch
                    .iter()
                    .map(|asset| (asset, seen.insert(asset.principal.clone())))
                    .collect::<Vec<_>>();
                let icp = &self.icp;
                let responses =
                    super::bounded_observations::collect(&requests, |(asset, unique)| {
                        Self::inspect_reinstall_asset(
                            icp,
                            candid_by_root.get(root_name).map(PathBuf::as_path),
                            root,
                            asset,
                            *unique,
                            allow_pending_reset,
                            recovery_controllers,
                        )
                    })?;
                for (asset, inspected) in batch.iter().zip(responses) {
                    let ReinstallAssetStatus {
                        response,
                        controllers,
                        cycles,
                    } = inspected;

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
        }
        assets.sort_by(|a, b| a.principal.cmp(&b.principal));
        Ok(assets)
    }

    fn inspect_reinstall_asset(
        icp: &IcpCli,
        candid: Option<&Path>,
        root: &str,
        asset: &EstatePoolAssetObservation,
        unique: bool,
        allow_pending_reset: bool,
        recovery_controllers: &[Principal],
    ) -> Result<ReinstallAssetStatus, IcpEnsurePlatformError> {
        let terminal = matches!(
            asset.lifecycle,
            EstatePoolAssetLifecycle::Ready | EstatePoolAssetLifecycle::Workload
        );
        let recoverable = allow_pending_reset
            && matches!(
                asset.lifecycle,
                EstatePoolAssetLifecycle::PendingReset | EstatePoolAssetLifecycle::Failed
            );
        if !unique || !(terminal || recoverable) {
            return Err(IcpEnsurePlatformError::RootManagement(
                "pool asset is duplicated or not terminal".to_string(),
            ));
        }
        let authority = PoolInspectionAuthority {
            target: parse_principal("reset pool asset", &asset.principal)?,
            root: parse_principal("reset Root", root)?,
        };
        let candid = candid
            .ok_or_else(|| pool_configuration_error("missing reset read contract".to_string()))?;
        // Reset review always fetches fresh evidence; it does not consume the
        // earlier observation's balance or inspection cache.
        let response = Self::fetch_pool_inspection(icp, candid, authority)?;
        let mut controllers = response
            .settings
            .controllers
            .iter()
            .map(Principal::to_text)
            .collect::<Vec<_>>();
        if !controllers.iter().any(|controller| controller == root) {
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
        controllers.sort();
        let mut expected = recovery_controllers
            .iter()
            .map(Principal::to_text)
            .collect::<Vec<_>>();
        expected.push(root.to_string());
        expected.sort();
        expected.dedup();
        if controllers != expected {
            return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                canister: asset.principal.clone(),
                field: "exact Root controllers",
            });
        }
        Ok(ReinstallAssetStatus {
            response,
            controllers,
            cycles,
        })
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
            self.record_cached_read();
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

    pub(super) fn read_status_optional(
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
    ) -> Result<Option<RootInspectionStatus>, IcpEnsurePlatformError> {
        self.root_owned_inspection_authority(configured, principal, state)?
            .map(|authority| self.read_pool_inspection(authority))
            .transpose()
    }

    fn root_owned_inspection_authority(
        &self,
        configured: &crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<PoolInspectionAuthority>, IcpEnsurePlatformError> {
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
        PoolInspectionAuthority::parse(root, principal).map(Some)
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
            self.desired
                .bootstrap
                .as_ref()
                .map_or(&[], |bootstrap| &bootstrap.recovery_controllers),
        )?;
        Ok((Some(cycles), false))
    }

    fn install_status_optional(
        &self,
        principal: &str,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let Some(live) = self.status_optional(principal)? else {
            return Ok(None);
        };
        self.complete_install_status(principal, live).map(Some)
    }

    fn complete_install_status(
        &self,
        principal: &str,
        mut live: LiveCanister,
    ) -> Result<LiveCanister, IcpEnsurePlatformError> {
        let exact = exact_install_canister_status(
            &self.icp,
            principal,
            live.canister_version,
            live.module_sha256.clone(),
        )?;
        live.canister_version = Some(exact.canister_version);
        live.module_sha256 = exact.module_sha256;
        Ok(live)
    }

    // A direct response can also supply the reconciliation balance, except where
    // action_cycles must establish the configured Pool's protected Root authority.
    fn direct_reconciliation_cycles(
        &self,
        name: &str,
        live: Option<&LiveCanister>,
    ) -> Option<u128> {
        let configured = self.desired.canisters.iter().find(|c| c.name == name);
        if configured.is_some_and(|c| c.kind == DesiredCanisterKind::Pool) {
            return None;
        }
        live.map(|live| live.cycles)
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
        match self.prepare_root_owned_observation(configured, principal, state)? {
            PreparedRootOwnedObservation::Observed(live) => Ok(live),
            PreparedRootOwnedObservation::Pending(pending) => {
                let response = self.read_pool_inspection(pending.inspection.authority)?;
                self.complete_pending_pool_observation(*pending, response)
            }
        }
    }

    fn prepare_root_owned_observation<'a>(
        &'a self,
        configured: &'a crate::fleet_ensure::model::DesiredCanister,
        principal: &'a str,
        state: &'a FleetEnsureStateRecord,
    ) -> Result<PreparedRootOwnedObservation<'a>, IcpEnsurePlatformError> {
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
                .retained_root_owned_observation(configured, principal, parent, root, state)
                .map(PreparedRootOwnedObservation::Observed);
        }
        let target = parse_principal("Root-owned canister", principal)?;
        let mut start_after = None;
        loop {
            let page = self.query_estate_pool_page(
                &candid,
                parse_principal("Fleet Subnet Root", root)?,
                start_after,
            )?;
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
                    let inspection =
                        self.prepare_pending_pool_inspection(configured, principal, state)?;
                    return Ok(PreparedRootOwnedObservation::Pending(Box::new(
                        PendingPoolObservation {
                            configured,
                            principal,
                            asset,
                            inspection,
                        },
                    )));
                }
                return Self::observed_root_owned_asset(configured, principal, root, asset)
                    .map(PreparedRootOwnedObservation::Observed);
            }
            let next = page.next_start_after;
            if next.is_none() {
                return Ok(PreparedRootOwnedObservation::Observed(None));
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

    fn complete_pending_pool_observation(
        &self,
        mut pending: PendingPoolObservation<'_>,
        response: RootInspectionStatus,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let root = pending.inspection.root;
        let cycles = self.complete_pool_inspection(pending.inspection, response)?;
        pending.asset.cycles = Cycles::new(cycles);
        Self::observed_root_owned_asset(pending.configured, pending.principal, root, pending.asset)
    }

    fn prepare_pending_pool_inspection<'a>(
        &'a self,
        configured: &'a crate::fleet_ensure::model::DesiredCanister,
        principal: &str,
        state: &'a FleetEnsureStateRecord,
    ) -> Result<PreparedPoolInspection<'a>, IcpEnsurePlatformError> {
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
            let authority = self
                .root_owned_inspection_authority(configured, principal, state)?
                .ok_or_else(
                    || IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        canister: configured.name.clone(),
                        field: "installed Root",
                    },
                )?;
            return Ok(PreparedPoolInspection {
                authority,
                name: &configured.name,
                root,
                requirement: PoolInspectionRequirement::PendingFresh,
            });
        }
        self.prepare_pool_balance_inspection(
            &configured.name,
            root,
            principal,
            InspectedModule::Any,
        )
    }

    fn inspect_pool_balance(
        &self,
        name: &str,
        root: &str,
        principal: &str,
        module: InspectedModule,
    ) -> Result<u128, IcpEnsurePlatformError> {
        let inspection = self.prepare_pool_balance_inspection(name, root, principal, module)?;
        let response = self.read_pool_inspection(inspection.authority)?;
        self.complete_pool_inspection(inspection, response)
    }

    fn refresh_pool_balances(
        &self,
        root: &str,
        assets: &mut [EstatePoolAssetObservation],
    ) -> Result<(), IcpEnsurePlatformError> {
        let indices = assets
            .iter()
            .enumerate()
            .filter_map(|(index, asset)| {
                matches!(
                    asset.lifecycle,
                    EstatePoolAssetLifecycle::PendingReset | EstatePoolAssetLifecycle::Failed
                )
                .then_some(index)
            })
            .collect::<Vec<_>>();
        for batch in indices.chunks(super::bounded_observations::MAX_IN_FLIGHT) {
            // One shared authority check precedes this batch's independent reads.
            // The next batch, observation and effect all establish authority again.
            self.require_pool_balance_authority(&assets[batch[0]].principal, root)?;
            let prepared = batch
                .iter()
                .map(|index| {
                    let target = &assets[*index].principal;
                    Ok::<_, IcpEnsurePlatformError>(PreparedPoolInspection {
                        authority: PoolInspectionAuthority::parse(root, target)?,
                        name: target,
                        root,
                        requirement: PoolInspectionRequirement::RootControlled(
                            InspectedModule::Any,
                        ),
                    })
                })
                .collect::<Vec<_>>();
            let authorities = prepared
                .iter()
                .filter_map(|result| result.as_ref().ok().map(|inspection| inspection.authority))
                .collect::<Vec<_>>();
            let mut responses = self.read_pool_inspection_batch(&authorities)?;
            // Drain issued reads, then validate in inventory order before publishing
            // this batch's balances. A failed batch never starts the next group.
            let balances = prepared
                .into_iter()
                .map(|inspection| {
                    let inspection = inspection?;
                    let response =
                        self.pool_inspection_batch_response(&mut responses, inspection.authority)?;
                    self.complete_pool_inspection(inspection, response)
                })
                .collect::<Result<Vec<_>, IcpEnsurePlatformError>>()?;
            for (index, cycles) in batch.iter().zip(balances) {
                assets[*index].cycles = cycles;
            }
        }
        Ok(())
    }

    fn prepare_pool_balance_inspection<'a>(
        &self,
        name: &'a str,
        root: &'a str,
        principal: &str,
        module: InspectedModule,
    ) -> Result<PreparedPoolInspection<'a>, IcpEnsurePlatformError> {
        self.require_pool_balance_authority(name, root)?;
        Ok(PreparedPoolInspection {
            authority: PoolInspectionAuthority::parse(root, principal)?,
            name,
            root,
            requirement: PoolInspectionRequirement::RootControlled(module),
        })
    }

    fn require_pool_balance_authority(
        &self,
        name: &str,
        root: &str,
    ) -> Result<(), IcpEnsurePlatformError> {
        if self.required_root_status(name, root)? != CanisterRuntimeStatus::Running {
            return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                canister: name.to_string(),
                field: "running Root",
            });
        }
        self.require_operator()
    }

    fn complete_pool_inspection(
        &self,
        inspection: PreparedPoolInspection<'_>,
        response: RootInspectionStatus,
    ) -> Result<u128, IcpEnsurePlatformError> {
        let cycles = match inspection.requirement {
            PoolInspectionRequirement::PendingFresh => validate_pending_fresh_pool_inspection(
                inspection.name,
                inspection.root,
                &self.desired.operator,
                &response.settings.controllers,
                response.module_hash.as_deref(),
                &response.cycles,
                self.desired
                    .bootstrap
                    .as_ref()
                    .map_or(&[], |bootstrap| &bootstrap.recovery_controllers),
            ),
            PoolInspectionRequirement::RootControlled(module) => {
                validate_root_controlled_inspection(
                    inspection.name,
                    module,
                    inspection.root,
                    &response.settings.controllers,
                    response.module_hash.as_deref(),
                    &response.cycles,
                    self.desired
                        .bootstrap
                        .as_ref()
                        .map_or(&[], |bootstrap| &bootstrap.recovery_controllers),
                )
            }
        }?;
        self.retain_pool_inspection(inspection.authority, response);
        Ok(cycles)
    }

    // Read raw evidence only after the caller has established its Root/operator authority.
    fn read_pool_inspection(
        &self,
        authority: PoolInspectionAuthority,
    ) -> Result<RootInspectionStatus, IcpEnsurePlatformError> {
        if let Some(response) = self
            .observation_snapshot
            .borrow()
            .as_ref()
            .and_then(|snapshot| snapshot.pool_inspections.get(&authority).cloned())
        {
            self.record_cached_read();
            return Ok(response);
        }
        Self::fetch_pool_inspection(&self.icp, &self.root_protocol_candid()?, authority)
    }

    fn fetch_pool_inspection(
        icp: &IcpCli,
        candid: &Path,
        authority: PoolInspectionAuthority,
    ) -> Result<RootInspectionStatus, IcpEnsurePlatformError> {
        icp.measure_canister_inspection(authority.root, authority.target, || {
            crate::canister_protocol::inspection::preflight_inspection(
                icp,
                candid,
                authority.root,
                authority.target,
            )
            .map_err(current_protocol::CurrentProtocolError::from)?;
            let response: RootInspectionResponse = call_with_candid(
                icp,
                candid,
                authority.root,
                canic_protocol::CANIC_ROOT_COMMAND,
                &RootInspectionCommand::InspectCanister(CanisterInspectionRequest {
                    canister_id: authority.target,
                }),
            )
            .map_err(current_protocol::CurrentProtocolError::from)?;
            response
                .into_status(authority.root, authority.target)
                .map_err(current_protocol::CurrentProtocolError::from)
                .map_err(Into::into)
        })
    }

    // Only a successfully validated consumer may retain a response in this observation.
    fn retain_pool_inspection(
        &self,
        authority: PoolInspectionAuthority,
        response: RootInspectionStatus,
    ) {
        if let Some(snapshot) = self.observation_snapshot.borrow_mut().as_mut() {
            snapshot.pool_inspections.insert(authority, response);
        }
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
            | RootOwnedCanisterLifecycle::Reconciling
            | RootOwnedCanisterLifecycle::Retained => CanisterRuntimeStatus::Stopped,
        };
        Ok(Some(LiveCanister {
            canister_version: None,
            controllers: {
                let mut controllers = configured.controllers.clone();
                controllers.push(root.to_string());
                controllers.sort();
                controllers.dedup();
                controllers
            },
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
            controllers: {
                let mut controllers = configured.controllers.clone();
                controllers.push(root.to_string());
                controllers.sort();
                controllers.dedup();
                controllers
            },
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
                .filter(|root| {
                    let mut expected = configured.controllers.clone();
                    expected.push((*root).clone());
                    expected.sort();
                    expected.dedup();
                    live.controllers == expected
                })
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
        self.icp.record_remote_call();
        let result = self
            .icp
            .measure_request(
                crate::icp::IcpRequestKind::Install,
                Some(principal),
                None,
                || run_status(&mut command),
            )
            .map_err(IcpEnsurePlatformError::from);
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
        self.icp.record_remote_call();
        self.icp.measure_request(
            crate::icp::IcpRequestKind::SetControllers,
            Some(principal),
            None,
            || run_status(&mut command),
        )?;
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
                    platform.refresh_pool_balances(root, &mut pool.assets)?;
                    for asset in &mut pool.assets {
                        if matches!(
                            asset.lifecycle,
                            EstatePoolAssetLifecycle::PendingReset
                                | EstatePoolAssetLifecycle::Failed
                        ) {
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
            let pool_batch = configured[cursor].kind == DesiredCanisterKind::Pool;
            let end = configured[cursor..]
                .iter()
                .take(super::bounded_observations::MAX_IN_FLIGHT)
                .take_while(|canister| (canister.kind == DesiredCanisterKind::Pool) == pool_batch)
                .count()
                + cursor;
            if pool_batch {
                self.observe_configured_pool_batch(&configured[cursor..end], state, &mut observed)?;
                cursor = end;
                continue;
            }
            self.observe_configured_status_batch(&configured[cursor..end], state, &mut observed)?;
            cursor = end;
        }
        Ok(observed)
    }

    fn observe_configured_pool_batch(
        &self,
        configured: &[crate::fleet_ensure::model::DesiredCanister],
        state: &FleetEnsureStateRecord,
        observed: &mut BTreeMap<String, Option<LiveCanister>>,
    ) -> Result<(), IcpEnsurePlatformError> {
        // Preparation and result consumption follow desired order. An earlier transport
        // failure must still precede a later preparation failure, regardless of arrival.
        let prepared = configured
            .iter()
            .map(|canister| {
                self.current_principal(state, &canister.name).map_or(
                    Ok(PreparedRootOwnedObservation::Observed(None)),
                    |principal| self.prepare_root_owned_observation(canister, principal, state),
                )
            })
            .collect::<Vec<_>>();
        let authorities = prepared
            .iter()
            .filter_map(|result| match result {
                Ok(PreparedRootOwnedObservation::Pending(pending)) => {
                    Some(pending.inspection.authority)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut responses = self.read_pool_inspection_batch(&authorities)?;
        for (canister, prepared) in configured.iter().zip(prepared) {
            let live = match prepared? {
                PreparedRootOwnedObservation::Observed(live) => live,
                PreparedRootOwnedObservation::Pending(pending) => {
                    let response = self.pool_inspection_batch_response(
                        &mut responses,
                        pending.inspection.authority,
                    )?;
                    self.complete_pending_pool_observation(*pending, response)?
                }
            };
            observed.insert(canister.name.clone(), live);
        }
        Ok(())
    }

    fn read_pool_inspection_batch(
        &self,
        authorities: &[PoolInspectionAuthority],
    ) -> Result<PoolInspectionBatch, IcpEnsurePlatformError> {
        let mut seen = BTreeSet::new();
        let requests = authorities
            .iter()
            .copied()
            .filter(|authority| {
                seen.insert(*authority)
                    && !self
                        .observation_snapshot
                        .borrow()
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.pool_inspections.contains_key(authority))
            })
            .collect::<Vec<_>>();
        if requests.is_empty() {
            return Ok(PoolInspectionBatch::default());
        }
        let candid = self.root_protocol_candid()?;
        let icp = &self.icp;
        Ok(PoolInspectionBatch {
            responses: super::bounded_observations::collect(&requests, |authority| {
                Ok::<_, std::convert::Infallible>((
                    *authority,
                    Self::fetch_pool_inspection(icp, &candid, *authority),
                ))
            })
            .expect("inspection results are retained for ordered authority decisions")
            .into_iter()
            .collect(),
        })
    }

    fn pool_inspection_batch_response(
        &self,
        batch: &mut PoolInspectionBatch,
        authority: PoolInspectionAuthority,
    ) -> Result<RootInspectionStatus, IcpEnsurePlatformError> {
        match batch.responses.get(&authority) {
            Some(Ok(response)) => Ok(response.clone()),
            Some(Err(_)) => batch
                .responses
                .remove(&authority)
                .expect("observed batch failure"),
            None => self.read_pool_inspection(authority),
        }
    }

    fn observe_configured_status_batch(
        &self,
        configured: &[crate::fleet_ensure::model::DesiredCanister],
        state: &FleetEnsureStateRecord,
        observed: &mut BTreeMap<String, Option<LiveCanister>>,
    ) -> Result<(), IcpEnsurePlatformError> {
        let mut results = self.read_status_batch(
            configured
                .iter()
                .filter_map(|canister| self.current_principal(state, &canister.name)),
        );
        for canister in configured {
            let Some(principal) = self.current_principal(state, &canister.name) else {
                observed.insert(canister.name.clone(), None);
                continue;
            };
            let status = self.status_batch_response(&mut results, principal);
            let live = self.configured_status_or_root(canister, principal, state, status)?;
            observed.insert(canister.name.clone(), live);
        }
        Ok(())
    }

    fn read_status_batch<'a>(
        &self,
        principals: impl Iterator<Item = &'a str>,
    ) -> BTreeMap<&'a str, Result<Option<LiveCanister>, IcpEnsurePlatformError>> {
        let mut seen = BTreeSet::new();
        let requests = principals
            .filter(|principal| seen.insert(*principal))
            .filter(|principal| matches!(self.cached_status(principal), StatusCacheEntry::Miss))
            .collect::<Vec<_>>();
        let icp = &self.icp;
        super::bounded_observations::collect(&requests, |principal| {
            Ok::<_, std::convert::Infallible>((*principal, Self::read_status_with(icp, principal)))
        })
        .expect("read outcomes are retained for ordered authority decisions")
        .into_iter()
        .collect()
    }

    fn status_batch_response(
        &self,
        results: &mut BTreeMap<&str, Result<Option<LiveCanister>, IcpEnsurePlatformError>>,
        principal: &str,
    ) -> Result<Option<LiveCanister>, IcpEnsurePlatformError> {
        let status = if let StatusCacheEntry::Hit(cached) = self.cached_status(principal) {
            self.record_cached_read();
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
        status
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

    fn retirement_debit_block(&self) -> Option<u64> {
        self.retirement_debit_block
    }

    fn observe_retirement_debit(
        &mut self,
        block: u64,
    ) -> Result<Option<crate::fleet_ensure::model::RetirementWithdrawalRecord>, Self::Error> {
        Ok(Some(crate::fleet_ensure::ops::reinstall::debit::observe(
            &self.icp,
            &self.desired.cycles_ledger,
            &self.desired.operator,
            block,
        )?))
    }

    fn with_planning_observations<T, E>(
        &mut self,
        observe: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        self.measure_observation(FleetObservationStage::Planning, |platform| {
            platform.with_readonly_snapshot(observe)
        })
    }

    fn with_preparation_observations<T>(
        &mut self,
        action: &EnsureAction,
        observe: impl FnOnce(&mut Self) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error> {
        if matches!(action, EnsureAction::Install { .. }) {
            // A single fresh status supplies balance and version before intent. Even
            // an enclosing scope must not carry older authority into preparation.
            self.observation_snapshot.take();
            self.with_observation_snapshot(observe)
        } else {
            observe(self)
        }
    }

    fn with_independent_observations<T, E>(
        &mut self,
        observe: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        // Every pass starts fresh, including retries and nested invocations.
        self.staging_observations = Some(current_protocol::StoreStagingObservations::default());
        let result = observe(self);
        self.staging_observations = None;
        result
    }

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
        self.invalidate_observation_snapshot();
        self.desired = desired.clone();
        Ok(())
    }

    fn with_activity<T, E>(
        &mut self,
        stage: FleetObservationStage,
        activity: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        self.measure_observation(stage, activity)
    }

    fn pace_effect_observation(
        &mut self,
        _action: &EnsureAction,
        consecutive_unchanged_observations: u32,
    ) {
        self.invalidate_observation_snapshot();
        let _: Result<(), std::convert::Infallible> =
            self.measure_observation(FleetObservationStage::Backoff, |platform| {
                thread::sleep(protocol_observation_delay(
                    consecutive_unchanged_observations,
                    platform.initial_observation_delay,
                    platform.maximum_observation_delay,
                ));
                Ok(())
            });
    }

    fn pace_root_owned_observation(
        &mut self,
        _target: &str,
        consecutive_retained_observations: u32,
    ) {
        self.invalidate_observation_snapshot();
        let _: Result<(), std::convert::Infallible> =
            self.measure_observation(FleetObservationStage::Backoff, |platform| {
                thread::sleep(protocol_observation_delay(
                    consecutive_retained_observations,
                    platform.initial_observation_delay,
                    platform.maximum_observation_delay,
                ));
                Ok(())
            });
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
        self.timed_observation(FleetObservationStage::RootManagement, |platform| {
            let binding = intent
                .authorities
                .iter()
                .find(|binding| binding.name == root_name)
                .ok_or_else(|| {
                    IcpEnsurePlatformError::RootManagement("missing reset Root".to_string())
                })?;
            let principal = parse_principal("reset Root", &binding.principal)?;
            let candid = if let Some(source) = intent.source.as_deref()
                && check == super::ReinstallAssetCheck::BeforeReset
            {
                let path = source
                    .reviewed_desired
                    .desired()
                    .protocol
                    .as_ref()
                    .map(|protocol| &protocol.root_candid)
                    .ok_or_else(|| {
                        pool_configuration_error("missing source Root protocol".into())
                    })?;
                let hash = super::artifact_sha256(&platform.root, path)
                    .map_err(|error| pool_configuration_error(error.to_string()))?;
                if source.candid_sha256_by_path.get(path) != Some(&hash) {
                    return Err(pool_configuration_error(
                        "source Root protocol changed".into(),
                    ));
                }
                platform.root.join(path)
            } else {
                platform.root_protocol_candid()?
            };
            let assets = intent
                .assets
                .iter()
                .filter(|asset| asset.root == root_name)
                .collect::<Vec<_>>();
            Self::reinstall_assets_match_bound(&platform.icp, &candid, principal, &assets, check)
        })
    }

    fn reinstall_authorities(
        &mut self,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<BTreeMap<String, RootManagementCanisterObservation>>, Self::Error> {
        self.timed_observation(FleetObservationStage::RootManagement, |platform| {
            platform.require_operator()?;
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
                    load_cached_mainnet_subnet_catalog(&platform.root, now)
                        .map_err(IcpEnsurePlatformError::SubnetCatalog)?,
                )
            } else {
                None
            };
            let mut authorities = BTreeMap::new();
            let root_targets = platform
                .desired
                .canisters
                .iter()
                .filter(|c| c.kind == DesiredCanisterKind::Root)
                .map(|c| c.name.clone())
                .collect();
            let roots = platform
                .observe_root_management(state, &root_targets)?
                .ok_or_else(|| {
                    IcpEnsurePlatformError::RootManagement("missing Root authority".to_string())
                })?;
            let configured = platform
                .desired
                .canisters
                .iter()
                .filter(|c| c.kind != DesiredCanisterKind::Pool)
                .collect::<Vec<_>>();
            for batch in configured.chunks(super::bounded_observations::MAX_IN_FLIGHT) {
                // Root prerequisites finish first. Drain this batch, then consume its
                // outcomes in configured order before issuing any later batch.
                let mut statuses = platform.read_status_batch(
                    batch
                        .iter()
                        .filter(|c| !roots.roots.contains_key(&c.name))
                        .filter_map(|c| platform.current_principal(state, &c.name)),
                );
                for configured in batch {
                    let principal = platform
                        .current_principal(state, &configured.name)
                        .ok_or_else(|| {
                            IcpEnsurePlatformError::UnresolvedCreated(configured.name.clone())
                        })?;
                    // Reuse only this pass's Root observation; a subsequent guard starts fresh.
                    let live = if let Some(root) = roots.roots.get(&configured.name) {
                        platform.complete_install_status(principal, root.live.clone())?
                    } else {
                        let live = platform
                            .status_batch_response(&mut statuses, principal)?
                            .ok_or_else(|| {
                                IcpEnsurePlatformError::RootManagement(
                                    "missing infrastructure".to_string(),
                                )
                            })?;
                        platform.complete_install_status(principal, live)?
                    };
                    let subnet = if let Some(root) = roots.roots.get(&configured.name) {
                        root.subnet.clone()
                    } else if let Some(catalog) = &catalog {
                        catalog
                            .catalog
                            .resolve_canister_route(principal)
                            .map(|route| route.subnet.to_text())
                            .map_err(|error| {
                                IcpEnsurePlatformError::RootManagement(error.to_string())
                            })?
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
            }
            Ok(Some(authorities))
        })
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
                    load_cached_mainnet_subnet_catalog(&platform.root, now)
                        .map_err(IcpEnsurePlatformError::SubnetCatalog)?,
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

    fn observe_operator_funding(
        &mut self,
    ) -> Result<Option<crate::fleet_ensure::view::OperatorFundingObservation>, Self::Error> {
        self.require_operator()?;
        let ledger_fee_cycles = ledger_fee_cycles(self.icp.canister_query_candid(
            &self.desired.cycles_ledger,
            "icrc1_fee",
            &(),
            None,
        )?)?;
        Ok(Some(
            crate::fleet_ensure::view::OperatorFundingObservation {
                cycles_ledger: self.desired.cycles_ledger.clone(),
                ledger_fee_cycles,
                operator_cycles: self.cycles_ledger_balance(&self.desired.operator)?,
            },
        ))
    }

    fn observe_native_funding(
        &mut self,
        root: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<crate::fleet_ensure::model::NativeFundingObservation>, Self::Error> {
        self.require_operator()?;
        let configured = self
            .desired
            .canisters
            .iter()
            .find(|configured| {
                configured.name == root
                    && configured.kind == DesiredCanisterKind::Root
                    && configured.presence == crate::fleet_ensure::model::DesiredPresence::Present
            })
            .ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement(
                    "native funding requires a configured Root".into(),
                )
            })?;
        let principal = self
            .current_principal(state, &configured.name)
            .ok_or_else(|| {
                IcpEnsurePlatformError::RootManagement(
                    "native funding requires an allocated Root".into(),
                )
            })?;
        let live = self.status_optional(principal)?.ok_or_else(|| {
            IcpEnsurePlatformError::RootManagement("native funding Root is unavailable".into())
        })?;
        if let Some(wasm) = &configured.wasm {
            let expected = artifact_hash(&resolve_path(&self.root, wasm))?;
            if live.module_sha256.as_deref() != Some(&expected) {
                return Err(IcpEnsurePlatformError::RootManagement(
                    "native funding Root module differs from the reviewed artifact".into(),
                ));
            }
        }
        let fee = ledger_fee_cycles(self.icp.canister_query_candid(
            &self.desired.cycles_ledger,
            "icrc1_fee",
            &(),
            None,
        )?)?;
        let operator_cycles = self.cycles_ledger_balance(&self.desired.operator)?;
        Ok(Some(crate::fleet_ensure::model::NativeFundingObservation {
            cycles_ledger: self.desired.cycles_ledger.clone(),
            ledger_fee_cycles: fee,
            live,
            operator_cycles,
        }))
    }

    fn observe(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        self.timed_observation(FleetObservationStage::FleetSnapshot, |platform| {
            platform.with_observation_snapshot(|platform| {
                platform.observe_fleet_snapshot(operation_id, state)
            })
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
        self.measure_observation(FleetObservationStage::EffectReconciliation, |platform| {
        if matches!(action, EnsureAction::SealAuthority { .. }) {
            return super::authority_seal::observe(&platform.icp, &platform.root, operation_id, action)
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
                        platform.created_canister_cycles(
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
                    super::maximum_creation_observation_burn(&platform.desired).ok_or(
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
                let live = platform.status_optional(Self::action_principal(state, principal)?)?;
                (live.is_none(), format!("delete:{live:?}"))
            }
            EnsureAction::Fund {
                amount,
                expected_post_cycles,
                funding_deficit_cycles,
                funding_margin_cycles,
                ..
            } => {
                let live_cycles = platform.action_cycles(action, state)?;
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
                let source_cycles = platform
                    .icp
                    .identity_cycles_balance()
                    .map_err(|error| IcpEnsurePlatformError::LedgerTransfer(error.to_string()))?;
                let target = Self::action_principal(state, principal)?;
                let destination_cycles = platform.cycles_ledger_balance(target)?;
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
                    platform.install_status_optional(Self::action_principal(state, principal)?)?;
                post_cycles = platform.direct_reconciliation_cycles(action.name(), live.as_ref());
                let applied = if let Some(witness) = reinstall_witness {
                    let current = platform.reinstall_authorities(state)?;
                    let authority = current
                        .as_ref()
                        .and_then(|a| a.get(&witness.authority.name))
                        .and_then(super::reinstall::authority_binding);
                    let history_authority = authority
                        .as_ref()
                        .and_then(|current| {
                            crate::fleet_ensure::policy::reinstall::history_authority(
                                witness,
                                current,
                                principal,
                                wasm_sha256,
                                &record.state,
                            )
                        })
                        .ok_or_else(|| IcpEnsurePlatformError::ReinstallHistoryConflict {
                            canister: witness.authority.principal.clone(),
                        })?;
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
                        &platform.icp,
                        &platform.root,
                        &history_authority,
                        parse_principal("operator", &platform.desired.operator)?,
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
                let observed = match platform.staging_observations.as_mut() {
                    Some(staging) => {
                        let before = staging.cached_reads();
                        let result = current_protocol::observe_with_staging(
                            &platform.icp, &platform.root, action, staging,
                        );
                        if staging.cached_reads() > before {
                            platform.record_cached_read();
                        }
                        result
                    }
                    None => current_protocol::observe(&platform.icp, &platform.root, action),
                };
                let mut observation = match observed
                {
                    Ok(observation) => observation,
                    Err(error)
                        if matches!(
                            current_action.as_ref(),
                            crate::fleet_ensure::model::CurrentFleetProtocolAction::AdoptStore { .. }
                        ) && recoverable_current_protocol_error(&error) =>
                    {
                        EffectObservation {
                            provisioning_progress: None,
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
                platform.refresh_estate_funding_observation(&mut observation, state)?;
                return Ok(observation);
            }
            EnsureAction::Protocol { .. } => {
                let observation = protocol::observe(
                    &platform.icp,
                    &platform.root,
                    operation_id,
                    &platform.protocol_principals(state),
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
                    platform.resolved_controllers(state, controllers, controller_canisters)?;
                let principal = Self::action_principal(state, principal)?;
                let configured = platform
                    .desired
                    .canisters
                    .iter()
                    .find(|configured| configured.name == *name);
                let mut observed_controllers = if let Some(configured) =
                    configured.filter(|configured| configured.kind == DesiredCanisterKind::Pool)
                {
                    platform.inspect_root_owned_canister(configured, principal, state)?
                        .map(|response| {
                            response
                                .settings
                                .controllers
                                .into_iter()
                                .map(|controller| controller.to_text())
                                .collect::<Vec<_>>()
                        })
                } else {
                    platform.status_optional(principal)?
                        .map(|live| {
                            post_cycles = platform.direct_reconciliation_cycles(name, Some(&live));
                            live.controllers
                        })
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
                let live = platform.status_optional(Self::action_principal(state, principal)?)?;
                post_cycles = platform.direct_reconciliation_cycles(action.name(), live.as_ref());
                (
                    live.as_ref()
                        .is_some_and(|live| live.status == CanisterRuntimeStatus::Running),
                    format!("start:{:?}", live.map(|live| live.status)),
                )
            }
            EnsureAction::Stop { principal, .. } => {
                let live = platform.status_optional(Self::action_principal(state, principal)?)?;
                post_cycles = platform.direct_reconciliation_cycles(action.name(), live.as_ref());
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
                let source = platform
                    .status_optional(Self::action_principal(state, principal)?)?
                    .map(|live| live.cycles)
                    .ok_or_else(|| IcpEnsurePlatformError::MissingTransferBalance {
                        canister: name.clone(),
                        side: "live source",
                    })?;
                let destination = platform
                    .status_optional(platform.current_principal(state, destination).ok_or_else(
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
            provisioning_progress: None,
            provisioning_failure: None,
            applied,
            estate_funding_required: None,
            post_cycles,
            progress_identity,
            retry,
        })
            })
    }

    fn apply_independent_effects(
        &mut self,
        _operation_id: &str,
        uploads: &[super::independent_effects::IndependentEffect<'_>],
        _state: &FleetEnsureStateRecord,
    ) -> Result<Vec<Result<EffectOutcome, Self::Error>>, Self::Error> {
        self.measure_observation(FleetObservationStage::IndependentSubmission, |platform| {
            platform.observation_snapshot.take();
            platform.staging_observations = None;
            platform.require_operator()?;
            Ok(
                super::independent_effects::apply(&platform.icp, &platform.root, uploads)?
                    .into_iter()
                    .map(|result| result.map_err(Into::into))
                    .collect(),
            )
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
        self.measure_observation(FleetObservationStage::EffectSubmission, |platform| {
            platform.observation_snapshot.take();
            platform.staging_observations = None;
            platform.require_operator()?;
            match action {
                EnsureAction::SealAuthority { .. } => super::authority_seal::apply(
                    &platform.icp,
                    &platform.root,
                    operation_id,
                    action,
                )
                .map_err(Into::into),
                EnsureAction::Create {
                    controller_canisters,
                    controllers,
                    created_at_time,
                    ledger,
                    requested_initial_cycles,
                    subnet,
                    ..
                } => platform.apply_create(
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
                    if let Some(live) = platform.status_optional(principal)? {
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
                        platform
                            .icp
                            .delete_canister_without_cycle_recovery(principal)?;
                    }
                    Ok(empty_outcome())
                }
                EnsureAction::Fund {
                    amount,
                    created_at_time,
                    ledger,
                    principal,
                    ..
                } => platform.apply_fund(
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
                } => platform.apply_estate_fund(
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
                } => platform.apply_install(
                    operation_id,
                    &platform.protocol_principals(state),
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
                    current_protocol::apply(&platform.icp, &platform.root, action)
                        .map_err(Into::into)
                }
                EnsureAction::Protocol { .. } => {
                    let action = resolved_protocol_action(action, state)?;
                    protocol::apply(
                        &platform.icp,
                        &platform.root,
                        operation_id,
                        &platform.protocol_principals(state),
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
                } => platform.apply_controllers(
                    Self::action_principal(state, principal)?,
                    &platform.resolved_controllers(state, controllers, controller_canisters)?,
                ),
                EnsureAction::Start { principal, .. } => {
                    platform
                        .icp
                        .start_canister(Self::action_principal(state, principal)?)?;
                    Ok(empty_outcome())
                }
                EnsureAction::Stop { principal, .. } => {
                    platform
                        .icp
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
                } => platform.apply_transfer(
                    *amount,
                    candid,
                    candid_sha256,
                    platform
                        .current_principal(state, destination)
                        .ok_or_else(|| {
                            IcpEnsurePlatformError::UnresolvedCreated(destination.clone())
                        })?,
                    method,
                    operation_id,
                    Self::action_principal(state, principal)?,
                ),
            }
        })
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
    let Some(_) = pre_cycles.checked_add(observation.amount) else {
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
    // A donation cannot stand in for the separate Ledger receipt, but it may
    // increase either native balance observation without invalidating that receipt.
    let observation_is_bounded = live_cycles >= minimum_live_cycles;
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
    recovery_controllers: &[Principal],
) -> Result<u128, IcpEnsurePlatformError> {
    let mut actual = controllers
        .iter()
        .map(Principal::to_text)
        .collect::<Vec<_>>();
    actual.sort();
    let mut ready = recovery_controllers
        .iter()
        .map(Principal::to_text)
        .collect::<Vec<_>>();
    ready.push(root.to_string());
    ready.sort();
    let mut temporary = ready.clone();
    temporary.push(operator.to_string());
    temporary.sort();
    if actual != ready && actual != temporary {
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
    recovery_controllers: &[Principal],
) -> Result<u128, IcpEnsurePlatformError> {
    let mut expected = recovery_controllers.to_vec();
    expected.push(Principal::from_text(root).map_err(|_| {
        IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
            canister: canister.to_string(),
            field: "Root Principal",
        }
    })?);
    expected.sort_unstable();
    let mut actual = controllers.to_vec();
    actual.sort_unstable();
    if actual != expected {
        return Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
            canister: canister.to_string(),
            field: "exact Root controllers",
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
    #[cfg(unix)]
    use std::sync::{Arc, Mutex};

    #[test]
    fn identity_binding_keeps_operator_fence_before_effects() {
        let mut fixture = ProtocolOwnersFixture::new();
        fixture.platform = fixture.platform.with_identity(Some("reviewed"));
        std::fs::write(
            fixture.root.join("icp"),
            crate::test_support::tool_script(
                r#"#!/bin/sh
if [ "$1" = --version ]; then echo 'icp @ICP_VERSION@'; exit 0; fi
while [ "$1" = --project-root-override ] || [ "$1" = --identity-password-file ]; do shift 2; done
if [ "$1 $2" = 'identity default' ]; then touch default-accessed; exit 91; fi
if [ "$1 $2" = 'identity principal' ]; then cat principal; exit 0; fi
echo effect >> effects
"#,
            ),
        )
        .unwrap();
        std::fs::write(fixture.root.join("principal"), "operator").unwrap();
        fixture.platform.require_operator().unwrap();
        assert!(!fixture.root.join("default-accessed").exists());
        std::fs::write(fixture.root.join("principal"), "changed-principal").unwrap();
        let action = EnsureAction::Start {
            name: "root".into(),
            principal: "root".into(),
        };
        let receipt = EffectRecord {
            maintenance_attempts: 0,
            publication_attempts: 0,
            action_sha256: crate::fleet_ensure::ops::action_sha256(&action),
            created_principal: None,
            destination_post_cycles: None,
            destination_pre_cycles: None,
            post_cycles: None,
            pre_cycles: None,
            pre_canister_version: None,
            progress_identity: None,
            receipt: None,
            state: crate::fleet_ensure::model::EffectState::Issued,
        };
        assert!(matches!(
            fixture
                .platform
                .apply("operation", &action, &receipt, &fixture.state),
            Err(IcpEnsurePlatformError::OperatorMismatch { .. })
        ));
        assert!(!fixture.root.join("effects").exists());
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

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

    #[cfg(unix)]
    #[test]
    fn pool_inspection_preflight_stops_before_update_and_requeries_on_retry() {
        use crate::icp::IcpRequestKind;
        use std::sync::{Arc, Mutex};
        let mut fixture = PoolInspectionFixture::new();
        let timings = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&timings);
        fixture.owners.platform.icp = fixture
            .owners
            .platform
            .icp
            .clone()
            .with_timing_handler(move |event| sink.lock().unwrap().push(event));
        let root = fixture.root_id;
        let target = fixture.target;
        let path = fixture.owners.root.clone();
        fixture.inspection_hook(&format!(
            "touch '{}'",
            path.join("inspection-attempted").display()
        ));
        PoolInspectionFixture::reserve_response(&path, root, target, 99);
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                let error = platform
                    .inspect_pool_balance(
                        "pool",
                        &root.to_text(),
                        &target.to_text(),
                        InspectedModule::Any,
                    )
                    .unwrap_err();
                let IcpEnsurePlatformError::CurrentProtocol(
                    current_protocol::CurrentProtocolError::Transport(
                        crate::CanisterProtocolError::InspectionPreflightReserve(evidence),
                    ),
                ) = error
                else {
                    panic!("preflight reserve shortfall");
                };
                assert_eq!(evidence.available_liquid_cycles, 99);
                assert!(!path.join("inspection-attempted").exists());
                PoolInspectionFixture::reserve_response(&path, root, target, 100);
                assert_eq!(
                    platform.inspect_pool_balance(
                        "pool",
                        &root.to_text(),
                        &target.to_text(),
                        InspectedModule::Any,
                    )?,
                    1000
                );
                assert!(path.join("inspection-attempted").exists());
                Ok(())
            })
            .unwrap();
        let timings = timings.lock().unwrap();
        let inspections = timings
            .iter()
            .filter(|event| {
                event.kind == IcpRequestKind::CanisterInspection && event.succeeded.is_some()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            inspections
                .iter()
                .map(|event| event.succeeded)
                .collect::<Vec<_>>(),
            [Some(false), Some(true)]
        );
        for inspection in inspections {
            assert_eq!(inspection.subject, Some(target));
            assert_eq!(inspection.target.as_deref(), Some(root.to_text().as_str()));
            let children = timings
                .iter()
                .filter(|event| event.parent_request_id == Some(inspection.request_id))
                .collect::<Vec<_>>();
            assert!(children.iter().all(|event| event.subject == Some(target)));
            assert!(
                children
                    .iter()
                    .any(|event| event.method.as_deref()
                        == Some(canic_protocol::CANIC_OBSERVABILITY))
            );
            assert_eq!(
                children.iter().any(
                    |event| event.method.as_deref() == Some(canic_protocol::CANIC_ROOT_COMMAND)
                ),
                inspection.succeeded == Some(true)
            );
        }
        drop(timings);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_inspection_retains_protected_reserve_evidence() {
        let mut fixture = PoolInspectionFixture::new();
        let root = fixture.root_id;
        let target = fixture.target;
        let path = fixture.owners.root.clone();
        let evidence = canic_core::dto::canister::CanisterInspectionReserveResponse {
            caller: root,
            canister_id: target,
            native_cycles: 1000,
            available_liquid_cycles: 50,
            required_liquid_cycles: 100,
        };
        let bytes = candid::encode_one(Ok::<_, canic_core::dto::error::Error>(
            RootInspectionResponse::InspectionReserveRequired(evidence.clone()),
        ))
        .unwrap();
        std::fs::write(
            path.join("inspection.json"),
            serde_json::json!({"response_bytes": hex_bytes(bytes)}).to_string(),
        )
        .unwrap();
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                let error = platform
                    .inspect_pool_balance(
                        "pool",
                        &root.to_text(),
                        &target.to_text(),
                        InspectedModule::Any,
                    )
                    .unwrap_err();
                let IcpEnsurePlatformError::CurrentProtocol(
                    current_protocol::CurrentProtocolError::Transport(
                        crate::CanisterProtocolError::InspectionReserve(actual),
                    ),
                ) = error
                else {
                    panic!("typed reserve failure");
                };
                assert_eq!(*actual, evidence);
                PoolInspectionFixture::response(&path, root, 1000);
                assert_eq!(
                    platform.inspect_pool_balance(
                        "pool",
                        &root.to_text(),
                        &target.to_text(),
                        InspectedModule::Any
                    )?,
                    1000
                );
                Ok(())
            })
            .unwrap();
        std::fs::remove_dir_all(path).unwrap();
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
            crate::test_support::tool_script(&format!(
                "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp @ICP_VERSION@'; exit 0; fi\ncat '{}'/balance.json\n",
                fixture.root.display(),
            )),
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
                provisioning_progress: None,
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
                provisioning_progress: None,
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

    #[test]
    fn donation_does_not_substitute_for_a_withdrawal_receipt() {
        let observation = NativeFundingObservation {
            amount: 350,
            expected_post_cycles: 800,
            funding_deficit_cycles: 150,
            funding_margin_cycles: 200,
            live_cycles: Some(10_000),
            pre_cycles: Some(9_000),
        };
        assert!(!native_funding_completion(observation, false).unwrap());
        assert!(native_funding_completion(observation, true).unwrap());
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
            fs::write(&executable, crate::test_support::tool_script(&format!(
                "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp @ICP_VERSION@'; exit 0; fi\nwhile [ \"$#\" -gt 0 ] && [ \"$1\" != status ]; do shift; done\nshift\nprintf '%s\\n' \"$1\" >> '{}'/calls\ncat '{}'/\"$1\".json\n",
                root.display(), root.display(),
            ))).unwrap();
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

        fn synchronized_owners(count: usize) -> Self {
            Self::synchronized_owner_reads(count, false)
        }

        fn synchronized_reinstall_owners(independent: usize) -> Self {
            Self::synchronized_owner_reads(independent + 1, true)
        }

        fn synchronized_owner_reads(count: usize, roots_first: bool) -> Self {
            let mut fixture = Self::new();
            for index in fixture.platform.desired.canisters.len()..count {
                let mut root = fixture.platform.desired.canisters[1].clone();
                root.name = format!("owner-{index:02}");
                root.principal = Some(root.name.clone());
                fixture.status(&root.name, "Running", true);
                fixture.platform.desired.canisters.push(root);
            }
            if roots_first {
                let root = fixture.platform.desired.canisters.remove(1);
                for owner in &mut fixture.platform.desired.canisters {
                    owner.kind = DesiredCanisterKind::Coordinator;
                }
                fixture.platform.desired.canisters.push(root);
                for owner in &fixture.platform.desired.canisters {
                    let file = fixture.root.join(format!("{}.json", owner.name));
                    let mut status: serde_json::Value =
                        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
                    status["version"] = serde_json::json!(7);
                    std::fs::write(file, serde_json::to_vec(&status).unwrap()).unwrap();
                }
            }
            let parallel_count = count - usize::from(roots_first);
            let delay_peer = if roots_first { "store" } else { "root" };
            let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
            std::fs::write(
                fixture.root.join("icp"),
                crate::test_support::tool_script(&format!(
                    r#"#!/bin/sh
if [ "$1" = '--version' ]; then echo 'icp @ICP_VERSION@'; exit 0; fi
while [ "$#" -gt 0 ] && [ "$1" != status ] && [ "$1" != identity ] && [ "$1" != cycles ]; do shift; done
case "$1" in
identity) echo operator; exit;;
cycles) echo '{{"balance":"1000000000000 cycles"}}'; exit;;
status) shift;;
*) exit 1;;
esac
cd '{}'
if [ '{roots_first}' = true ] && [ "$1" = root ]; then
    printf 'root\n' >> prerequisites
    cat root.json
    exit
fi
printf 'start:%s\n' "$1" >> events
started=$(grep -c '^start:' events)
goal=$(( (started + {bound} - 1) / {bound} * {bound} ))
[ "$goal" -le {parallel_count} ] || goal={parallel_count}
attempts=0
while [ "$(grep -c '^start:' events)" -lt "$goal" ]; do
    attempts=$((attempts + 1))
    [ "$attempts" -lt 500 ] || exit 1
    sleep 0.01
done
if [ -e "$1.delay" ]; then
    attempts=0
    while ! grep -q '^finish:{delay_peer}$' events; do
        attempts=$((attempts + 1))
        [ "$attempts" -lt 500 ] || exit 1
        sleep 0.01
    done
fi
printf 'finish:%s\n' "$1" >> events
cat "$1.json"
"#,
                    fixture.root.display()
                )),
            )
            .unwrap();
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
    fn install_preparation_action() -> EnsureAction {
        EnsureAction::Install {
            canic_init: None,
            reinstall_witness: None,
            init_arg: None,
            init_arg_sha256: None,
            init_candid: None,
            init_candid_sha256: None,
            mode: InstallMode::Install,
            name: "root".into(),
            principal: "root".into(),
            wasm: "owner.wasm".into(),
            wasm_sha256: "ff".repeat(32),
        }
    }

    #[cfg(unix)]
    fn preparation_status(fixture: &ProtocolOwnersFixture, version: u64) {
        let path = fixture.root.join("root.json");
        let mut status: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        status["version"] = serde_json::json!(version);
        std::fs::write(path, serde_json::to_vec(&status).unwrap()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reconciliation_reuses_direct_balance_and_refreshes_state_after_failure() {
        let mut install = install_preparation_action();
        let mut fixture = ProtocolOwnersFixture::new();
        if let EnsureAction::Install { wasm_sha256, .. } = &mut install {
            *wasm_sha256 = artifact_hash(&fixture.root.join("owner.wasm")).unwrap();
        }
        let actions = [
            install,
            EnsureAction::Start {
                name: "root".into(),
                principal: "root".into(),
            },
            EnsureAction::Stop {
                name: "root".into(),
                principal: "root".into(),
            },
            EnsureAction::SetControllers {
                name: "root".into(),
                principal: "root".into(),
                controllers: vec![],
                controller_canisters: vec![],
            },
        ];
        for action in actions {
            fixture.status("root", "Running", true);
            preparation_status(&fixture, 7);
            let prepared = super::super::effect_preparation::prepare_effect(
                &mut fixture.platform,
                "op",
                &action,
                &fixture.state,
            )
            .unwrap();
            let before = fixture.platform.icp.remote_call_count();
            let observed = fixture
                .platform
                .observe_effect("op", &action, &prepared.record, &fixture.state)
                .unwrap();
            assert_eq!(observed.post_cycles, Some(1_000_000_000_000));
            assert_eq!(fixture.platform.icp.remote_call_count() - before, 1);
            assert_eq!(
                observed.applied,
                !matches!(action, EnsureAction::Stop { .. })
            );

            let file = fixture.root.join("root.json");
            let mut status: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
            status["cycles"] = serde_json::json!("123");
            status["status"] = serde_json::json!("Stopped");
            status["module_hash"] = serde_json::json!("ab".repeat(32));
            status["settings"]["controllers"] = serde_json::json!(["other"]);
            std::fs::write(&file, serde_json::to_vec(&status).unwrap()).unwrap();
            let changed = fixture
                .platform
                .observe_effect("op", &action, &prepared.record, &fixture.state)
                .unwrap();
            assert_eq!(changed.post_cycles, Some(123));
            assert_eq!(changed.applied, matches!(action, EnsureAction::Stop { .. }));

            status["id"] = serde_json::json!("wrong");
            std::fs::write(&file, serde_json::to_vec(&status).unwrap()).unwrap();
            assert!(matches!(
                fixture
                    .platform
                    .observe_effect("op", &action, &prepared.record, &fixture.state,),
                Err(IcpEnsurePlatformError::StatusIdentityMismatch { .. })
            ));
            status["id"] = serde_json::json!("root");
            status["cycles"] = serde_json::json!("456");
            std::fs::write(&file, serde_json::to_vec(&status).unwrap()).unwrap();
            let retried = fixture
                .platform
                .observe_effect("op", &action, &prepared.record, &fixture.state)
                .unwrap();
            assert_eq!(retried.post_cycles, Some(456));
            assert_eq!(fixture.platform.icp.remote_call_count() - before, 4);
        }
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reconciliation_keeps_protected_pool_balance_authority() {
        let mut fixture = PoolInspectionFixture::fresh(1);
        let principal = fixture.target.to_text();
        std::fs::write(
            fixture.owners.root.join(format!("{principal}.json")),
            serde_json::json!({
                "id": principal, "version": 7, "status": "Running",
                "settings": { "controllers": [fixture.root_id.to_text()] },
                "module_hash": null, "cycles": "9999",
            })
            .to_string(),
        )
        .unwrap();
        let mut install = install_preparation_action();
        if let EnsureAction::Install {
            name,
            principal: target,
            ..
        } = &mut install
        {
            *name = "pool-000".into();
            *target = principal.clone();
        }
        let actions = [
            install,
            EnsureAction::Start {
                name: "pool-000".into(),
                principal: principal.clone(),
            },
            EnsureAction::Stop {
                name: "pool-000".into(),
                principal: principal.clone(),
            },
            EnsureAction::SetControllers {
                name: "pool-000".into(),
                principal,
                controllers: vec![fixture.root_id.to_text()],
                controller_canisters: vec![],
            },
        ];
        for action in actions {
            PoolInspectionFixture::fresh_response(
                &fixture.owners.root,
                vec![fixture.root_id],
                None,
                1000,
            );
            let prepared = super::super::effect_preparation::prepare_effect(
                &mut fixture.owners.platform,
                "op",
                &action,
                &fixture.owners.state,
            )
            .unwrap();
            let observed = fixture
                .owners
                .platform
                .observe_effect("op", &action, &prepared.record, &fixture.owners.state)
                .unwrap();
            assert_eq!(observed.post_cycles, None);
            assert_eq!(
                fixture
                    .owners
                    .platform
                    .action_cycles(&action, &fixture.owners.state)
                    .unwrap(),
                Some(1000)
            );
            PoolInspectionFixture::fresh_response(
                &fixture.owners.root,
                vec![fixture.root_id],
                None,
                900,
            );
            assert_eq!(
                fixture
                    .owners
                    .platform
                    .action_cycles(&action, &fixture.owners.state)
                    .unwrap(),
                Some(900)
            );
            PoolInspectionFixture::fresh_response(
                &fixture.owners.root,
                vec![Principal::anonymous()],
                None,
                900,
            );
            assert!(matches!(
                fixture
                    .owners
                    .platform
                    .action_cycles(&action, &fixture.owners.state),
                Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict { .. })
            ));
        }
        std::fs::remove_dir_all(fixture.owners.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn install_preparation_shares_one_fresh_status_and_expires_on_success_and_failure() {
        use std::sync::{Arc, Mutex};
        let mut fixture = ProtocolOwnersFixture::new();
        let action = install_preparation_action();
        preparation_status(&fixture, 7);
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        fixture.platform = fixture
            .platform
            .with_observation_handler(move |event| sink.lock().unwrap().push(event));
        let prepared = super::super::effect_preparation::prepare_effect(
            &mut fixture.platform,
            "op",
            &action,
            &fixture.state,
        )
        .unwrap();
        assert_eq!(prepared.record.pre_cycles, Some(1_000_000_000_000));
        assert_eq!(prepared.record.pre_canister_version, Some(7));
        assert_eq!(fixture.platform.icp.remote_call_count(), 1);
        assert!(fixture.platform.observation_snapshot.borrow().is_none());
        let measured = events.lock().unwrap();
        let start = measured.first().unwrap();
        let end = measured.last().unwrap();
        assert_eq!(start.succeeded, None);
        assert_eq!(end.succeeded, Some(true));
        assert_eq!(start.span_id, end.span_id);
        assert_eq!(end.remote_call_attempts, 1);
        assert_eq!(end.cached_read_hits, 1);
        drop(measured);
        preparation_status(&fixture, 8);
        let next = super::super::effect_preparation::prepare_effect(
            &mut fixture.platform,
            "op",
            &action,
            &fixture.state,
        )
        .unwrap();
        assert_eq!(next.record.pre_canister_version, Some(8));
        assert_eq!(fixture.platform.icp.remote_call_count(), 2);
        let status = std::fs::read(fixture.root.join("root.json")).unwrap();
        std::fs::write(fixture.root.join("root.json"), b"{}").unwrap();
        assert!(matches!(
            super::super::effect_preparation::prepare_effect(
                &mut fixture.platform,
                "op",
                &action,
                &fixture.state
            ),
            Err(IcpEnsurePlatformError::Icp(_))
        ));
        assert!(fixture.platform.observation_snapshot.borrow().is_none());
        std::fs::write(fixture.root.join("root.json"), status).unwrap();
        assert!(
            super::super::effect_preparation::prepare_effect(
                &mut fixture.platform,
                "op",
                &action,
                &fixture.state
            )
            .is_ok()
        );
        assert_eq!(fixture.platform.icp.remote_call_count(), 4);
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "opt-in matched local IPC measurement; no live IC timing claim"]
    fn install_preparation_matched_measurement() {
        let mut fixture = ProtocolOwnersFixture::new();
        preparation_status(&fixture, 7);
        let action = install_preparation_action();
        // Prime tool compatibility identically; this measures warm IPC, not first CLI launch.
        fixture.platform.status_optional("root").unwrap();
        for pair in 0..8 {
            for shared in if pair % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let before = fixture.platform.icp.remote_call_count();
                let started = std::time::Instant::now();
                let read = |platform: &mut IcpEnsurePlatform| -> Result<_, IcpEnsurePlatformError> {
                    Ok((
                        platform.action_cycles(&action, &fixture.state)?,
                        platform.action_canister_version(&action, &fixture.state)?,
                    ))
                };
                let result = if shared {
                    fixture
                        .platform
                        .with_preparation_observations(&action, read)
                } else {
                    read(&mut fixture.platform)
                }
                .unwrap();
                let elapsed = started.elapsed().as_micros();
                let calls = fixture.platform.icp.remote_call_count() - before;
                assert_eq!(result, (Some(1_000_000_000_000), Some(7)));
                assert_eq!(calls, if shared { 1 } else { 2 });
                eprintln!(
                    "[CANIC-PREPARATION] {}",
                    serde_json::json!({"pair": pair, "shared": shared, "elapsed_micros": elapsed, "calls": calls, "cycles": result.0.map(|v| v.to_string()), "version": result.1})
                );
            }
        }
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    struct PoolInspectionFixture {
        owners: ProtocolOwnersFixture,
        root_id: Principal,
        target: Principal,
    }

    #[cfg(unix)]
    impl PoolInspectionFixture {
        fn new() -> Self {
            let mut owners = ProtocolOwnersFixture::new();
            let root_id = Principal::from_slice(&[2]);
            let operator = Principal::from_slice(&[9]);
            owners.platform.desired.operator = operator.to_text();
            owners.platform.desired.protocol =
                Some(crate::fleet_ensure::model::DesiredFleetProtocol {
                    app_config: "unused.toml".to_string(),
                    component_group_placements: Vec::new(),
                    root_candid: "root.did".to_string(),
                    coordinator_candid: "unused.did".to_string(),
                    store_candid: "unused.did".to_string(),
                });
            std::fs::write(owners.root.join("root.did"),
                "service : { canic_observability : (variant { InspectionReserve : record { canister_id : principal } }) -> () query }"
            ).unwrap();
            std::fs::write(owners.root.join("operator"), operator.to_text()).unwrap();
            for id in [root_id, Principal::from_slice(&[3])] {
                std::fs::write(
                    owners.root.join(format!("{id}.json")),
                    serde_json::json!({
                        "id": id.to_text(), "status": "Running", "settings": {"controllers": []},
                        "module_hash": null, "cycles": "10000",
                    })
                    .to_string(),
                )
                .unwrap();
            }
            std::fs::write(owners.root.join("icp"), crate::test_support::tool_script(&format!(
                "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp @ICP_VERSION@'; exit 0; fi\nwhile [ \"$#\" -gt 0 ] && [ \"$1\" != canister ] && [ \"$1\" != identity ]; do shift; done\ncase \"$1\" in\nidentity) cat '{}'/operator;;\ncanister) shift; case \"$1\" in\nstatus) shift; cat '{}'/\"$1\".json;;\ncall) if [ -e '{}'/fail ]; then exit 1; fi; cat '{}'/inspection.json;;\n*) exit 1;;\nesac;;\n*) exit 1;;\nesac\n",
                owners.root.display(), owners.root.display(), owners.root.display(), owners.root.display(),
            ))).unwrap();
            let script = std::fs::read_to_string(owners.root.join("icp")).unwrap();
            let reserve_route = format!(
                r#"call) if [ "$3" = canic_observability ]; then
for expected in '{}'/reserve-"$2"-*.bin; do
    if cmp -s "$5" "$expected"; then cat "${{expected%.bin}}.json"; exit; fi
done
exit 1
fi
if"#,
                owners.root.display()
            );
            std::fs::write(
                owners.root.join("icp"),
                script.replace("call) if", &reserve_route),
            )
            .unwrap();
            Self::response(&owners.root, root_id, 1_000);
            for (caller, target) in [
                (root_id, Principal::from_slice(&[4])),
                (root_id, Principal::from_slice(&[5])),
                (Principal::from_slice(&[3]), Principal::from_slice(&[4])),
            ] {
                Self::reserve_response(&owners.root, caller, target, 1_000);
            }
            Self {
                owners,
                root_id,
                target: Principal::from_slice(&[4]),
            }
        }

        fn reserve_response(path: &Path, caller: Principal, target: Principal, liquid: u128) {
            use crate::canister_protocol::inspection::{
                InspectionReserveRequest, InspectionReserveResponse,
            };
            let base = path.join(format!("reserve-{caller}-{target}"));
            std::fs::write(
                base.with_extension("bin"),
                candid::encode_one(InspectionReserveRequest::InspectionReserve(
                    CanisterInspectionRequest {
                        canister_id: target,
                    },
                ))
                .unwrap(),
            )
            .unwrap();
            let response = Ok::<_, canic_core::dto::error::Error>(
                InspectionReserveResponse::InspectionReserve(
                    canic_core::dto::canister::CanisterInspectionReserveResponse {
                        caller,
                        canister_id: target,
                        native_cycles: 1000,
                        available_liquid_cycles: liquid,
                        required_liquid_cycles: 100,
                    },
                ),
            );
            std::fs::write(
                base.with_extension("json"),
                serde_json::json!({
                    "response_bytes": hex_bytes(candid::encode_one(response).unwrap()),
                })
                .to_string(),
            )
            .unwrap();
        }

        fn response(path: &Path, controller: Principal, cycles: u128) {
            let response = Ok::<_, canic_core::dto::error::Error>(
                RootInspectionResponse::InspectCanister(RootInspectionStatus {
                    status: canic_core::dto::canister::CanisterStatusType::Running,
                    settings: ManagementCanisterObservationSettings {
                        controllers: vec![controller],
                    },
                    module_hash: Some(vec![1; 32]),
                    cycles: Nat::from(cycles),
                }),
            );
            std::fs::write(
                path.join("inspection.json"),
                serde_json::json!({
                    "response_bytes": hex_bytes(candid::encode_one(response).unwrap()),
                })
                .to_string(),
            )
            .unwrap();
        }
    }

    // Exercise preparation and completion directly when checking scope expiry.
    #[cfg(unix)]
    impl IcpEnsurePlatform {
        fn inspect_pending_pool_balance(
            &self,
            configured: &crate::fleet_ensure::model::DesiredCanister,
            principal: &str,
            state: &FleetEnsureStateRecord,
        ) -> Result<u128, IcpEnsurePlatformError> {
            let inspection = self.prepare_pending_pool_inspection(configured, principal, state)?;
            let response = self.read_pool_inspection(inspection.authority)?;
            self.complete_pool_inspection(inspection, response)
        }
    }

    #[cfg(unix)]
    impl PoolInspectionFixture {
        fn fresh(pool_count: u32) -> Self {
            let mut fixture = Self::new();
            let owners = &mut fixture.owners;
            let operator = Principal::from_text(&owners.platform.desired.operator).unwrap();
            let mut root = owners.platform.desired.canisters[1].clone();
            root.principal = Some(fixture.root_id.to_text());
            root.controllers = vec![operator.to_text()];
            owners.platform.desired.canisters = vec![root.clone()];
            let config = canic_core::bootstrap::parse_config_model(
                "[app]\nname = 'inspection'\n[roles.root]\nkind = 'root'\n",
            )
            .unwrap();
            owners.platform.desired.bootstrap = Some(crate::fleet_ensure::model::DesiredFleetBootstrap {
            admission_identity_origin: None,
                admission: canic_core::shared_support::fleet_admission_policy::compile_fleet_admission_policy_template(vec![operator], Vec::new()).unwrap(),
                app: canic_core::ids::AppId::from("inspection"),
                canonical_network_id: canic_core::ids::CanonicalNetworkId::ic_mainnet(),
                component_deployment_configuration: config.compile_component_deployment_configuration().unwrap(),
                coordinator: "coordinator".into(),
                coordinator_subnet: canic_core::ids::SubnetId::from_principal(fixture.root_id),
                fleet_id: canic_core::ids::FleetId::from_generated_bytes([5; 32]),
                fresh_estate: true,
                release_build_id: canic_core::ids::ReleaseBuildId::from_nonce(canic_core::ids::ReleaseBuildNonce::from_random_bytes([7; 32])),
                root_funding: None,
                roots: Vec::new(),
                            recovery_controllers: Vec::new(),
});
            std::fs::write(
                owners.root.join(format!("{}.json", fixture.root_id)),
                serde_json::json!({
                    "id": fixture.root_id.to_text(), "status": "Running",
                    "settings": {"controllers": [operator.to_text()]},
                    "module_hash": artifact_hash(&owners.root.join("owner.wasm")).unwrap(),
                    "cycles": "10000",
                })
                .to_string(),
            )
            .unwrap();
            let mut entries = Vec::new();
            for index in 0..pool_count {
                let target = Principal::from_slice(&[u8::try_from(index + 10).unwrap()]);
                Self::reserve_response(&owners.root, fixture.root_id, target, 1_000);
                let mut configured = root.clone();
                configured.name = format!("pool-{index:03}");
                configured.kind = DesiredCanisterKind::Pool;
                configured.principal = None;
                configured.parent = Some("root".into());
                owners
                    .state
                    .pending_principals
                    .insert(configured.name.clone(), target.to_text());
                owners.platform.desired.canisters.push(configured);
                entries.push(CanisterPoolAsset {
                    canister_id: target,
                    creation_receipt: None,
                    cycles: Cycles::new(0),
                    origin: CanisterPoolAssetOrigin::Created,
                    status: CanisterPoolAssetStatus::PendingReset,
                    added_at_ns: 1,
                    updated_at_ns: 1,
                });
            }
            fixture.target = entries[0].canister_id;
            Self::fresh_pool_page(&owners.root, entries);
            let script = std::fs::read_to_string(owners.root.join("icp")).unwrap();
            let script = script.replace(
                "call) if",
                &format!(
                    "call) case \"$*\" in *canic_root_status*) cat '{}'/pool.json; exit;; esac; if",
                    owners.root.display()
                ),
            );
            std::fs::write(owners.root.join("icp"), script).unwrap();
            Self::fresh_response(&owners.root, vec![fixture.root_id], None, 1_000);
            fixture
        }

        fn fresh_pool_page(path: &Path, entries: Vec<CanisterPoolAsset>) {
            let pool_count = u32::try_from(entries.len()).unwrap();
            let page = CanisterPoolResponse {
                config: canic_core::ids::FleetSubnetCanisterPoolConfig {
                    minimum_size: 1,
                    maximum_size: 128,
                    canister_cycles: Cycles::new(1_000),
                    creation_execution_margin: Cycles::new(100),
                },
                tracked: pool_count,
                store: 0,
                store_deletion_pending: 0,
                pooled: pool_count,
                workload: 0,
                surplus: 0,
                ready: 0,
                pending_reset: pool_count,
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
            std::fs::write(
                path.join("pool.json"),
                serde_json::json!({
                    "response_bytes": hex_bytes(candid::encode_one(response).unwrap()),
                })
                .to_string(),
            )
            .unwrap();
        }

        fn fresh_response(
            path: &Path,
            controllers: Vec<Principal>,
            module_hash: Option<Vec<u8>>,
            cycles: u128,
        ) {
            let response = Ok::<_, canic_core::dto::error::Error>(
                RootInspectionResponse::InspectCanister(RootInspectionStatus {
                    status: canic_core::dto::canister::CanisterStatusType::Running,
                    settings: ManagementCanisterObservationSettings { controllers },
                    module_hash,
                    cycles: Nat::from(cycles),
                }),
            );
            std::fs::write(
                path.join("inspection.json"),
                serde_json::json!({
                    "response_bytes": hex_bytes(candid::encode_one(response).unwrap()),
                })
                .to_string(),
            )
            .unwrap();
        }
    }

    #[cfg(unix)]
    impl PoolInspectionFixture {
        fn retained_observation(&self) -> FleetObservation {
            let assets = self
                .owners
                .state
                .pending_principals
                .values()
                .map(|principal| EstatePoolAssetObservation {
                    creation_receipt: None,
                    cycles: 500,
                    lifecycle: EstatePoolAssetLifecycle::Workload,
                    origin: EstatePoolAssetOrigin::Created,
                    principal: principal.clone(),
                })
                .collect::<Vec<_>>();
            FleetObservation {
                additional_controlled_cycles: assets
                    .iter()
                    .map(|asset| (asset.principal.clone(), asset.cycles))
                    .collect(),
                canisters: BTreeMap::new(),
                estate_funding_domains: BTreeMap::from([(
                    "root".into(),
                    EstateFundingDomainObservation {
                        balance_cycles: Some(0),
                        cycles_ledger: self.owners.platform.desired.cycles_ledger.clone(),
                        pool: Some(EstatePoolInventoryObservation {
                            assets,
                            maximum_size: 128,
                            minimum_size: 1,
                            pending_creation: None,
                            readiness_floor_cycles: 1_000,
                            creation_execution_margin_cycles: 100,
                        }),
                        root_principal: Some(self.root_id.to_text()),
                    },
                )]),
                ledger_fee_cycles: 100,
                operator_cycles: 1_000,
                protocol_ready: BTreeMap::new(),
            }
        }

        fn inspect_retained(
            &self,
            observation: &mut FleetObservation,
            allow_pending: bool,
        ) -> Result<
            Vec<crate::fleet_ensure::model::FleetReinstallAssetRecord>,
            IcpEnsurePlatformError,
        > {
            let authorities = BTreeMap::from([(
                "root".into(),
                RootManagementCanisterObservation {
                    live: LiveCanister {
                        canister_version: None,
                        controllers: vec![self.owners.platform.desired.operator.clone()],
                        cycles: 10_000,
                        module_sha256: Some("01".repeat(32)),
                        principal: self.root_id.to_text(),
                        reinstall_required: false,
                        root_owned_lifecycle: None,
                        status: CanisterRuntimeStatus::Running,
                    },
                    name: "root".into(),
                    subnet: "subnet".into(),
                },
            )]);
            let candid = BTreeMap::from([(
                "root".into(),
                self.owners.platform.root_protocol_candid().unwrap(),
            )]);
            self.owners.platform.inspect_reinstall_assets(
                &self.owners.state,
                &authorities,
                observation,
                &candid,
                allow_pending,
            )
        }

        fn verify_retained(
            &self,
            assets: &[crate::fleet_ensure::model::FleetReinstallAssetRecord],
            check: super::super::ReinstallAssetCheck,
        ) -> Result<bool, IcpEnsurePlatformError> {
            IcpEnsurePlatform::reinstall_assets_match_bound(
                &self.owners.platform.icp,
                &self.owners.platform.root_protocol_candid().unwrap(),
                self.root_id,
                &assets.iter().collect::<Vec<_>>(),
                check,
            )
        }

        fn inspection_hook(&self, hook: &str) {
            let path = &self.owners.root;
            let script = std::fs::read_to_string(path.join("icp")).unwrap();
            let response = format!("cat '{}'/inspection.json", path.display());
            assert!(script.contains(&response));
            std::fs::write(
                path.join("icp"),
                script.replace(&response, &format!("{hook}\n{response}")),
            )
            .unwrap();
        }

        fn route_inspection_targets(&self) {
            let path = &self.owners.root;
            for configured in self
                .owners
                .platform
                .desired
                .canisters
                .iter()
                .filter(|canister| canister.kind == DesiredCanisterKind::Pool)
            {
                let target = &self.owners.state.pending_principals[&configured.name];
                let request = RootInspectionCommand::InspectCanister(CanisterInspectionRequest {
                    canister_id: Principal::from_text(target).unwrap(),
                });
                std::fs::write(
                    path.join(format!("request-{}.bin", configured.name)),
                    candid::encode_one(request).unwrap(),
                )
                .unwrap();
            }
            self.inspection_hook(&format!(
                r#"
cd '{}'
request_name=
for expected in request-*.bin; do
    if cmp -s "$5" "$expected"; then
        request_name=${{expected#request-}}
        request_name=${{request_name%.bin}}
        break
    fi
done
[ -n "$request_name" ] || exit 1
printf 'start:%s\n' "$request_name" >> events
if [ -f "$request_name.fail" ]; then
    printf 'finish:%s\n' "$request_name" >> events
    exit 1
fi
if [ -f "$request_name.response" ]; then
    attempts=0
    while ! grep -q '^finish:pool-001$' events; do
        attempts=$((attempts + 1))
        [ "$attempts" -lt 500 ] || exit 1
        sleep 0.01
    done
    printf 'finish:%s\n' "$request_name" >> events
    cat "$request_name.response"
    exit
fi
printf 'finish:%s\n' "$request_name" >> events
"#,
                path.display()
            ));
        }
    }

    #[cfg(unix)]
    #[test]
    fn reinstall_inspections_overlap_and_retain_exact_inventory() {
        let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
        let count = bound * 2 + 1;
        let fixture = PoolInspectionFixture::fresh(u32::try_from(count).unwrap());
        let path = &fixture.owners.root;
        fixture.inspection_hook(&format!(
            r#"
cd '{}'
printf 'start\n' >> events
started=$(grep -c '^start$' events)
goal=$(( (started + {bound} - 1) / {bound} * {bound} ))
[ "$goal" -le {count} ] || goal={count}
attempts=0
while [ "$(grep -c '^start$' events)" -lt "$goal" ]; do
    attempts=$((attempts + 1))
    [ "$attempts" -lt 500 ] || exit 1
    sleep 0.01
done
printf 'finish\n' >> events
"#,
            path.display()
        ));
        let mut observation = fixture.retained_observation();
        let assets = fixture.inspect_retained(&mut observation, false).unwrap();
        assert_eq!(assets.len(), count);
        assert!(
            assets
                .windows(2)
                .all(|pair| pair[0].principal < pair[1].principal)
        );
        assert!(
            assets
                .iter()
                .all(|asset| asset.controllers == [fixture.root_id.to_text()]
                    && asset.module_sha256.is_none())
        );
        assert_eq!(observation.canisters.len(), count);
        assert!(observation.additional_controlled_cycles.is_empty());
        assert!(
            observation
                .canisters
                .values()
                .all(|live| live.as_ref().unwrap().cycles == 1_000)
        );
        std::fs::write(path.join("events"), []).unwrap();
        assert!(
            fixture
                .verify_retained(&assets, super::super::ReinstallAssetCheck::BeforeReset)
                .unwrap()
        );
        let (mut active, mut maximum) = (0, 0);
        for event in std::fs::read_to_string(path.join("events"))
            .unwrap()
            .lines()
        {
            match event {
                "start" => {
                    active += 1;
                    maximum = maximum.max(active);
                }
                "finish" => {
                    assert!(active > 0);
                    active -= 1;
                }
                _ => panic!("unknown inspection event"),
            }
            assert!(active <= bound);
        }
        assert_eq!((active, maximum), (0, bound));
        assert_eq!(
            fixture.owners.platform.icp.remote_call_count(),
            4 * u64::try_from(count).unwrap()
        );
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reinstall_verification_drains_mismatch_before_later_error_and_reads_fresh() {
        use super::super::ReinstallAssetCheck::{BeforeReset, Terminal};
        let fixture = PoolInspectionFixture::fresh(5);
        let path = &fixture.owners.root;
        let assets = fixture
            .owners
            .state
            .pending_principals
            .values()
            .map(
                |principal| crate::fleet_ensure::model::FleetReinstallAssetRecord {
                    controllers: vec![fixture.root_id.to_text()],
                    module_sha256: None,
                    principal: principal.clone(),
                    root: "root".into(),
                    subnet: "subnet".into(),
                },
            )
            .collect::<Vec<_>>();
        PoolInspectionFixture::fresh_response(path, vec![Principal::anonymous()], None, 1000);
        std::fs::copy(path.join("inspection.json"), path.join("pool-000.response")).unwrap();
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], None, 1000);
        std::fs::write(path.join("pool-001.fail"), []).unwrap();
        fixture.route_inspection_targets();
        assert!(!fixture.verify_retained(&assets, BeforeReset).unwrap());
        let events = std::fs::read_to_string(path.join("events")).unwrap();
        for index in 0..4 {
            assert!(events.contains(&format!("finish:pool-{index:03}")));
        }
        assert!(!events.contains("pool-004"));
        assert!(events.find("finish:pool-001").unwrap() < events.find("finish:pool-000").unwrap());
        std::fs::remove_file(path.join("pool-000.response")).unwrap();
        assert!(matches!(
            fixture.verify_retained(&assets, BeforeReset),
            Err(IcpEnsurePlatformError::CurrentProtocol(_))
        ));
        std::fs::remove_file(path.join("pool-001.fail")).unwrap();
        assert!(fixture.verify_retained(&assets, BeforeReset).unwrap());
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], Some(vec![1; 32]), 1000);
        assert!(!fixture.verify_retained(&assets, BeforeReset).unwrap());
        assert!(fixture.verify_retained(&assets, Terminal).unwrap());
        PoolInspectionFixture::fresh_response(
            path,
            vec![Principal::anonymous()],
            Some(vec![1; 32]),
            1000,
        );
        assert!(!fixture.verify_retained(&assets, Terminal).unwrap());
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], None, 1000);
        PoolInspectionFixture::reserve_response(path, fixture.root_id, fixture.target, 0);
        assert!(matches!(
            fixture.verify_retained(&assets, BeforeReset),
            Err(IcpEnsurePlatformError::CurrentProtocol(
                current_protocol::CurrentProtocolError::Transport(
                    CanisterProtocolError::InspectionPreflightReserve(_)
                )
            ))
        ));
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reinstall_inspections_drain_failure_in_order_and_retry_fresh() {
        let fixture = PoolInspectionFixture::fresh(5);
        let path = &fixture.owners.root;
        PoolInspectionFixture::fresh_response(
            path,
            vec![fixture.root_id, Principal::anonymous()],
            None,
            1_000,
        );
        std::fs::copy(path.join("inspection.json"), path.join("pool-000.response")).unwrap();
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], None, 2_000);
        std::fs::write(path.join("pool-001.fail"), []).unwrap();
        fixture.route_inspection_targets();
        let mut observation = fixture.retained_observation();
        let before = observation.clone();
        assert!(
            matches!(fixture.inspect_retained(&mut observation, false), Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict { field: "exact Root controllers", canister }) if canister == fixture.target.to_text())
        );
        assert_eq!(observation, before);
        let events = std::fs::read_to_string(path.join("events")).unwrap();
        for index in 0..4 {
            assert!(events.contains(&format!("finish:pool-{index:03}")));
        }
        assert!(!events.contains("pool-004"));
        assert!(events.find("finish:pool-001").unwrap() < events.find("finish:pool-000").unwrap());
        assert_eq!(fixture.owners.platform.icp.remote_call_count(), 8);
        std::fs::remove_file(path.join("pool-000.response")).unwrap();
        std::fs::remove_file(path.join("pool-001.fail")).unwrap();
        for cycles in [2_000, 3_000] {
            PoolInspectionFixture::fresh_response(
                path,
                vec![fixture.root_id],
                Some(vec![1; 32]),
                cycles,
            );
            let assets = fixture.inspect_retained(&mut observation, false).unwrap();
            assert_eq!(assets.len(), 5);
            assert!(
                assets
                    .iter()
                    .all(|asset| asset.module_sha256.as_deref() == Some(&"01".repeat(32)))
            );
            assert!(
                observation
                    .canisters
                    .values()
                    .all(|live| live.as_ref().unwrap().cycles == cycles)
            );
        }
        assert_eq!(fixture.owners.platform.icp.remote_call_count(), 28);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reinstall_inspections_reject_duplicates_and_preserve_lifecycle_admission() {
        let fixture = PoolInspectionFixture::fresh(5);
        let mut observation = fixture.retained_observation();
        let pool = observation
            .estate_funding_domains
            .get_mut("root")
            .unwrap()
            .pool
            .as_mut()
            .unwrap();
        pool.assets[4].principal = pool.assets[0].principal.clone();
        assert!(matches!(
            fixture.inspect_retained(&mut observation, false),
            Err(IcpEnsurePlatformError::RootManagement(_))
        ));
        assert_eq!(fixture.owners.platform.icp.remote_call_count(), 8);
        for lifecycle in [
            EstatePoolAssetLifecycle::PendingReset,
            EstatePoolAssetLifecycle::Failed,
            EstatePoolAssetLifecycle::Claimed,
        ] {
            let mut observation = fixture.retained_observation();
            let pool = observation
                .estate_funding_domains
                .get_mut("root")
                .unwrap()
                .pool
                .as_mut()
                .unwrap();
            pool.assets.truncate(1);
            pool.assets[0].lifecycle = lifecycle;
            let before = fixture.owners.platform.icp.remote_call_count();
            assert!(matches!(
                fixture.inspect_retained(&mut observation, false),
                Err(IcpEnsurePlatformError::RootManagement(_))
            ));
            assert_eq!(fixture.owners.platform.icp.remote_call_count(), before);
            let result = fixture.inspect_retained(&mut observation, true);
            if lifecycle == EstatePoolAssetLifecycle::Claimed {
                assert!(matches!(
                    result,
                    Err(IcpEnsurePlatformError::RootManagement(_))
                ));
                assert_eq!(fixture.owners.platform.icp.remote_call_count(), before);
            } else {
                assert_eq!(result.unwrap().len(), 1);
                assert_eq!(fixture.owners.platform.icp.remote_call_count(), before + 2);
            }
        }
        std::fs::remove_dir_all(&fixture.owners.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reinstall_inspections_require_fresh_reserve_even_with_cached_evidence() {
        let fixture = PoolInspectionFixture::fresh(1);
        let platform = &fixture.owners.platform;
        *platform.observation_snapshot.borrow_mut() = Some(FleetObservationSnapshot::default());
        platform.retain_pool_inspection(
            PoolInspectionAuthority {
                root: fixture.root_id,
                target: fixture.target,
            },
            RootInspectionStatus {
                status: canic_core::dto::canister::CanisterStatusType::Running,
                settings: ManagementCanisterObservationSettings {
                    controllers: vec![fixture.root_id],
                },
                module_hash: None,
                cycles: Nat::from(9_999_u64),
            },
        );
        let mut observation = fixture.retained_observation();
        PoolInspectionFixture::reserve_response(
            &fixture.owners.root,
            fixture.root_id,
            fixture.target,
            0,
        );
        assert!(matches!(fixture.inspect_retained(&mut observation, false),
            Err(IcpEnsurePlatformError::CurrentProtocol(current_protocol::CurrentProtocolError::Transport(
                CanisterProtocolError::InspectionPreflightReserve(evidence)
            ))) if evidence.canister_id == fixture.target && evidence.caller == fixture.root_id
        ));
        assert_eq!(platform.icp.remote_call_count(), 1);
        PoolInspectionFixture::reserve_response(
            &fixture.owners.root,
            fixture.root_id,
            fixture.target,
            1_000,
        );
        fixture.inspect_retained(&mut observation, false).unwrap();
        assert_eq!(
            observation.canisters["pool-000"].as_ref().unwrap().cycles,
            1_000
        );
        assert_eq!(platform.icp.remote_call_count(), 3);
        std::fs::remove_dir_all(&fixture.owners.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn planning_observations_share_root_reads_and_expire_before_retry() {
        use std::sync::{Arc, Mutex};
        let mut fixture = PoolInspectionFixture::fresh(1);
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        fixture.owners.platform = fixture
            .owners
            .platform
            .with_observation_handler(move |event| {
                if event.succeeded.is_some() {
                    sink.lock().unwrap().push(event);
                }
            })
            .with_observation_delay_bounds(Duration::ZERO, Duration::ZERO);
        let platform = &mut fixture.owners.platform;
        let state = &fixture.owners.state;
        let root = fixture.root_id.to_text();
        platform
            .with_planning_observations(|platform| {
                assert!(
                    platform
                        .observe_root_management(state, &BTreeSet::new())?
                        .is_none()
                );
                platform.timed_observation(
                    FleetObservationStage::ConfiguredCanisters,
                    |platform| {
                        platform.with_observation_snapshot(|platform| {
                            platform.observe_configured_canisters(state)
                        })
                    },
                )?;
                assert_eq!(platform.icp.remote_call_count(), 4);
                platform.pace_root_owned_observation("root", 1);
                platform.status_optional(&root)?;
                assert_eq!(platform.icp.remote_call_count(), 5);
                Ok::<_, IcpEnsurePlatformError>(())
            })
            .unwrap();
        assert!(platform.observation_snapshot.borrow().is_none());
        let events = events.lock().unwrap();
        let total = events.last().unwrap();
        assert_eq!(total.stage, FleetObservationStage::Planning);
        assert_eq!(total.parent_stage, None);
        assert_eq!(total.remote_call_attempts, 5);
        assert!(total.cached_read_hits > 0);
        for event in &events[..events.len() - 1] {
            assert_eq!(event.parent_stage, Some(FleetObservationStage::Planning));
        }
        drop(events);
        platform
            .with_planning_observations(|platform| {
                platform.status_optional(&root)?;
                let desired = platform.desired.clone();
                platform.bind_reviewed_desired(&desired)?;
                platform.status_optional(&root)?;
                Err::<(), _>(IcpEnsurePlatformError::Arithmetic("failed planning"))
            })
            .unwrap_err();
        assert!(platform.observation_snapshot.borrow().is_none());
        assert_eq!(platform.icp.remote_call_count(), 7);
        platform.status_optional(&root).unwrap();
        assert_eq!(platform.icp.remote_call_count(), 8);
        std::fs::remove_dir_all(&fixture.owners.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_balance_timings_distinguish_cached_reads_and_bound_identity_lookups() {
        use std::sync::{Arc, Mutex};
        let count = super::super::bounded_observations::MAX_IN_FLIGHT * 2 + 1;
        let mut fixture = PoolInspectionFixture::fresh(u32::try_from(count).unwrap());
        let mut observation = fixture.retained_observation();
        let assets = &mut observation
            .estate_funding_domains
            .get_mut("root")
            .unwrap()
            .pool
            .as_mut()
            .unwrap()
            .assets;
        for asset in assets.iter_mut() {
            asset.lifecycle = EstatePoolAssetLifecycle::Failed;
        }
        let timings = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&timings);
        fixture.owners.platform = fixture
            .owners
            .platform
            .with_observation_handler(move |timing| {
                if timing.succeeded.is_some() {
                    sink.lock().unwrap().push(timing);
                }
            });
        let platform = &mut fixture.owners.platform;
        let root = fixture.root_id.to_text();
        platform
            .with_observation_snapshot(|platform| {
                for _ in 0..2 {
                    platform
                        .timed_observation(FleetObservationStage::PoolBalances, |platform| {
                            platform.refresh_pool_balances(&root, assets)
                        })?;
                }
                Ok(())
            })
            .unwrap();
        let recorded = timings.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        let batches =
            u64::try_from(count.div_ceil(super::super::bounded_observations::MAX_IN_FLIGHT))
                .unwrap();
        assert_eq!(
            recorded[0].remote_call_attempts,
            1 + 2 * u64::try_from(count).unwrap()
        );
        assert_eq!(recorded[0].identity_lookup_attempts, batches);
        assert_eq!(recorded[1].remote_call_attempts, 0);
        assert_eq!(recorded[1].identity_lookup_attempts, batches);
        assert_eq!(
            recorded[1].cached_read_hits,
            u64::try_from(count).unwrap() + batches
        );
        drop(recorded);
        std::fs::write(
            fixture.owners.root.join("operator"),
            Principal::from_slice(&[99]).to_text(),
        )
        .unwrap();
        let failed = platform.with_observation_snapshot(|platform| {
            platform.timed_observation(FleetObservationStage::PoolBalances, |platform| {
                platform.refresh_pool_balances(&root, assets)
            })
        });
        assert!(matches!(
            failed,
            Err(IcpEnsurePlatformError::OperatorMismatch { .. })
        ));
        let recorded = timings.lock().unwrap();
        assert_eq!(recorded[2].succeeded, Some(false));
        assert_eq!(recorded[2].identity_lookup_attempts, 1);
        assert_eq!(recorded[2].remote_call_attempts, 1);
        drop(recorded);
        std::fs::remove_dir_all(&fixture.owners.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_balance_batch_rechecks_operator_before_issuing_later_reads() {
        let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
        let mut fixture = PoolInspectionFixture::fresh(u32::try_from(bound + 1).unwrap());
        fixture.inspection_hook(&format!(
            "printf '%s' '{}' > '{}/operator'",
            Principal::from_slice(&[99]),
            fixture.owners.root.display()
        ));
        let mut observation = fixture.retained_observation();
        let assets = &mut observation
            .estate_funding_domains
            .get_mut("root")
            .unwrap()
            .pool
            .as_mut()
            .unwrap()
            .assets;
        for asset in assets.iter_mut() {
            asset.lifecycle = EstatePoolAssetLifecycle::Failed;
        }
        let result = fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                platform.refresh_pool_balances(&fixture.root_id.to_text(), assets)
            });
        assert!(matches!(
            result,
            Err(IcpEnsurePlatformError::OperatorMismatch { .. })
        ));
        assert!(assets[..bound].iter().all(|asset| asset.cycles == 1_000));
        assert_eq!(assets[bound].cycles, 500);
        assert_eq!(
            fixture.owners.platform.icp.remote_call_count(),
            1 + 2 * u64::try_from(bound).unwrap()
        );
        std::fs::remove_dir_all(&fixture.owners.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_balances_overlap_only_pending_and_failed_assets_within_the_bound() {
        let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
        let count = bound * 2 + 1;
        let mut fixture = PoolInspectionFixture::fresh(u32::try_from(count + 1).unwrap());
        let path = &fixture.owners.root;
        fixture.inspection_hook(&format!(
            r#"
cd '{}'
printf 'start\n' >> events
started=$(grep -c '^start$' events)
goal=$(( (started + {bound} - 1) / {bound} * {bound} ))
[ "$goal" -le {count} ] || goal={count}
attempts=0
while [ "$(grep -c '^start$' events)" -lt "$goal" ]; do
    attempts=$((attempts + 1))
    [ "$attempts" -lt 500 ] || exit 1
    sleep 0.01
done
printf 'finish\n' >> events
"#,
            path.display()
        ));
        let mut observed = fixture.retained_observation();
        let assets = &mut observed
            .estate_funding_domains
            .get_mut("root")
            .unwrap()
            .pool
            .as_mut()
            .unwrap()
            .assets;
        for (index, asset) in assets.iter_mut().take(count).enumerate() {
            asset.lifecycle = if index % 2 == 0 {
                EstatePoolAssetLifecycle::Failed
            } else {
                EstatePoolAssetLifecycle::PendingReset
            };
        }
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                platform.refresh_pool_balances(&fixture.root_id.to_text(), assets)
            })
            .unwrap();
        assert!(assets[..count].iter().all(|asset| asset.cycles == 1_000));
        assert_eq!(assets[count].cycles, 500);
        let mut active = 0;
        let mut maximum = 0;
        for event in std::fs::read_to_string(path.join("events"))
            .unwrap()
            .lines()
        {
            match event {
                "start" => {
                    active += 1;
                    maximum = maximum.max(active);
                }
                "finish" => {
                    assert!(active > 0);
                    active -= 1;
                }
                _ => panic!("unknown inspection event"),
            }
            assert!(active <= bound);
        }
        assert_eq!((active, maximum), (0, bound));
        assert_eq!(
            fixture.owners.platform.icp.remote_call_count(),
            2 * u64::try_from(count).unwrap() + 1
        );
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_balances_drain_failed_batch_preserve_inventory_and_retry_fresh() {
        let mut fixture = PoolInspectionFixture::fresh(5);
        let path = &fixture.owners.root;
        PoolInspectionFixture::fresh_response(path, vec![Principal::from_slice(&[99])], None, 900);
        std::fs::copy(path.join("inspection.json"), path.join("pool-000.response")).unwrap();
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], None, 1_000);
        std::fs::write(path.join("pool-001.fail"), []).unwrap();
        fixture.route_inspection_targets();
        let mut observed = fixture.retained_observation();
        let assets = &mut observed
            .estate_funding_domains
            .get_mut("root")
            .unwrap()
            .pool
            .as_mut()
            .unwrap()
            .assets;
        for asset in assets.iter_mut() {
            asset.lifecycle = EstatePoolAssetLifecycle::Failed;
        }
        let unchanged = assets.clone();
        let failed = fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                platform.refresh_pool_balances(&fixture.root_id.to_text(), assets)
            });
        assert!(
            matches!(failed, Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict { canister, field: "exact Root controllers" }) if canister == unchanged[0].principal)
        );
        assert_eq!(*assets, unchanged);
        let events = std::fs::read_to_string(path.join("events")).unwrap();
        for index in 0..4 {
            assert!(events.contains(&format!("finish:pool-{index:03}")));
        }
        assert!(!events.contains("pool-004"));
        assert!(events.find("finish:pool-001").unwrap() < events.find("finish:pool-000").unwrap());
        assert_eq!(fixture.owners.platform.icp.remote_call_count(), 9);
        std::fs::remove_file(path.join("pool-000.response")).unwrap();
        std::fs::remove_file(path.join("pool-001.fail")).unwrap();
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], None, 2_000);
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                platform.refresh_pool_balances(&fixture.root_id.to_text(), assets)
            })
            .unwrap();
        assert!(assets.iter().all(|asset| asset.cycles == 2_000));
        assert_eq!(fixture.owners.platform.icp.remote_call_count(), 20);
        assert!(
            fixture
                .owners
                .platform
                .observation_snapshot
                .borrow()
                .is_none()
        );
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn configured_pool_inspections_overlap_within_the_bound_and_drain_partial_batches() {
        let bound = crate::fleet_ensure::ops::bounded_observations::MAX_IN_FLIGHT;
        let pool_count = bound * 2 + 1;
        let mut fixture = PoolInspectionFixture::fresh(u32::try_from(pool_count).unwrap());
        let path = &fixture.owners.root;
        fixture.inspection_hook(&format!(
            r#"
cd '{}'
printf 'start\n' >> events
started=$(grep -c '^start$' events)
goal=$(( (started + {bound} - 1) / {bound} * {bound} ))
[ "$goal" -le {pool_count} ] || goal={pool_count}
attempts=0
while [ "$(grep -c '^start$' events)" -lt "$goal" ]; do
    attempts=$((attempts + 1))
    [ "$attempts" -lt 500 ] || exit 1
    sleep 0.01
done
printf 'finish\n' >> events
"#,
            path.display()
        ));
        let observed = fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                platform.observe_configured_canisters(&fixture.owners.state)
            })
            .unwrap();
        assert_eq!(
            observed.len(),
            fixture.owners.platform.desired.canisters.len()
        );
        let events = std::fs::read_to_string(path.join("events")).unwrap();
        let mut active = 0;
        let mut maximum = 0;
        for event in events.lines() {
            match event {
                "start" => {
                    active += 1;
                    maximum = maximum.max(active);
                }
                "finish" => {
                    assert!(active > 0);
                    active -= 1;
                }
                _ => panic!("unknown fixture event"),
            }
            assert!(active <= bound);
        }
        assert_eq!(active, 0);
        assert_eq!(maximum, bound);
        assert_eq!(
            fixture.owners.platform.icp.remote_call_count(),
            2 * u64::try_from(pool_count).unwrap() + 2
        );
        assert!(
            fixture
                .owners
                .platform
                .observation_snapshot
                .borrow()
                .is_none()
        );
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_batch_keeps_consumer_error_before_later_transport_error_and_retries_fresh() {
        let mut fixture = PoolInspectionFixture::fresh(5);
        let path = &fixture.owners.root;
        PoolInspectionFixture::fresh_response(
            path,
            vec![fixture.root_id],
            Some(vec![1; 32]),
            1_000,
        );
        std::fs::copy(path.join("inspection.json"), path.join("pool-000.response")).unwrap();
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], None, 1_000);
        std::fs::write(path.join("pool-001.fail"), []).unwrap();
        fixture.route_inspection_targets();
        let failed = fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                platform.observe_configured_canisters(&fixture.owners.state)
            });
        assert!(
            matches!(failed, Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict { canister, field: "module-free pool asset" }) if canister == "pool-000")
        );
        let events = std::fs::read_to_string(path.join("events")).unwrap();
        for index in 0..4 {
            assert!(events.contains(&format!("finish:pool-{index:03}")));
        }
        assert!(!events.contains("pool-004"));
        assert!(events.find("finish:pool-001").unwrap() < events.find("finish:pool-000").unwrap());
        assert_eq!(fixture.owners.platform.icp.remote_call_count(), 10);
        assert!(
            fixture
                .owners
                .platform
                .observation_snapshot
                .borrow()
                .is_none()
        );
        std::fs::remove_file(path.join("pool-000.response")).unwrap();
        std::fs::remove_file(path.join("pool-001.fail")).unwrap();
        let observed = fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                platform.observe_configured_canisters(&fixture.owners.state)
            })
            .unwrap();
        assert_eq!(observed.len(), 6);
        assert_eq!(fixture.owners.platform.icp.remote_call_count(), 22);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_batch_preserves_preparation_and_transport_error_precedence() {
        for invalid_index in [1, 2] {
            let mut fixture = PoolInspectionFixture::fresh(5);
            let path = &fixture.owners.root;
            // Index zero is the Root; reject authority for exactly one pool request.
            fixture.owners.platform.desired.canisters[invalid_index].parent =
                Some("missing-root".into());
            fixture.route_inspection_targets();
            for index in 0..4 {
                std::fs::write(path.join(format!("pool-{index:03}.fail")), []).unwrap();
            }
            let failed = fixture
                .owners
                .platform
                .with_observation_snapshot(|platform| {
                    platform.observe_configured_canisters(&fixture.owners.state)
                });
            if invalid_index == 1 {
                assert!(matches!(
                    failed,
                    Err(IcpEnsurePlatformError::CurrentProtocol(
                        current_protocol::CurrentProtocolError::Configuration(_)
                    ))
                ));
            } else {
                assert!(matches!(
                    failed,
                    Err(IcpEnsurePlatformError::CurrentProtocol(
                        current_protocol::CurrentProtocolError::Transport(_)
                    ))
                ));
            }
            let events = std::fs::read_to_string(path.join("events")).unwrap();
            assert_eq!(
                events
                    .lines()
                    .filter(|line| line.starts_with("finish:"))
                    .count(),
                3
            );
            assert!(!events.contains(&format!("pool-{:03}", invalid_index - 1)));
            assert!(!events.contains("pool-004"));
            assert_eq!(fixture.owners.platform.icp.remote_call_count(), 8);
            assert!(
                fixture
                    .owners
                    .platform
                    .observation_snapshot
                    .borrow()
                    .is_none()
            );
            std::fs::remove_dir_all(path).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn pool_batch_benchmark_matches_serial_observation_with_bounded_reads() {
        for pool_count in [1, 14, 27] {
            let mut serial_observed = None;
            for concurrent in [false, true] {
                let mut fixture = PoolInspectionFixture::fresh(pool_count);
                fixture.inspection_hook("sleep 0.02");
                let observed = fixture.owners.platform.with_observation_snapshot(|platform| {
                    let started = std::time::Instant::now();
                    let observed = if concurrent {
                        platform.observe_configured_canisters(&fixture.owners.state)?
                    } else {
                        platform.desired.canisters.iter().map(|configured| {
                            let principal = platform.current_principal(&fixture.owners.state, &configured.name).unwrap();
                            platform.observe_configured_canister(configured, principal, &fixture.owners.state).map(|live| (configured.name.clone(), live))
                        }).collect::<Result<BTreeMap<_, _>, _>>()?
                    };
                    let configured_ms = started.elapsed().as_millis();
                    let configured_calls = platform.icp.remote_call_count();
                    for live in observed.values().filter_map(Option::as_ref).filter(|live| live.principal != fixture.root_id.to_text()) {
                        assert_eq!(platform.inspect_pool_balance("pool", &fixture.root_id.to_text(), &live.principal, InspectedModule::Any)?, live.cycles);
                    }
                    assert_eq!(platform.icp.remote_call_count(), configured_calls);
                    assert_eq!(configured_calls, 2 * u64::from(pool_count) + 2);
                    println!("pool_batch_benchmark assets={pool_count} concurrent={concurrent} configured_ms={configured_ms} configured_calls={configured_calls} extra_balance_calls=0 latency_ms=20");
                    Ok(observed)
                }).unwrap();
                if let Some(serial) = &serial_observed {
                    assert_eq!(&observed, serial);
                } else {
                    serial_observed = Some(observed);
                }
                std::fs::remove_dir_all(&fixture.owners.root).unwrap();
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn fresh_pool_observation_shares_inspections_with_balance_review() {
        let mut redundant_calls = 0;
        for pool_count in [1, 14, 27] {
            let mut fixture = PoolInspectionFixture::fresh(pool_count);
            let root = fixture.root_id.to_text();
            let path = &fixture.owners.root;
            fixture.owners.platform.with_observation_snapshot(|platform| {
                let started = std::time::Instant::now();
                let observed = platform.observe_configured_canisters(&fixture.owners.state)?;
                let configured_elapsed = started.elapsed();
                let configured_calls = platform.icp.remote_call_count();
                assert_eq!(observed.len(), usize::try_from(pool_count).unwrap() + 1);
                assert_eq!(configured_calls, 2 * u64::from(pool_count) + 2);
                let started = std::time::Instant::now();
                for live in observed.values().filter_map(Option::as_ref).filter(|live| live.principal != root) {
                    assert_eq!(live.cycles, 1_000);
                    assert_eq!(platform.inspect_pool_balance("pool", &root, &live.principal, InspectedModule::Any)?, 1_000);
                }
                let balance_calls = platform.icp.remote_call_count() - configured_calls;
                println!("fresh_pool_observation assets={pool_count} configured_ms={} configured_calls={configured_calls} balance_ms={} balance_calls={balance_calls}", configured_elapsed.as_millis(), started.elapsed().as_millis());
                redundant_calls += balance_calls;
                Ok(())
            }).unwrap();
            PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], None, 900);
            let configured = &fixture.owners.platform.desired.canisters[1];
            assert_eq!(
                fixture
                    .owners
                    .platform
                    .inspect_pending_pool_balance(
                        configured,
                        &fixture.target.to_text(),
                        &fixture.owners.state
                    )
                    .unwrap(),
                900
            );
            std::fs::remove_dir_all(path).unwrap();
        }
        assert_eq!(redundant_calls, 0);
    }

    #[cfg(unix)]
    #[test]
    fn fresh_pool_inspection_preserves_consumer_authority_and_failed_scope_expiry() {
        let mut fixture = PoolInspectionFixture::fresh(1);
        let root = fixture.root_id.to_text();
        let target = fixture.target.to_text();
        let operator = Principal::from_text(&fixture.owners.platform.desired.operator).unwrap();
        let configured = fixture.owners.platform.desired.canisters[1].clone();
        let path = &fixture.owners.root;
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id, operator], None, 1_000);
        let failed = fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                assert_eq!(
                    platform.inspect_pending_pool_balance(
                        &configured,
                        &target,
                        &fixture.owners.state
                    )?,
                    1_000
                );
                let calls = platform.icp.remote_call_count();
                assert!(matches!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any),
                    Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        field: "exact Root controllers",
                        ..
                    })
                ));
                assert_eq!(platform.icp.remote_call_count(), calls);
                std::fs::write(path.join("operator"), Principal::from_slice(&[8]).to_text())
                    .unwrap();
                assert!(matches!(
                    platform.inspect_pending_pool_balance(
                        &configured,
                        &target,
                        &fixture.owners.state
                    ),
                    Err(IcpEnsurePlatformError::OperatorMismatch { .. })
                ));
                Err::<(), _>(IcpEnsurePlatformError::Arithmetic(
                    "injected observation failure",
                ))
            });
        assert!(matches!(failed, Err(IcpEnsurePlatformError::Arithmetic(_))));
        assert!(
            fixture
                .owners
                .platform
                .observation_snapshot
                .borrow()
                .is_none()
        );
        std::fs::write(path.join("operator"), operator.to_text()).unwrap();
        PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], Some(vec![1; 32]), 900);
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                assert_eq!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any)?,
                    900
                );
                assert!(matches!(
                    platform.inspect_pending_pool_balance(
                        &configured,
                        &target,
                        &fixture.owners.state
                    ),
                    Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        field: "module-free pool asset",
                        ..
                    })
                ));
                Ok(())
            })
            .unwrap();
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn fresh_pool_inspection_retries_failures_and_rechecks_root_authority_on_cache_hits() {
        let mut fixture = PoolInspectionFixture::fresh(1);
        let target = fixture.target.to_text();
        let root = fixture.root_id.to_text();
        let configured = fixture.owners.platform.desired.canisters[1].clone();
        let path = &fixture.owners.root;
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                let inspect = |platform: &IcpEnsurePlatform| {
                    platform.inspect_pending_pool_balance(
                        &configured,
                        &target,
                        &fixture.owners.state,
                    )
                };
                std::fs::write(path.join("fail"), []).unwrap();
                assert!(matches!(
                    inspect(platform),
                    Err(IcpEnsurePlatformError::CurrentProtocol(
                        current_protocol::CurrentProtocolError::Transport(_)
                    ))
                ));
                std::fs::remove_file(path.join("fail")).unwrap();
                PoolInspectionFixture::fresh_response(
                    path,
                    vec![Principal::from_slice(&[8])],
                    None,
                    900,
                );
                assert!(matches!(
                    inspect(platform),
                    Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        field: "reviewed fresh pool controllers",
                        ..
                    })
                ));
                PoolInspectionFixture::fresh_response(path, vec![fixture.root_id], None, 800);
                assert_eq!(inspect(platform)?, 800);
                assert_eq!(platform.icp.remote_call_count(), 7);
                // Funding accepts the same response, but fresh observation must still
                // establish its own exact Root artifact and controller authority.
                assert_eq!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any)?,
                    800
                );
                std::fs::write(path.join("owner.wasm"), b"changed-root-artifact").unwrap();
                assert!(matches!(
                    inspect(platform),
                    Err(
                        IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                            field: "Root module SHA-256",
                            ..
                        }
                    )
                ));
                std::fs::write(path.join("owner.wasm"), b"current-owner-module").unwrap();
                platform.desired.canisters[0].controllers =
                    vec![Principal::from_slice(&[8]).to_text()];
                assert!(matches!(
                    inspect(platform),
                    Err(
                        IcpEnsurePlatformError::RootOwnedObservationAuthorityConflict {
                            field: "Root controllers",
                            ..
                        }
                    )
                ));
                Ok(())
            })
            .unwrap();
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_inspection_honors_bound_source_contract_without_bypassing_query_failures() {
        let fixture = PoolInspectionFixture::new();
        let root = fixture.root_id.to_text();
        let target = fixture.target.to_text();
        let path = &fixture.owners.root;
        let platform = &fixture.owners.platform;
        let candid_path = path.join("root.did");
        let current = std::fs::read_to_string(&candid_path).unwrap();
        std::fs::write(
            &candid_path,
            "service : { canic_observability : (variant { CycleBalance }) -> () query }",
        )
        .unwrap();
        let inspect =
            || platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any);
        assert_eq!(inspect().unwrap(), 1000);
        // Source observation still checks Root authority and executes protected inspection.
        assert_eq!(platform.icp.remote_call_count(), 2);
        std::fs::write(&candid_path, &current).unwrap();
        PoolInspectionFixture::reserve_response(path, fixture.root_id, fixture.target, 99);
        let before = platform.icp.remote_call_count();
        assert!(matches!(
            inspect(),
            Err(IcpEnsurePlatformError::CurrentProtocol(
                current_protocol::CurrentProtocolError::Transport(
                    CanisterProtocolError::InspectionPreflightReserve(_)
                )
            ))
        ));
        assert_eq!(platform.icp.remote_call_count() - before, 2);
        // A declared query that fails cannot fall through to the inspection update.
        std::fs::write(
            path.join(format!(
                "reserve-{}-{}.json",
                fixture.root_id, fixture.target
            )),
            "invalid response",
        )
        .unwrap();
        let before = platform.icp.remote_call_count();
        assert!(matches!(
            inspect(),
            Err(IcpEnsurePlatformError::CurrentProtocol(
                current_protocol::CurrentProtocolError::Transport(
                    CanisterProtocolError::Response { .. }
                )
            ))
        ));
        assert_eq!(platform.icp.remote_call_count() - before, 2);
        std::fs::write(&candid_path, "invalid Candid").unwrap();
        let before = platform.icp.remote_call_count();
        assert!(matches!(
            inspect(),
            Err(IcpEnsurePlatformError::CurrentProtocol(
                current_protocol::CurrentProtocolError::Transport(
                    CanisterProtocolError::InspectionContract { .. }
                )
            ))
        ));
        assert_eq!(platform.icp.remote_call_count() - before, 1);
        std::fs::write(&candid_path, current).unwrap();
        PoolInspectionFixture::reserve_response(path, fixture.root_id, fixture.target, 1000);
        let before = platform.icp.remote_call_count();
        assert_eq!(inspect().unwrap(), 1000);
        assert_eq!(platform.icp.remote_call_count() - before, 3);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_inspection_snapshot_expires_after_success_and_failure() {
        let mut fixture = PoolInspectionFixture::new();
        let root = fixture.root_id.to_text();
        let target = fixture.target.to_text();
        let path = &fixture.owners.root;
        let inspect = |platform: &IcpEnsurePlatform| {
            platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any)
        };
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                assert_eq!(inspect(platform)?, 1_000);
                PoolInspectionFixture::response(path, fixture.root_id, 900);
                assert_eq!(inspect(platform)?, 1_000);
                // Root status, reserve preflight and inspection serve both projections.
                assert_eq!(platform.icp.remote_call_count(), 3);
                Ok(())
            })
            .unwrap();
        assert!(
            fixture
                .owners
                .platform
                .observation_snapshot
                .borrow()
                .is_none()
        );
        assert_eq!(inspect(&fixture.owners.platform).unwrap(), 900);
        let failed: Result<(), IcpEnsurePlatformError> = fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                assert_eq!(inspect(platform)?, 900);
                PoolInspectionFixture::response(path, fixture.root_id, 800);
                Err(IcpEnsurePlatformError::Arithmetic(
                    "injected observation failure",
                ))
            });
        assert!(matches!(failed, Err(IcpEnsurePlatformError::Arithmetic(_))));
        assert!(
            fixture
                .owners
                .platform
                .observation_snapshot
                .borrow()
                .is_none()
        );
        assert_eq!(inspect(&fixture.owners.platform).unwrap(), 800);
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                assert_eq!(inspect(platform)?, 800);
                Ok(())
            })
            .unwrap();
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_inspection_snapshot_binds_root_target_module_and_operator() {
        let mut fixture = PoolInspectionFixture::new();
        let root = fixture.root_id.to_text();
        let target = fixture.target.to_text();
        let other_root = Principal::from_slice(&[3]);
        let other_target = Principal::from_slice(&[5]).to_text();
        let path = &fixture.owners.root;
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                assert_eq!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any)?,
                    1_000
                );
                assert!(matches!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Empty),
                    Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        field: "module-free pool asset",
                        ..
                    })
                ));
                PoolInspectionFixture::response(path, fixture.root_id, 900);
                assert_eq!(
                    platform.inspect_pool_balance(
                        "other asset",
                        &root,
                        &other_target,
                        InspectedModule::Any
                    )?,
                    900
                );
                PoolInspectionFixture::response(path, other_root, 800);
                assert_eq!(
                    platform.inspect_pool_balance(
                        "other Root",
                        &other_root.to_text(),
                        &target,
                        InspectedModule::Any
                    )?,
                    800
                );
                assert_eq!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any)?,
                    1_000
                );
                // Cached data cannot authorize an operator selected after the first inspection.
                std::fs::write(path.join("operator"), other_root.to_text()).unwrap();
                assert!(matches!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any),
                    Err(IcpEnsurePlatformError::OperatorMismatch { .. })
                ));
                Ok(())
            })
            .unwrap();
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pool_inspection_snapshot_does_not_retain_transport_or_authority_failures() {
        let mut fixture = PoolInspectionFixture::new();
        let root = fixture.root_id.to_text();
        let target = fixture.target.to_text();
        let path = &fixture.owners.root;
        fixture
            .owners
            .platform
            .with_observation_snapshot(|platform| {
                std::fs::write(path.join("fail"), []).unwrap();
                assert!(matches!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any),
                    Err(IcpEnsurePlatformError::CurrentProtocol(
                        current_protocol::CurrentProtocolError::Transport(_)
                    ))
                ));
                std::fs::remove_file(path.join("fail")).unwrap();
                PoolInspectionFixture::response(path, Principal::from_slice(&[3]), 900);
                assert!(matches!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any),
                    Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict {
                        field: "exact Root controllers",
                        ..
                    })
                ));
                PoolInspectionFixture::response(path, fixture.root_id, 800);
                assert_eq!(
                    platform.inspect_pool_balance("pool", &root, &target, InspectedModule::Any)?,
                    800
                );
                assert_eq!(platform.icp.remote_call_count(), 7);
                Ok(())
            })
            .unwrap();
        std::fs::remove_dir_all(path).unwrap();
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
            fs::write(&executable, crate::test_support::tool_script(&format!(
                "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp @ICP_VERSION@'; exit 0; fi\nwhile [ \"$#\" -gt 0 ] && [ \"$1\" != canister ]; do shift; done\nshift\nsleep 0.05\ncase \"$1\" in\nstatus) shift; cat '{}'/\"$1\".json;;\ncall) cat '{}'/pool.json;;\n*) exit 1;;\nesac\n",
                fixture.root.display(), fixture.root.display()))).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
            let timings = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&timings);
            fixture.platform = fixture.platform.with_observation_handler(move |timing| {
                if timing.succeeded.is_some() {
                    sink.lock().unwrap().push(timing);
                }
            });
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
            assert_eq!(timing.succeeded, Some(true));
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
        fixture.platform = fixture.platform.with_observation_handler(move |timing| {
            if timing.succeeded.is_some() {
                sink.lock().unwrap().push(timing);
            }
        });
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
        assert_eq!(timing.succeeded, Some(false));
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
    fn reinstall_authorities_overlap_after_root_and_drain_partial_batches() {
        let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
        let independent = bound * 2 + 1;
        let mut fixture = ProtocolOwnersFixture::synchronized_reinstall_owners(independent);
        let timings = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&timings);
        fixture.platform = fixture.platform.with_observation_handler(move |timing| {
            sink.lock().unwrap().push(timing);
        });
        let observed = fixture
            .platform
            .reinstall_authorities(&fixture.state)
            .unwrap()
            .unwrap();
        assert_eq!(observed.len(), independent + 1);
        let recorded = timings.lock().unwrap();
        let started = recorded.first().unwrap();
        let finished = recorded.last().unwrap();
        assert_eq!(started.span_id, finished.span_id);
        assert_eq!(started.succeeded, None);
        assert_eq!(finished.succeeded, Some(true));
        assert_eq!(finished.stage, FleetObservationStage::RootManagement);
        assert_eq!(finished.parent_span_id, None);
        assert_eq!(
            finished.remote_call_attempts,
            fixture.platform.icp.remote_call_count()
        );
        assert!(
            recorded
                .iter()
                .any(|timing| timing.parent_span_id == Some(started.span_id))
        );
        drop(recorded);
        assert!(
            observed
                .values()
                .all(|a| a.live.canister_version == Some(7))
        );
        let (mut active, mut maximum, mut finished) = (0, 0, 0);
        for event in std::fs::read_to_string(fixture.root.join("events"))
            .unwrap()
            .lines()
        {
            let (kind, _) = event.split_once(':').unwrap();
            match kind {
                "start" => {
                    active += 1;
                    maximum = maximum.max(active);
                }
                "finish" => {
                    assert!(active > 0);
                    active -= 1;
                    finished += 1;
                }
                _ => panic!("unknown observation event"),
            }
            assert!(active <= bound);
        }
        assert_eq!((active, maximum, finished), (0, bound, independent));
        assert_eq!(
            fixture.platform.icp.remote_call_count(),
            u64::try_from(independent + 2).unwrap()
        );
        assert_eq!(
            std::fs::read_to_string(fixture.root.join("prerequisites"))
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["root"]
        );
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reinstall_authorities_drain_failed_batch_keep_error_order_and_retry_fresh() {
        let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
        let mut fixture = ProtocolOwnersFixture::synchronized_reinstall_owners(bound + 1);
        let timings = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&timings);
        fixture.platform = fixture.platform.with_observation_handler(move |timing| {
            sink.lock().unwrap().push(timing);
        });
        let original = std::fs::read(fixture.root.join("coordinator.json")).unwrap();
        std::fs::copy(
            fixture.root.join("store.json"),
            fixture.root.join("coordinator.json"),
        )
        .unwrap();
        std::fs::rename(
            fixture.root.join("store.json"),
            fixture.root.join("store.saved"),
        )
        .unwrap();
        std::fs::write(fixture.root.join("coordinator.delay"), []).unwrap();
        assert!(
            matches!(fixture.platform.reinstall_authorities(&fixture.state),
            Err(IcpEnsurePlatformError::StatusIdentityMismatch { expected, .. }) if expected == "coordinator")
        );
        let finished = timings.lock().unwrap().last().unwrap().clone();
        assert_eq!(finished.succeeded, Some(false));
        assert_eq!(finished.stage, FleetObservationStage::RootManagement);
        assert_eq!(finished.parent_span_id, None);
        assert_eq!(
            finished.remote_call_attempts,
            fixture.platform.icp.remote_call_count()
        );
        let events = std::fs::read_to_string(fixture.root.join("events")).unwrap();
        let finished = events
            .lines()
            .filter_map(|line| line.strip_prefix("finish:"))
            .collect::<Vec<_>>();
        assert_eq!(finished.len(), bound);
        assert!(!finished.contains(&"owner-05"));
        assert!(
            finished.iter().position(|name| *name == "store").unwrap()
                < finished
                    .iter()
                    .position(|name| *name == "coordinator")
                    .unwrap()
        );
        assert_eq!(
            fixture.platform.icp.remote_call_count(),
            u64::try_from(bound + 2).unwrap()
        );

        let mut changed: serde_json::Value = serde_json::from_slice(&original).unwrap();
        changed["version"] = serde_json::json!(9);
        std::fs::write(
            fixture.root.join("coordinator.json"),
            serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
        std::fs::rename(
            fixture.root.join("store.saved"),
            fixture.root.join("store.json"),
        )
        .unwrap();
        std::fs::remove_file(fixture.root.join("events")).unwrap();
        preparation_status(&fixture, 8);
        let observed = fixture
            .platform
            .reinstall_authorities(&fixture.state)
            .unwrap()
            .unwrap();
        assert_eq!(observed["root"].live.canister_version, Some(8));
        assert_eq!(observed["coordinator"].live.canister_version, Some(9));
        assert_eq!(observed.len(), bound + 2);
        assert!(fixture.platform.observation_snapshot.borrow().is_none());

        std::fs::remove_file(fixture.root.join("events")).unwrap();
        std::fs::remove_file(fixture.root.join("root.json")).unwrap();
        assert!(matches!(
            fixture.platform.reinstall_authorities(&fixture.state),
            Err(IcpEnsurePlatformError::Icp(_))
        ));
        assert!(!fixture.root.join("events").exists());
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn reinstall_authority_pass_reads_statuses_and_refreshes_after_failure() {
        let mut fixture = ProtocolOwnersFixture::new();
        let path = &fixture.root;
        std::fs::write(path.join("icp"), crate::test_support::tool_script(r#"#!/bin/sh
if [ "$1" = '--version' ]; then echo 'icp @ICP_VERSION@'; exit 0; fi
while [ "$#" -gt 0 ] && [ "$1" != canister ] && [ "$1" != identity ] && [ "$1" != cycles ]; do shift; done
case "$1" in
identity) echo operator;;
cycles) echo '{"balance":"1000000000000 cycles"}';;
canister) shift; [ "$1" = status ] || exit 1; shift; printf '%s\n' "$1" >> calls; cat "$1.json";;
*) exit 1;;
esac
"#)).unwrap();
        for name in ["root", "coordinator", "store"] {
            let file = path.join(format!("{name}.json"));
            let mut status: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
            status["version"] = serde_json::json!(7);
            std::fs::write(file, serde_json::to_vec(&status).unwrap()).unwrap();
        }
        let first = fixture
            .platform
            .reinstall_authorities(&fixture.state)
            .unwrap()
            .unwrap();
        assert_eq!(first.len(), 3);
        assert!(
            first
                .values()
                .all(|value| value.live.canister_version == Some(7))
        );
        let calls = std::fs::read_to_string(path.join("calls")).unwrap();
        eprintln!(
            "reinstall authority status calls: {calls:?}; remote calls: {}",
            fixture.platform.icp.remote_call_count()
        );
        let mut calls = calls.lines().collect::<Vec<_>>();
        assert_eq!(calls.first(), Some(&"root"));
        calls.sort_unstable();
        assert_eq!(calls, ["coordinator", "root", "store"]);
        assert_eq!(fixture.platform.icp.remote_call_count(), 4);
        let file = path.join("root.json");
        let mut status: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        status["version"] = serde_json::json!(8);
        status["status"] = serde_json::json!("Stopped");
        status["module_hash"] = serde_json::json!("ab".repeat(32));
        status["settings"]["controllers"] = serde_json::json!(["changed-controller"]);
        std::fs::write(&file, serde_json::to_vec(&status).unwrap()).unwrap();
        std::fs::rename(path.join("store.json"), path.join("store.saved")).unwrap();
        assert!(
            fixture
                .platform
                .reinstall_authorities(&fixture.state)
                .is_err()
        );
        std::fs::rename(path.join("store.saved"), path.join("store.json")).unwrap();
        let after = fixture
            .platform
            .reinstall_authorities(&fixture.state)
            .unwrap()
            .unwrap();
        let root = &after["root"].live;
        assert_eq!(root.canister_version, Some(8));
        assert_eq!(root.status, CanisterRuntimeStatus::Stopped);
        assert_eq!(
            root.module_sha256.as_deref(),
            Some("ab".repeat(32).as_str())
        );
        assert_eq!(root.controllers, vec!["changed-controller"]);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "manual matched host-readiness latency measurement"]
    fn protocol_owner_readiness_matched_latency_measurement() {
        for count in [3_u64, 9] {
            let mut fixture = ProtocolOwnersFixture::new();
            for index in 3..count {
                let mut owner = fixture.platform.desired.canisters[1].clone();
                owner.name = format!("owner-{index}");
                owner.principal = Some(owner.name.clone());
                fixture.status(&owner.name, "Running", true);
                fixture.platform.desired.canisters.push(owner);
            }
            let executable = fixture.root.join("icp");
            let script = std::fs::read_to_string(&executable)
                .unwrap()
                .replace("\ncat ", "\nsleep 0.02\ncat ");
            std::fs::write(&executable, script).unwrap();
            for round in 0..3 {
                // Alternate order to avoid assigning warm-up only to one candidate.
                for concurrent in if round % 2 == 0 {
                    [false, true]
                } else {
                    [true, false]
                } {
                    let calls = fixture.platform.icp.remote_call_count();
                    let started = std::time::Instant::now();
                    let ready = if concurrent {
                        fixture
                            .platform
                            .current_protocol_owners_are_ready(&fixture.state)
                            .unwrap()
                    } else {
                        let mut ready = true;
                        for configured in &fixture.platform.desired.canisters {
                            let live = fixture
                                .platform
                                .status_optional(configured.principal.as_deref().unwrap())
                                .unwrap()
                                .unwrap();
                            ready &= live.status == CanisterRuntimeStatus::Running
                                && live.module_sha256
                                    == Some(
                                        artifact_hash(&fixture.root.join("owner.wasm")).unwrap(),
                                    );
                        }
                        ready
                    };
                    let elapsed_us = started.elapsed().as_micros();
                    let calls = fixture.platform.icp.remote_call_count() - calls;
                    assert!(ready);
                    assert_eq!(calls, count);
                    println!(
                        "protocol_owner_readiness owners={count} round={round} concurrent={concurrent} elapsed_us={elapsed_us} calls={calls} latency_ms=20"
                    );
                }
            }
            std::fs::remove_dir_all(fixture.root).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn protocol_owner_observations_overlap_within_bound_and_drain_partial_batches() {
        let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
        let count = bound * 2 + 1;
        let fixture = ProtocolOwnersFixture::synchronized_owners(count);
        assert!(
            fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        let events = std::fs::read_to_string(fixture.root.join("events")).unwrap();
        let mut active = 0;
        let mut maximum = 0;
        let mut completed = 0;
        for event in events.lines() {
            if event.starts_with("start:") {
                active += 1;
                maximum = maximum.max(active);
            } else {
                assert!(event.starts_with("finish:"));
                assert!(active > 0);
                active -= 1;
                completed += 1;
            }
            assert!(active <= bound);
        }
        assert_eq!(active, 0);
        assert_eq!(maximum, bound);
        assert_eq!(completed, count);
        assert_eq!(
            fixture.platform.icp.remote_call_count(),
            u64::try_from(count).unwrap()
        );
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn protocol_owner_observations_drain_failure_keep_order_and_retry_fresh() {
        let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
        let count = bound + 1;
        let fixture = ProtocolOwnersFixture::synchronized_owners(count);
        let wrong = std::fs::read(fixture.root.join("store.json")).unwrap();
        std::fs::write(fixture.root.join("coordinator.json"), wrong).unwrap();
        std::fs::write(fixture.root.join("coordinator.delay"), b"").unwrap();
        std::fs::remove_file(fixture.root.join("root.json")).unwrap();
        assert!(
            matches!(fixture.platform.current_protocol_owners_are_ready(&fixture.state),
                Err(IcpEnsurePlatformError::StatusIdentityMismatch { expected, .. }) if expected == "coordinator"
            )
        );
        let events = std::fs::read_to_string(fixture.root.join("events")).unwrap();
        assert_eq!(
            events
                .lines()
                .filter(|event| event.starts_with("finish:"))
                .count(),
            bound
        );
        assert!(!events.contains("owner-04"));
        assert!(events.find("finish:root").unwrap() < events.find("finish:coordinator").unwrap());
        assert_eq!(
            fixture.platform.icp.remote_call_count(),
            u64::try_from(bound).unwrap()
        );
        fixture.status("coordinator", "Running", true);
        fixture.status("root", "Running", true);
        std::fs::remove_file(fixture.root.join("events")).unwrap();
        assert!(
            fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        assert_eq!(
            fixture.platform.icp.remote_call_count(),
            u64::try_from(bound + count).unwrap()
        );
        assert!(fixture.platform.observation_snapshot.borrow().is_none());
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn protocol_owner_observations_stopped_owner_precedes_later_failure() {
        let bound = super::super::bounded_observations::MAX_IN_FLIGHT;
        let fixture = ProtocolOwnersFixture::synchronized_owners(bound + 1);
        fixture.status("coordinator", "Stopped", true);
        std::fs::remove_file(fixture.root.join("root.json")).unwrap();
        assert!(
            !fixture
                .platform
                .current_protocol_owners_are_ready(&fixture.state)
                .unwrap()
        );
        let events = std::fs::read_to_string(fixture.root.join("events")).unwrap();
        assert_eq!(
            events
                .lines()
                .filter(|event| event.starts_with("finish:"))
                .count(),
            bound
        );
        assert!(!events.contains("owner-04"));
        std::fs::remove_dir_all(fixture.root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn replay_planning_shares_configured_owner_reads_and_refreshes_next_decision() {
        let mut fixture = ProtocolOwnersFixture::new();
        let state = &fixture.state;
        let platform = &mut fixture.platform;
        platform
            .with_planning_observations(|platform| {
                platform.with_observation_snapshot(|platform| {
                    platform.observe_configured_canisters(state)
                })?;
                assert_eq!(platform.icp.remote_call_count(), 3);
                assert!(platform.current_protocol_owners_are_ready(state)?);
                assert_eq!(platform.icp.remote_call_count(), 3);
                Ok::<_, IcpEnsurePlatformError>(())
            })
            .unwrap();
        assert!(platform.observation_snapshot.borrow().is_none());
        fixture.status("store", "Stopped", true);
        let platform = &mut fixture.platform;
        let ready = platform
            .with_planning_observations(|platform| {
                platform.with_observation_snapshot(|platform| {
                    platform.observe_configured_canisters(state)
                })?;
                platform.current_protocol_owners_are_ready(state)
            })
            .unwrap();
        assert!(!ready);
        assert_eq!(platform.icp.remote_call_count(), 6);
        assert!(platform.observation_snapshot.borrow().is_none());
        std::fs::remove_dir_all(fixture.root).unwrap();
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
        let calls = std::fs::read_to_string(fixture.root.join("calls")).unwrap();
        let mut owners = calls.lines().collect::<Vec<_>>();
        owners.sort_unstable();
        assert_eq!(owners, ["coordinator", "root", "store"]);
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
        fs::write(&executable, crate::test_support::tool_script(&format!(
            "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo 'icp @ICP_VERSION@'; exit 0; fi\nprintf '%s\\n' \"$*\" >> '{}'\ncat '{}'\n",
            commands.display(), response.display(),
        ))).expect("write observation transport");
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
    fn icp_1_5_public_non_controller_status_is_typed_unavailable_evidence() {
        use std::{fs, os::unix::fs::PermissionsExt};

        let canister = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let root = crate::test_support::temp_dir("canic-public-status-projection");
        fs::create_dir_all(&root).expect("create public status fixture");
        let executable = root.join("icp");
        let commands = root.join("commands.log");
        let script = crate::test_support::tool_script(&format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo 'icp @ICP_VERSION@'; exit 0; fi\n\
             printf '%s\\n' \"$*\" >> '{}'\n\
             printf '%s\\n' '{{\"id\":\"{canister}\",\"controllers\":[\"rdmx6-jaaaa-aaaaa-aaadq-cai\"],\"module_hash\":null}}'\n",
            commands.display(),
        ));
        fs::write(&executable, script).expect("write fake ICP 1.5.0");
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
            r#"{{"id":"{canister}","name":"coordinator","status":"Running","settings":{{"controllers":["rdmx6-jaaaa-aaaaa-aaadq-cai"]}},"module_hash":"0x{}","memory_size":"1","cycles":"1000000000000","query_stats":{{"num_calls_total":"0","num_instructions_total":"0","request_payload_bytes_total":"0","response_payload_bytes_total":"0"}}}}"#,
            "11".repeat(32),
        );
        let response_bytes = candid::encode_one(CanonicalManagementCanisterStatusFixture {
            version: 42,
            module_hash: Some(vec![0x22; 32]),
        })
        .expect("encode independently modelled management status");
        let script = crate::test_support::tool_script(&format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then printf '%s\\n' 'icp @ICP_VERSION@'; exit 0; fi\n\
             printf '%s\\n' \"$*\" >> '{}'\n\
             case \"$*\" in\n\
               *\"canister status {canister}\"*) printf '%s\\n' '{}' ;;\n\
               *) printf '%s\\n' 'unexpected fake ICP command' >&2; exit 23 ;;\n\
             esac\n",
            commands.display(),
            status_json,
        ));
        fs::write(&executable, script).expect("write fake ICP 1.5.0");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("make fake ICP executable");
        let icp = IcpCli::new(executable.to_string_lossy(), Some("ic".to_string()));

        let projected = icp
            .canister_status_report(canister)
            .expect("read ICP 1.5.0 status projection");
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
            crate::test_support::tool_script("#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'icp @ICP_VERSION@'; exit 0; fi\necho '{\"not_response_bytes\":true}'\n"),
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

    #[cfg(unix)]
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one platform-to-policy matrix proves lifecycle projection and exclusive funding authority"
    )]
    fn configured_pool_observation_preserves_one_funding_owner() {
        use crate::fleet_ensure::{model::DesiredFleetArtifacts, policy::compile_plan};

        let mut fixture = PoolInspectionFixture::fresh(1);
        fixture.target = Principal::from_slice(&[10]);
        PoolInspectionFixture::reserve_response(
            &fixture.owners.root,
            fixture.root_id,
            fixture.target,
            1_000,
        );
        let root_id = fixture.root_id.to_text();
        let target = fixture.target.to_text();
        let mut desired = fixture.owners.platform.desired.clone();
        desired.cycles_ledger = Principal::from_slice(&[8]).to_text();
        desired.treasury = "root".into();
        desired.ledger_fee_cycles = "5".into();
        desired.management_creation_fee_cycles = "500".into();
        desired.material_cycle_threshold = "1".into();
        desired.maximum_observation_burn_cycles = "10".into();
        desired.maximum_update_burn_cycles = "20".into();
        let mut root = desired.canisters[0].clone();
        root.principal = Some(root_id.clone());
        root.subnet = root_id.clone();
        root.controllers = vec![desired.operator.clone()];
        root.wasm = None;
        root.initial_cycles = "1000".into();
        root.minimum_cycles = "1000".into();
        let mut configured = root.clone();
        configured.name = "pool".into();
        configured.kind = DesiredCanisterKind::Pool;
        configured.principal = Some(target.clone());
        configured.controllers = vec![root_id.clone()];
        configured.parent = Some(root.name.clone());
        let mut coordinator = root.clone();
        coordinator.name = "coordinator".into();
        coordinator.kind = DesiredCanisterKind::Coordinator;
        coordinator.principal = Some(Principal::from_slice(&[20]).to_text());
        root.parent = Some(coordinator.name.clone());
        desired.canisters = vec![coordinator.clone(), root, configured.clone()];
        let bootstrap = desired.bootstrap.as_mut().unwrap();
        bootstrap.fresh_estate = false;
        bootstrap.roots = vec![crate::fleet_ensure::model::DesiredFleetBootstrapRoot {
            canister_pool_imports: vec![target.clone()],
            component_admissions: Vec::new(),
            component_topology_digest: canic_core::ids::ComponentTopologyDigest::from_bytes(
                [1; 32],
            ),
            funding: crate::test_support::fleet_subnet_root_funding_authority(),
            limits: canic_core::ids::FleetSubnetRootLimits {
                maximum_component_instances: 1,
                maximum_registry_bytes: 1,
                maximum_wasm_store_bytes: 1,
                canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
                    minimum_size: 0,
                    maximum_size: 1,
                    canister_cycles: Cycles::new(1_000),
                    creation_execution_margin: Cycles::new(100),
                },
                cycles_funding: canic_core::ids::CyclesFundingBudget {
                    window_secs: 1,
                    maximum_cycles: Cycles::new(1),
                },
                maximum_group_placements: 1,
            },
            placement_subnet: canic_core::ids::SubnetId::from_principal(fixture.root_id),
            root: "root".into(),
            store: "store".into(),
        }];
        let claim = canic_core::dto::pool::CanisterPoolClaim {
            component: canic_core::ids::ComponentInstanceId::from_generated_bytes([1; 32]),
            operation_id: [2; 32],
        };
        for status in [
            CanisterPoolAssetStatus::PendingReset,
            CanisterPoolAssetStatus::Failed {
                reason: "reset interrupted".into(),
            },
            CanisterPoolAssetStatus::Ready,
            CanisterPoolAssetStatus::Claimed {
                claim: claim.clone(),
            },
            CanisterPoolAssetStatus::Workload { claim },
        ] {
            for cycles in [900, 1_050, 1_100] {
                let lifecycle = estate_pool_lifecycle(&status).unwrap();
                let retained = matches!(
                    lifecycle,
                    EstatePoolAssetLifecycle::PendingReset | EstatePoolAssetLifecycle::Failed
                );
                let live = IcpEnsurePlatform::observed_root_owned_asset(
                    &configured,
                    &target,
                    &root_id,
                    CanisterPoolAsset {
                        canister_id: fixture.target,
                        creation_receipt: None,
                        cycles: Cycles::new(cycles),
                        origin: CanisterPoolAssetOrigin::Imported,
                        status: status.clone(),
                        added_at_ns: 1,
                        updated_at_ns: 1,
                    },
                )
                .unwrap();
                let mut observation = FleetObservation {
                    additional_controlled_cycles: BTreeMap::new(),
                    canisters: BTreeMap::from([
                        ("pool".into(), live),
                        (
                            "root".into(),
                            Some(LiveCanister {
                                canister_version: None,
                                controllers: vec![desired.operator.clone()],
                                cycles: 10_000,
                                module_sha256: None,
                                principal: root_id.clone(),
                                reinstall_required: false,
                                root_owned_lifecycle: None,
                                status: CanisterRuntimeStatus::Running,
                            }),
                        ),
                    ]),
                    estate_funding_domains: BTreeMap::from([(
                        "root".into(),
                        EstateFundingDomainObservation {
                            balance_cycles: Some(0),
                            cycles_ledger: desired.cycles_ledger.clone(),
                            root_principal: Some(root_id.clone()),
                            pool: Some(EstatePoolInventoryObservation {
                                assets: vec![EstatePoolAssetObservation {
                                    creation_receipt: None,
                                    cycles,
                                    lifecycle,
                                    origin: EstatePoolAssetOrigin::Imported,
                                    principal: target.clone(),
                                }],
                                maximum_size: 1,
                                minimum_size: 0,
                                pending_creation: None,
                                readiness_floor_cycles: 1_000,
                                creation_execution_margin_cycles: 100,
                            }),
                        },
                    )]),
                    ledger_fee_cycles: 5,
                    operator_cycles: 10_000,
                    protocol_ready: BTreeMap::new(),
                };
                let mut coordinator_live = observation.canisters["root"].clone().unwrap();
                coordinator_live.principal = coordinator.principal.clone().unwrap();
                observation
                    .canisters
                    .insert(coordinator.name.clone(), Some(coordinator_live));
                let actions = current_protocol::compile_pool_reconciliation(
                    &fixture.owners.root,
                    &desired,
                    &fixture.owners.state,
                    &observation.estate_funding_domains,
                )
                .unwrap();
                assert_eq!(actions.len(), usize::from(retained));
                let plan = compile_plan(
                    &desired,
                    &DesiredFleetArtifacts::default(),
                    &actions,
                    &"11".repeat(32),
                    &desired.fleet,
                    &observation,
                    1,
                    &"22".repeat(32),
                    None,
                )
                .expect("configured pool and reconciliation must share one funding owner");
                let pool_plan = plan
                    .canisters
                    .iter()
                    .find(|entry| entry.name == "pool")
                    .unwrap();
                let funds = pool_plan
                    .actions
                    .iter()
                    .filter(|action| matches!(action, EnsureAction::Fund { .. }))
                    .collect::<Vec<_>>();
                let should_fund = if retained {
                    cycles < 1_100
                } else {
                    cycles < 1_000 && lifecycle == EstatePoolAssetLifecycle::Ready
                };
                assert_eq!(funds.len(), usize::from(should_fund));
                if let Some(EnsureAction::Fund {
                    amount,
                    pool_funding,
                    expected_post_cycles,
                    ..
                }) = funds.first()
                {
                    let authority = pool_funding.as_ref().unwrap();
                    assert_eq!(
                        (&authority.root, authority.lifecycle),
                        (&root_id, lifecycle)
                    );
                    if retained {
                        assert_eq!((*amount, *expected_post_cycles), (1_130 - cycles, 1_130));
                    }
                }
                if retained && cycles == 1_100 {
                    observation
                        .canisters
                        .get_mut(&coordinator.name)
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .status = CanisterRuntimeStatus::Stopped;
                    let prerequisite = compile_plan(
                        &desired,
                        &DesiredFleetArtifacts::default(),
                        &[],
                        &"11".repeat(32),
                        &desired.fleet,
                        &observation,
                        1,
                        &"22".repeat(32),
                        None,
                    )
                    .expect("live reset evidence must not block Coordinator readiness");
                    assert!(prerequisite.canisters.iter().flat_map(|entry| &entry.actions)
                        .any(|action| matches!(action, EnsureAction::Start { name, .. } if name == &coordinator.name)));
                }
            }
        }
        std::fs::remove_dir_all(&fixture.owners.root).unwrap();
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
                &[],
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
                &[],
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
                &[],
            ),
            validate_root_controlled_inspection(
                "pool-0",
                InspectedModule::Empty,
                &root_text,
                &[foreign],
                None,
                &cycles,
                &[],
            ),
            validate_root_controlled_inspection(
                "pool-0",
                InspectedModule::Empty,
                &root_text,
                &[root],
                Some(&[1]),
                &cycles,
                &[],
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
                    &[],
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
                    &[],
                ),
                Err(IcpEnsurePlatformError::FundingInspectionAuthorityConflict { .. })
            ));
        }
    }

    #[test]
    fn recovery_controller_is_required_on_ready_and_pending_fresh_pool_assets() {
        let root = Principal::from_slice(&[7; 29]);
        let operator = Principal::from_slice(&[8; 29]);
        let recovery = Principal::from_slice(&[9; 29]);
        let cycles = Nat::from(17_u8);
        assert!(
            validate_root_controlled_inspection(
                "ready",
                InspectedModule::Empty,
                &root.to_text(),
                &[root, recovery],
                None,
                &cycles,
                &[recovery],
            )
            .is_ok()
        );
        assert!(
            validate_root_controlled_inspection(
                "ready",
                InspectedModule::Empty,
                &root.to_text(),
                &[root],
                None,
                &cycles,
                &[recovery],
            )
            .is_err()
        );
        for controllers in [vec![root, recovery], vec![root, operator, recovery]] {
            assert!(
                validate_pending_fresh_pool_inspection(
                    "pending",
                    &root.to_text(),
                    &operator.to_text(),
                    &controllers,
                    None,
                    &cycles,
                    &[recovery],
                )
                .is_ok()
            );
        }
        assert!(
            validate_pending_fresh_pool_inspection(
                "pending",
                &root.to_text(),
                &operator.to_text(),
                &[root, operator],
                None,
                &cycles,
                &[recovery],
            )
            .is_err()
        );
    }
}
