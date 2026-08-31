//! Module: fleet_ensure::ops::platform
//!
//! Responsibility: mechanically observe and mutate the exact current Fleet through ICP CLI.
//! Does not own: effect ordering, durable intent, retry policy, or plan approval.
//! Boundary: the workflow calls one method only after persisting its exact action identity.

use crate::{
    canister_protocol::{CanisterProtocolError, query_with_candid},
    fleet_ensure::{
        model::{
            CanisterRuntimeStatus, DesiredCanisterKind, DesiredFleet, EffectRecord, EnsureAction,
            FleetEnsureStateRecord, FleetObservation, InstallMode, LiveCanister,
            RetirementTransferBalances, RetirementTransferInvariantError,
            RetirementTransferReconciliation, RootManagementCanisterObservation,
            RootManagementObservation, RootOwnedCanisterLifecycle, reconcile_retirement_transfer,
        },
        ops::{
            EffectObservation, EffectOutcome, EffectRetry, EnsurePaths, EnsurePlatform,
            EnsureStateError, TerminalFleetInventory, canic_init, current_protocol,
            predecessor_root_status, protocol, read_root_start_authority, root_owned_lifecycle,
            verify_root_start_release_authority,
        },
    },
    icp::{
        IcpCandidCallError, IcpCli, IcpCommandError, IcpDiagnostic, IcpManagementCallError,
        run_status,
    },
    icp_config::resolve_icp_build_network_from_root,
    subnet_catalog::load_mainnet_subnet_catalog,
};
use candid::{CandidType, Nat, Principal};
use canic_core::{
    cdk::types::Cycles,
    dto::pool::{CanisterPoolAsset, CanisterPoolResponse, CanisterPoolStatusRequest},
    ids::BuildNetwork,
    protocol as canic_protocol,
};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

#[derive(CandidType)]
struct ManagementCanisterStatusRequest {
    canister_id: Principal,
}

#[derive(CandidType, Deserialize)]
struct ManagementCanisterStatusResponse {
    version: u64,
    module_hash: Option<Vec<u8>>,
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
    recovery_reinstalls: RefCell<BTreeSet<String>>,
    root: PathBuf,
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
            recovery_reinstalls: RefCell::new(BTreeSet::new()),
            root: root.to_path_buf(),
        }
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
        let status = match report.status.to_ascii_lowercase().as_str() {
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
                        canic_protocol::CANIC_STATUS,
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
            .public_cycle_balance(principal)
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

    fn public_cycle_balance(&self, principal: &str) -> Option<u128> {
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
        match response {
            Ok(success) => Ok(created_canister_outcome(
                success.canister_id,
                success.block_id.to_string(),
                authority.requested_initial_cycles,
            )),
            Err(CreateCanisterError::Duplicate {
                duplicate_of,
                canister_id: Some(canister_id),
            }) => Ok(created_canister_outcome(
                canister_id,
                duplicate_of.to_string(),
                authority.requested_initial_cycles,
            )),
            Err(CreateCanisterError::Duplicate {
                canister_id: None, ..
            }) => Err(IcpEnsurePlatformError::LedgerCreatePending),
            Err(error) => Err(IcpEnsurePlatformError::LedgerCreate(render_create_error(
                error,
            ))),
        }
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
        let mut canisters = BTreeMap::new();
        for configured in &self.desired.canisters {
            let observed = self
                .current_principal(state, &configured.name)
                .map(|principal| self.observe_configured_canister(configured, principal, state))
                .transpose()?
                .flatten();
            canisters.insert(configured.name.clone(), observed);
        }
        let mut completed_reinstalls = BTreeSet::new();
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
                completed_reinstalls.insert(name.clone());
            }
        }
        self.recovery_reinstalls
            .borrow_mut()
            .retain(|name| !completed_reinstalls.contains(name));
        for name in self.recovery_reinstalls.borrow().iter() {
            if let Some(live) = canisters.get_mut(name).and_then(Option::as_mut) {
                live.reinstall_required = true;
            }
        }
        let principals = self.protocol_principals(state);
        let protocol_ready = self
            .desired
            .protocol_steps
            .iter()
            .map(|step| {
                let Some(principal) = principals.get(&step.canister) else {
                    return Ok((step.name.clone(), false));
                };
                let live = canisters.get(&step.canister).and_then(Option::as_ref);
                let Some(live) = live else {
                    return Ok((step.name.clone(), false));
                };
                let configured = self
                    .desired
                    .canisters
                    .iter()
                    .find(|configured| configured.name == step.canister)
                    .expect("protocol target was validated against desired canisters");
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
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            canisters,
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
        let coordinator = self
            .desired
            .canisters
            .iter()
            .find(|configured| {
                configured.presence == crate::fleet_ensure::model::DesiredPresence::Present
                    && configured.kind
                        == crate::fleet_ensure::model::DesiredCanisterKind::Coordinator
            })
            .expect("typed topology validation requires one Coordinator");
        let Some(principal) = self.current_principal(state, &coordinator.name) else {
            return Ok(Vec::new());
        };
        let Some(live) = self.status_optional(principal)? else {
            return Ok(Vec::new());
        };
        if let Some(wasm) = &coordinator.wasm {
            let expected = artifact_hash(&resolve_path(&self.root, wasm))?;
            if live.module_sha256.as_deref() != Some(expected.as_str()) {
                return Ok(Vec::new());
            }
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
        let (applied, progress_identity) = match action {
            EnsureAction::Create { .. } => (
                record.created_principal.is_some(),
                format!("created:{:?}", record.created_principal),
            ),
            EnsureAction::Delete { principal, .. } => {
                let live = self.status_optional(Self::action_principal(state, principal)?)?;
                (live.is_none(), format!("delete:{live:?}"))
            }
            EnsureAction::Fund {
                expected_post_cycles,
                ..
            } => {
                let live_cycles = self.action_cycles(action, state)?;
                (
                    native_funding_applied(*expected_post_cycles, live_cycles),
                    format!(
                        "native-topup:ledger-withdraw:{}:actual:{live_cycles:?}:required:{expected_post_cycles}",
                        record.receipt.as_deref().unwrap_or("pending")
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
                principal,
                ..
            } => {
                let expected =
                    self.resolved_controllers(state, controllers, controller_canisters)?;
                let live = self.status_optional(Self::action_principal(state, principal)?)?;
                (
                    live.as_ref()
                        .is_some_and(|live| live.controllers == expected),
                    format!("controllers:{:?}", live.map(|live| live.controllers)),
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
    let child_identity_matches = state
        .principals
        .get(name)
        .is_some_and(|retained| retained == principal);
    let root_identity_matches = state
        .principals
        .get(parent)
        .is_some_and(|retained| retained == root);
    let topology = state
        .topology
        .get(name)
        .filter(|topology| topology.kind == kind && topology.parent.as_deref() == Some(parent));
    (child_identity_matches && root_identity_matches)
        .then_some(topology)
        .flatten()
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

pub fn native_funding_applied(expected_post_cycles: u128, live_cycles: Option<u128>) -> bool {
    expected_post_cycles > 0 && live_cycles.is_some_and(|actual| actual >= expected_post_cycles)
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
            IcpEnsurePlatformError::InstallVersionProofUnavailable { .. }
        ));
        assert!(error.to_string().contains("no install was issued"));
        assert!(error.to_string().contains("resume the same reviewed plan"));

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
    fn toko_fresh_fleet_create_retains_the_exact_requested_balance() {
        let canister = Principal::from_slice(&[9; 29]);
        let canister_text = canister.to_text();
        let outcome = created_canister_outcome(canister, "ledger-block".to_string(), 5_000);

        assert_eq!(
            outcome.created_principal.as_deref(),
            Some(canister_text.as_str())
        );
        assert_eq!(outcome.post_cycles, Some(5_000));
        assert_eq!(outcome.receipt.as_deref(), Some("ledger-block"));
    }
}
