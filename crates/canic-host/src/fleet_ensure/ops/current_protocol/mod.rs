//! Module: fleet_ensure::ops::current_protocol
//!
//! Responsibility: compile and execute current Canic Fleet choreography from typed App topology.
//! Does not own: journal sequencing, generic canister convergence, or historical recovery.
//! Boundary: the reviewed action binds one exact Coordinator, Candid contract, Registry, and plan.

#[cfg(test)]
mod tests;

use super::{
    EffectObservation, EffectOutcome, EffectRetry,
    canic_init::{self, CanicInitError},
};
use crate::{
    canister_protocol::{CanisterProtocolError, call_with_candid, query_with_candid},
    component_topology::{
        RootPoolCapacityError, RootPoolCapacityInput, validate_root_pool_capacity,
    },
    fleet_ensure::model::{
        CurrentFleetProtocolAction, DesiredCanisterKind, DesiredFleet, DesiredFleetProtocol,
        DesiredPresence, EnsureAction, FleetEnsureStateRecord,
    },
    icp::IcpCli,
    release_set::{
        AppConfigSnapshot, ApplicationArtifactEntry, ApplicationArtifactUnion,
        CanicInfrastructureRole, FleetSubnetRootReleaseSetManifest,
        load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
        validate_release_artifact_relative_path,
    },
};
use candid::{CandidType, Principal};
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorCommand, CoordinatorCommandResponse, CoordinatorOperationStatusResponse,
    CoordinatorStatusRequest, CoordinatorStatusResponse,
};
use canic_control_plane::dto::{
    root::RootOperationStatusResponse,
    template::{
        StoreCommand, StoreCommandResponse, StoreStatusRequest, StoreStatusResponse,
        TemplateChunkInput, TemplateChunkSetPrepareInput, TemplateLookupRequest,
        TemplateManifestInput, TemplateManifestResponse, TemplateStagingStatusResponse,
    },
};
use canic_control_plane::ids::{
    TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
};
use canic_core::{
    cdk::types::Cycles,
    control_plane_support::config::{ComponentDeploymentConfiguration, ComponentTopology},
    control_plane_support::ops::{
        component_provisioning_plan::ComponentProvisioningPlanOps, fleet_registry::FleetRegistryOps,
    },
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
            FleetComponentProvisioningPlan, FleetComponentProvisioningPrepareRequest,
            FleetSubnetRootProvisioningBatch,
        },
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        fleet_registry::{
            FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootRegistryMirrorActivationResponse,
            FleetSubnetRootRegistrySyncRequest, FleetSubnetRootRegistrySyncResponse,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootStatus,
        },
        fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetWasmStoreAdoptionRequest},
        pool::{
            CanisterPoolAssetStatus, CanisterPoolStatusRequest, PoolLedgerRecoveryArtifact,
            PoolLedgerRecoveryRequest,
        },
        role::{OperationReceipt, OperationStatusRequest},
        root_store::{
            ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX, ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES,
            ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX, RootStoreBootstrapRequest,
            RootStoreBootstrapResponse, RootStoreCatalogEntry,
        },
    },
    ids::{
        CanisterRole, ComponentGroupDeploymentId, ComponentGroupPlacementId,
        FleetSubnetRootBinding, FleetSubnetWasmStoreAuthority,
    },
    protocol,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const COMPONENT_PROVISIONING_ACTION: &str = "fleet-component-provisioning";
const CURRENT_PROTOCOL_OPERATION_DOMAIN: &[u8] = b"canic.fleet-ensure.current-protocol.v1\0";
const WASM_STORE_PUBLISH_CHUNK: &str = "canic_wasm_store_publish_chunk";

/// One exact initial placement resolved to its protected live Root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentComponentGroupPlacement {
    pub deployment: ComponentGroupDeploymentId,
    pub fleet_subnet_root: Principal,
    pub ordinal: u32,
}

/// One complete typed Coordinator request and its canonical plan identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCurrentComponentProvisioning {
    pub plan_hash: [u8; 32],
    pub request: FleetComponentProvisioningPrepareRequest,
}

/// One exact Coordinator compare-and-commit in the canonical initial Registry chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCurrentRegistryJoin {
    pub request: canic_core::dto::fleet_registry::FleetSubnetRootJoinRequest,
    pub resulting_registry: FleetRegistry,
}

/// Position of the observed Registry in the one deterministic initial chain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurrentRegistryStage {
    Active,
    Genesis,
    Joining(usize),
    Provisioned,
}

/// Complete current-only Registry chain compiled from live immutable Root authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCurrentRegistrySequence {
    pub activation_request: canic_core::dto::fleet_registry::FleetRegistryActivationRequest,
    pub active_registry: FleetRegistry,
    pub current_stage: CurrentRegistryStage,
    pub component_status:
        Option<canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse>,
    pub genesis: FleetRegistry,
    pub joins: Vec<CompiledCurrentRegistryJoin>,
}

/// One Root's exact Store publication, adoption, and bootstrap closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCurrentStoreSequence {
    pub actions: Vec<CurrentFleetProtocolAction>,
    pub bootstrap_request: RootStoreBootstrapRequest,
    pub expected_bootstrap: RootStoreBootstrapResponse,
    pub pool_ledger_recovery_artifact: Option<PoolLedgerRecoveryArtifact>,
}

/// One exact role-owned step in the deterministic current Fleet choreography.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCurrentProtocolStep {
    pub action: CurrentFleetProtocolAction,
    pub name: String,
    pub target: Principal,
}

#[derive(CandidType)]
enum RootCommandFragment {
    AdoptStore(FleetSubnetWasmStoreAdoptionRequest),
    BootstrapStore(RootStoreBootstrapRequest),
    PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
    RecoverPoolLedger(PoolLedgerRecoveryRequest),
    SynchronizeRegistry(FleetSubnetRootRegistrySyncRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponseFragment {
    OperationAccepted(OperationReceipt),
    PrepareComponentRegistry(RootComponentRegistryStatusResponse),
    RecoverPoolLedger(canic_core::dto::pool::PoolLedgerRecoveryReceipt),
}

#[derive(CandidType)]
#[expect(
    clippy::large_enum_variant,
    reason = "the private encoder mirrors the exact Root status request wire"
)]
enum RootStatusRequestFragment {
    ComponentRegistry(RootComponentRegistryPreparationRequest),
    FleetAuthority,
    Operation(OperationStatusRequest),
    Pool(CanisterPoolStatusRequest),
}

#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the private decoder mirrors the exact Root status response wire"
)]
enum RootStatusResponseFragment {
    ComponentRegistry(RootComponentRegistryStatusResponse),
    FleetAuthority(FleetSubnetRootAuthority),
    Operation(RootOperationStatusResponse),
    Pool(canic_core::dto::pool::CanisterPoolResponse),
}

/// Typed current-protocol compilation or transport failure.
#[derive(Debug, ThisError)]
pub enum CurrentProtocolError {
    #[error("current Fleet protocol app config is unavailable: {}", .0.display())]
    AppConfigUnavailable(PathBuf),

    #[error("current Fleet protocol requires one live Coordinator with exact Candid")]
    CoordinatorUnavailable,

    #[error("current Fleet protocol configuration is invalid: {0}")]
    Configuration(String),

    #[error("current Fleet protocol operation identity is not exactly 32 bytes")]
    InvalidOperationIdentity,

    #[error("current Fleet protocol placement is invalid: {0}")]
    InvalidPlacement(String),

    #[error("current Fleet protocol Registry is not an exact all-Active topology")]
    RegistryNotActive,

    #[error("current Fleet protocol Registry is not in its deterministic initial chain: {0}")]
    RegistrySequenceConflict(String),

    #[error("current Fleet protocol response does not match its reviewed action")]
    ResponseMismatch,

    #[error(transparent)]
    ComponentPoolCapacity(#[from] RootPoolCapacityError),

    #[error("failed to read current Fleet protocol Candid {}: {source}", path.display())]
    ReadCandid {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    AppConfig(#[from] crate::release_set::AppConfigError),

    #[error(transparent)]
    Init(#[from] CanicInitError),

    #[error(transparent)]
    Transport(#[from] CanisterProtocolError),
}

fn desired_cycles(field: &str, value: &str) -> Result<u128, CurrentProtocolError> {
    value
        .parse::<Cycles>()
        .map(|cycles| cycles.to_u128())
        .map_err(|_| {
            CurrentProtocolError::Configuration(format!("{field} is not an exact cycle amount"))
        })
}

/// Validate current desired Root pool targets against release-bound App demand.
pub(super) fn validate_component_pool_capacity(
    root: &Path,
    desired: &DesiredFleet,
) -> Result<(), CurrentProtocolError> {
    let Some(protocol) = &desired.protocol else {
        return Ok(());
    };
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(|| {
        CurrentProtocolError::Configuration(
            "typed Fleet protocol is missing generated bootstrap authority".to_string(),
        )
    })?;
    let config_path = resolve_path(root, &protocol.app_config);
    if !config_path.is_file() {
        return Err(CurrentProtocolError::AppConfigUnavailable(config_path));
    }
    let config = AppConfigSnapshot::load(&config_path)?;
    let compiled = config
        .model()
        .compile_component_deployment_configuration()
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    if compiled != bootstrap.component_deployment_configuration {
        return Err(CurrentProtocolError::Configuration(
            "current App config differs from generated bootstrap authority".to_string(),
        ));
    }
    let roots = bootstrap
        .roots
        .iter()
        .map(|entry| RootPoolCapacityInput {
            component_admissions: entry.component_admissions.clone(),
            pool_target_cycles: entry.limits.canister_pool.canister_cycles.to_u128(),
            root: entry.root.clone(),
        })
        .collect::<Vec<_>>();
    validate_root_pool_capacity(config.model(), &roots)?;
    Ok(())
}

/// Validate one retained controller-preparation action from the superseded
/// plan shape without granting it current protocol authority.
///
/// This is deliberately validation-only: the old action may be closed at a
/// typed replan boundary, but it is never compiled into a new plan or replayed
/// against a successor Root.
pub(in crate::fleet_ensure) fn retained_store_control_request_is_exact(
    root: &Path,
    desired: &DesiredFleet,
    operation_id: &str,
    state: &FleetEnsureStateRecord,
    root_name: &str,
    request: &FleetSubnetWasmStoreAdoptionRequest,
) -> Result<bool, CurrentProtocolError> {
    Ok(
        expected_retained_store_control_request(root, desired, operation_id, state, root_name)?
            .as_ref()
            == Some(request),
    )
}

fn expected_retained_store_control_request(
    root: &Path,
    desired: &DesiredFleet,
    operation_id: &str,
    state: &FleetEnsureStateRecord,
    root_name: &str,
) -> Result<Option<FleetSubnetWasmStoreAdoptionRequest>, CurrentProtocolError> {
    let principals = desired
        .canisters
        .iter()
        .filter_map(|canister| {
            retained_principal(desired, state, &canister.name)
                .map(|principal| (canister.name.clone(), principal))
        })
        .collect::<BTreeMap<_, _>>();
    let operation_id = operation_bytes(operation_id)?;
    let Some((_name, authority)) =
        canic_init::compile_root_authorities(root, desired, &principals)?
            .into_iter()
            .find(|(name, _authority)| name == root_name)
    else {
        return Ok(None);
    };
    Ok(Some(FleetSubnetWasmStoreAdoptionRequest {
        operation_id: derived_operation_id(
            operation_id,
            b"store-adoption",
            authority.binding.fleet_subnet_root,
        ),
        authority: authority.wasm_store_authority,
    }))
}

#[cfg(test)]
pub(in crate::fleet_ensure) fn expected_retained_store_control_request_for_test(
    root: &Path,
    desired: &DesiredFleet,
    operation_id: &str,
    state: &FleetEnsureStateRecord,
    root_name: &str,
) -> Result<Option<FleetSubnetWasmStoreAdoptionRequest>, CurrentProtocolError> {
    expected_retained_store_control_request(root, desired, operation_id, state, root_name)
}

/// Compile the complete current Store, Registry, Root-mirror, and Component sequence.
pub(super) fn compile(
    icp: &IcpCli,
    root: &Path,
    desired: &DesiredFleet,
    operation_id: &str,
    state: &FleetEnsureStateRecord,
) -> Result<Vec<EnsureAction>, CurrentProtocolError> {
    let Some(protocol_intent) = &desired.protocol else {
        return Ok(Vec::new());
    };
    let coordinator = desired
        .canisters
        .iter()
        .find(|canister| {
            canister.presence == DesiredPresence::Present
                && canister.kind == crate::fleet_ensure::model::DesiredCanisterKind::Coordinator
        })
        .ok_or(CurrentProtocolError::CoordinatorUnavailable)?;
    let coordinator_text = retained_principal(desired, state, &coordinator.name)
        .ok_or(CurrentProtocolError::CoordinatorUnavailable)?;
    let coordinator_candid_path = resolve_path(root, &protocol_intent.coordinator_candid);
    let coordinator_principal = Principal::from_text(&coordinator_text)
        .map_err(|_| CurrentProtocolError::CoordinatorUnavailable)?;
    let registry = query_registry(icp, &coordinator_candid_path, coordinator_principal)?;
    let operation_id = operation_bytes(operation_id)?;
    let app_config_path = resolve_path(root, &protocol_intent.app_config);
    if !app_config_path.is_file() {
        return Err(CurrentProtocolError::AppConfigUnavailable(app_config_path));
    }
    let config = AppConfigSnapshot::load(&app_config_path)?;
    let configuration = config
        .model()
        .compile_component_deployment_configuration()
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let root_candid_path = resolve_path(root, &protocol_intent.root_candid);
    let store_candid_path = resolve_path(root, &protocol_intent.store_candid);
    let mut root_authorities =
        query_current_root_authorities(icp, desired, state, &root_candid_path, &store_candid_path)?;
    root_authorities.sort_unstable_by_key(|authority| authority.binding.placement_subnet);
    let component_status = query_operation(
        icp,
        &coordinator_candid_path,
        coordinator_principal,
        operation_id,
    )?;
    let registry_sequence = compile_current_registry_sequence_with_status(
        desired,
        state,
        &configuration.component_topology,
        &registry,
        &root_authorities,
        component_status.as_ref(),
    )?;
    let mut stores = BTreeMap::new();
    for authority in &root_authorities {
        let sequence = compile_current_store_sequence(
            root,
            &configuration.component_topology,
            authority,
            operation_id,
        )?;
        stores.insert(authority.binding.fleet_subnet_root, sequence);
    }
    let mut compiled = compile_current_protocol_sequence(
        desired,
        state,
        &configuration,
        &registry_sequence,
        &root_authorities,
        &stores,
        operation_id,
    )?;
    compiled.extend(compile_pool_ledger_recovery_steps(
        icp,
        desired,
        &root_candid_path,
        &root_authorities,
        &stores,
        operation_id,
    )?);
    compiled.sort_by_key(|step| current_protocol_stage(&step.action));
    let per_action_burn_cycles = desired_cycles(
        "maximum_update_burn_cycles",
        &desired.maximum_update_burn_cycles,
    )?;
    bind_unapplied_actions(
        icp,
        root,
        desired,
        state,
        protocol_intent,
        compiled,
        per_action_burn_cycles,
    )
}

fn bind_unapplied_actions(
    icp: &IcpCli,
    root: &Path,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    protocol_intent: &DesiredFleetProtocol,
    compiled: Vec<CompiledCurrentProtocolStep>,
    per_action_burn_cycles: u128,
) -> Result<Vec<EnsureAction>, CurrentProtocolError> {
    compiled
        .into_iter()
        .map(|step| {
            bind_action(
                root,
                desired,
                state,
                protocol_intent,
                step.action,
                step.target,
                step.name,
                per_action_burn_cycles,
            )
        })
        .filter_map(|action| {
            match action
                .and_then(|action| observe(icp, root, &action).map(|observed| (action, observed)))
            {
                Ok((_action, observed)) if observed.applied => None,
                Ok((action, _)) => Some(Ok(action)),
                Err(error) => Some(Err(error)),
            }
        })
        .collect()
}

#[derive(CandidType)]
struct CyclesLedgerAccount {
    owner: Principal,
    subaccount: Option<[u8; 32]>,
}

fn compile_pool_ledger_recovery_steps(
    icp: &IcpCli,
    desired: &DesiredFleet,
    root_candid: &Path,
    authorities: &[FleetSubnetRootAuthority],
    stores: &BTreeMap<Principal, CompiledCurrentStoreSequence>,
    operation_id: [u8; 32],
) -> Result<Vec<CompiledCurrentProtocolStep>, CurrentProtocolError> {
    let fee = query_cycles_ledger_amount(icp, &desired.cycles_ledger, "icrc1_fee", &())?;
    let maximum_execution_burn_cycles = desired_cycles(
        "maximum_update_burn_cycles",
        &desired.maximum_update_burn_cycles,
    )?;
    let mut steps = Vec::new();
    for authority in authorities {
        let root = authority.binding.fleet_subnet_root;
        let helper = stores
            .get(&root)
            .and_then(|store| store.pool_ledger_recovery_artifact.clone())
            .ok_or_else(|| {
                CurrentProtocolError::Configuration(
                    "production Store sequence omits pool Ledger recovery authority".to_string(),
                )
            })?;
        let mut entries = query_pool_entries(icp, root_candid, root)?;
        entries.sort_unstable_by_key(|entry| entry.canister_id);
        for entry in entries {
            let recovery_operation =
                derived_operation_id(operation_id, b"pool-ledger-recovery", entry.canister_id);
            if let Some(RootOperationStatusResponse::RecoverPoolLedger(status)) =
                query_root_operation(icp, root_candid, root, recovery_operation)?
            {
                if status.request.canister_id != entry.canister_id
                    || status.request.cycles_ledger.to_text() != desired.cycles_ledger
                    || status.request.artifact != helper
                {
                    return Err(CurrentProtocolError::ResponseMismatch);
                }
                if status.receipt.is_none() {
                    steps.push(pool_ledger_recovery_step(root, status.request));
                    break;
                }
                continue;
            }
            if !matches!(
                entry.status,
                CanisterPoolAssetStatus::PendingReset
                    | CanisterPoolAssetStatus::Ready
                    | CanisterPoolAssetStatus::Failed { .. }
            ) {
                continue;
            }
            let balance = query_cycles_ledger_amount(
                icp,
                &desired.cycles_ledger,
                "icrc1_balance_of",
                &CyclesLedgerAccount {
                    owner: entry.canister_id,
                    subaccount: None,
                },
            )?;
            if balance == 0 {
                continue;
            }
            let withdrawal_amount = balance.checked_sub(fee).filter(|amount| *amount > 0).ok_or_else(
                || CurrentProtocolError::Configuration(format!(
                    "pool canister {} has {balance} Ledger cycles, which cannot cover exact fee {fee}",
                    entry.canister_id,
                )),
            )?;
            let created_at_time_ns = current_time_ns()?;
            steps.push(pool_ledger_recovery_step(
                root,
                PoolLedgerRecoveryRequest {
                    artifact: helper,
                    canister_id: entry.canister_id,
                    created_at_time_ns,
                    cycles_ledger: Principal::from_text(&desired.cycles_ledger).map_err(|_| {
                        CurrentProtocolError::Configuration(
                            "Cycles Ledger Principal is invalid".to_string(),
                        )
                    })?,
                    ledger_balance: Cycles::new(balance),
                    ledger_fee: Cycles::new(fee),
                    maximum_execution_burn_cycles: Cycles::new(maximum_execution_burn_cycles),
                    operation_id: recovery_operation,
                    withdrawal_amount: Cycles::new(withdrawal_amount),
                },
            ));
            break;
        }
    }
    Ok(steps)
}

fn current_time_ns() -> Result<u64, CurrentProtocolError> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        CurrentProtocolError::Configuration("system clock is before epoch".to_string())
    })?;
    u64::try_from(duration.as_nanos()).map_err(|_| {
        CurrentProtocolError::Configuration("system clock does not fit u64 nanoseconds".to_string())
    })
}

fn pool_ledger_recovery_step(
    root: Principal,
    request: PoolLedgerRecoveryRequest,
) -> CompiledCurrentProtocolStep {
    CompiledCurrentProtocolStep {
        name: format!("pool-ledger-recovery:{}", request.canister_id),
        target: root,
        action: CurrentFleetProtocolAction::RecoverPoolLedger { request },
    }
}

fn query_pool_entries(
    icp: &IcpCli,
    root_candid: &Path,
    root: Principal,
) -> Result<Vec<canic_core::dto::pool::CanisterPoolAsset>, CurrentProtocolError> {
    let mut entries = Vec::new();
    let mut start_after = None;
    loop {
        let response: RootStatusResponseFragment = query_with_candid(
            icp,
            root_candid,
            root,
            protocol::CANIC_STATUS,
            &RootStatusRequestFragment::Pool(CanisterPoolStatusRequest {
                start_after,
                limit: 256,
            }),
        )?;
        let RootStatusResponseFragment::Pool(page) = response else {
            return Err(CurrentProtocolError::ResponseMismatch);
        };
        entries.extend(page.entries);
        let Some(next) = page.next_start_after else {
            break;
        };
        if start_after.is_some_and(|previous| previous >= next) || entries.len() > 4_096 {
            return Err(CurrentProtocolError::ResponseMismatch);
        }
        start_after = Some(next);
    }
    Ok(entries)
}

fn query_cycles_ledger_amount<I: CandidType>(
    icp: &IcpCli,
    ledger: &str,
    method: &str,
    input: &I,
) -> Result<u128, CurrentProtocolError> {
    let amount: candid::Nat = icp
        .canister_query_candid(ledger, method, input, None)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    Cycles::try_from(amount)
        .map(|cycles| cycles.to_u128())
        .map_err(|_| CurrentProtocolError::Configuration(format!("{method} exceeds u128")))
}

/// Compile the exact current Store, Registry, mirror and Component order without transport.
#[expect(
    clippy::too_many_lines,
    reason = "one closed compiler makes the complete Store-to-Component ordering reviewable"
)]
pub fn compile_current_protocol_sequence(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    configuration: &ComponentDeploymentConfiguration,
    registry_sequence: &CompiledCurrentRegistrySequence,
    root_authorities: &[FleetSubnetRootAuthority],
    stores: &BTreeMap<Principal, CompiledCurrentStoreSequence>,
    operation_id: [u8; 32],
) -> Result<Vec<CompiledCurrentProtocolStep>, CurrentProtocolError> {
    let coordinator_principal = registry_sequence
        .active_registry
        .authority
        .binding
        .coordinator;
    let mut actions = Vec::new();
    for authority in root_authorities {
        let root_principal = authority.binding.fleet_subnet_root;
        let store_principal = authority.wasm_store_authority.wasm_store;
        let sequence = stores.get(&root_principal).ok_or_else(|| {
            CurrentProtocolError::Configuration("missing compiled Store sequence".to_string())
        })?;
        for (index, action) in sequence.actions.iter().enumerate() {
            let target = match action.target_kind() {
                DesiredCanisterKind::Root => root_principal,
                DesiredCanisterKind::Store => store_principal,
                _ => {
                    return Err(CurrentProtocolError::Configuration(
                        "Store sequence selected an unrelated role".to_string(),
                    ));
                }
            };
            actions.push(CompiledCurrentProtocolStep {
                action: action.clone(),
                name: format!("root-store:{}:{index}", root_principal.to_text()),
                target,
            });
        }
    }

    let joined_count = match registry_sequence.current_stage {
        CurrentRegistryStage::Genesis => 0,
        CurrentRegistryStage::Joining(count) => count,
        CurrentRegistryStage::Active | CurrentRegistryStage::Provisioned => {
            registry_sequence.joins.len()
        }
    };
    for (index, join) in registry_sequence
        .joins
        .iter()
        .enumerate()
        .skip(joined_count)
    {
        actions.push(CompiledCurrentProtocolStep {
            action: CurrentFleetProtocolAction::JoinRoot {
                expected_registry: join.resulting_registry.clone(),
                expected_version: registry_version(
                    &configuration.component_topology,
                    &join.resulting_registry,
                )?,
                request: join.request.clone(),
            },
            name: format!("registry-join:{index}"),
            target: coordinator_principal,
        });
    }
    let joining_registry = registry_sequence
        .joins
        .last()
        .ok_or(CurrentProtocolError::RegistryNotActive)?
        .resulting_registry
        .clone();
    let joining_version = registry_version(&configuration.component_topology, &joining_registry)?;
    let active_version = registry_version(
        &configuration.component_topology,
        &registry_sequence.active_registry,
    )?;
    for authority in root_authorities {
        let root_principal = authority.binding.fleet_subnet_root;
        let bootstrap = &stores
            .get(&root_principal)
            .ok_or_else(|| {
                CurrentProtocolError::Configuration("missing compiled Store sequence".to_string())
            })?
            .bootstrap_request;
        let sync_request = FleetSubnetRootRegistrySyncRequest {
            operation_id: derived_operation_id(
                operation_id,
                b"registry-synchronization",
                root_principal,
            ),
            expected_registry: joining_version.clone(),
            store_bootstrap: bootstrap.clone(),
        };
        if registry_sequence.current_stage != CurrentRegistryStage::Provisioned {
            let expected_sync = FleetSubnetRootRegistrySyncResponse {
                fleet_subnet_root: root_principal,
                version: joining_version.clone(),
                acknowledgement: FleetSubnetRootSnapshotAcknowledgement {
                    fleet_subnet_root: root_principal,
                    version: joining_version.clone(),
                },
            };
            actions.push(CompiledCurrentProtocolStep {
                action: CurrentFleetProtocolAction::SynchronizeRegistry {
                    expected: expected_sync,
                    request: sync_request.clone(),
                },
                name: format!("registry-sync:{}", root_principal.to_text()),
                target: root_principal,
            });
            let directory = FleetRegistryOps::directory_for_root(
                &registry_sequence.active_registry.authority,
                &configuration.component_topology,
                &registry_sequence.active_registry,
                root_principal,
            )
            .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
            let expected_activation = FleetSubnetRootRegistryMirrorActivationResponse {
                fleet_subnet_root: root_principal,
                previous_registry: joining_version.clone(),
                version: active_version.clone(),
                directory,
            };
            actions.push(CompiledCurrentProtocolStep {
                action: CurrentFleetProtocolAction::ActivateRegistryMirror {
                    expected: expected_activation,
                    request: sync_request,
                },
                name: format!("registry-mirror:{}", root_principal.to_text()),
                target: root_principal,
            });
        }
        let component_registry_request = RootComponentRegistryPreparationRequest {
            store_bootstrap: bootstrap.clone(),
            expected_fleet_registry: active_version.clone(),
        };
        actions.push(CompiledCurrentProtocolStep {
            action: CurrentFleetProtocolAction::PrepareComponentRegistry {
                expected: RootComponentRegistryStatusResponse {
                    fleet_subnet_root: root_principal,
                    prepared_against_registry: active_version.clone(),
                    release_set: authority.initial_release_set,
                    component_topology_digest: authority.binding.component_topology_digest,
                    next_allocation_sequence: 1,
                    reserved_component_instances: 0,
                    committed_component_instances: 0,
                    managed_descendants: 0,
                    known_created_component_canisters: 0,
                    encoded_bytes: 0,
                    initial_inventory: None,
                },
                request: component_registry_request,
            },
            name: format!("component-registry:{}", root_principal.to_text()),
            target: root_principal,
        });
    }
    if !matches!(
        registry_sequence.current_stage,
        CurrentRegistryStage::Active | CurrentRegistryStage::Provisioned
    ) {
        actions.push(CompiledCurrentProtocolStep {
            action: CurrentFleetProtocolAction::ActivateRegistry {
                expected_registry: registry_sequence.active_registry.clone(),
                expected_version: active_version,
                request: registry_sequence.activation_request.clone(),
            },
            name: "registry-activate".to_string(),
            target: coordinator_principal,
        });
    }
    actions.sort_by_key(|step| current_protocol_stage(&step.action));
    let placements = resolve_placements(desired, state, &registry_sequence.active_registry)?;
    let CompiledCurrentComponentProvisioning { request, plan_hash } =
        compile_current_component_provisioning(
            configuration,
            &registry_sequence.active_registry,
            operation_id,
            &placements,
        )?;
    if let Some(status) = &registry_sequence.component_status {
        require_component_status_matches(status, &request, plan_hash)?;
    }
    actions.push(CompiledCurrentProtocolStep {
        action: CurrentFleetProtocolAction::ProvisionComponents { request, plan_hash },
        name: COMPONENT_PROVISIONING_ACTION.to_string(),
        target: coordinator_principal,
    });
    Ok(actions)
}

fn require_component_status_matches(
    status: &canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
    request: &FleetComponentProvisioningPrepareRequest,
    plan_hash: [u8; 32],
) -> Result<(), CurrentProtocolError> {
    let expected = ComponentOperationAuthority {
        operation_id: request.operation_id,
        plan_hash,
        fleet_registry: &request.plan.fleet_registry,
        configuration_digest: request.plan.configuration_digest,
        operation: &request.plan.operation,
    };
    let observed = ComponentOperationAuthority {
        operation_id: status.operation_id,
        plan_hash: status.plan_hash,
        fleet_registry: &status.fleet_registry,
        configuration_digest: status.configuration_digest,
        operation: &status.operation,
    };
    if observed != expected {
        return Err(CurrentProtocolError::RegistrySequenceConflict(
            "retained Component operation differs from the current compiled plan".to_string(),
        ));
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct ComponentOperationAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    fleet_registry: &'a canic_core::dto::fleet_registry::FleetRegistryVersion,
    configuration_digest: canic_core::ids::ComponentDeploymentConfigurationDigest,
    operation: &'a FleetComponentProvisioningOperation,
}

const fn current_protocol_stage(action: &CurrentFleetProtocolAction) -> u8 {
    match action {
        CurrentFleetProtocolAction::PrepareStoreChunkSet { .. }
        | CurrentFleetProtocolAction::PublishStoreChunk { .. }
        | CurrentFleetProtocolAction::StageStoreManifest { .. }
        | CurrentFleetProtocolAction::AdoptStore { .. }
        | CurrentFleetProtocolAction::BootstrapStore { .. } => 0,
        CurrentFleetProtocolAction::RecoverPoolLedger { .. } => 1,
        CurrentFleetProtocolAction::JoinRoot { .. } => 2,
        CurrentFleetProtocolAction::SynchronizeRegistry { .. } => 3,
        CurrentFleetProtocolAction::ActivateRegistry { .. } => 4,
        CurrentFleetProtocolAction::ActivateRegistryMirror { .. } => 5,
        CurrentFleetProtocolAction::PrepareComponentRegistry { .. } => 6,
        CurrentFleetProtocolAction::ProvisionComponents { .. } => 7,
    }
}

pub(super) fn query_current_root_authorities(
    icp: &IcpCli,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    root_candid: &Path,
    store_candid: &Path,
) -> Result<Vec<FleetSubnetRootAuthority>, CurrentProtocolError> {
    let mut authorities = Vec::new();
    for configured in desired.canisters.iter().filter(|canister| {
        canister.presence == DesiredPresence::Present && canister.kind == DesiredCanisterKind::Root
    }) {
        let principal = retained_principal(desired, state, &configured.name)
            .and_then(|principal| Principal::from_text(principal).ok())
            .ok_or_else(|| {
                CurrentProtocolError::RegistrySequenceConflict(format!(
                    "Root {} has no exact Principal",
                    configured.name
                ))
            })?;
        let response: RootStatusResponseFragment = query_with_candid(
            icp,
            root_candid,
            principal,
            protocol::CANIC_STATUS,
            &RootStatusRequestFragment::FleetAuthority,
        )?;
        let RootStatusResponseFragment::FleetAuthority(authority) = response else {
            return Err(CurrentProtocolError::ResponseMismatch);
        };
        let store_response: StoreStatusResponse = query_with_candid(
            icp,
            store_candid,
            authority.wasm_store_authority.wasm_store,
            protocol::CANIC_STATUS,
            &StoreStatusRequest::Authority,
        )?;
        let StoreStatusResponse::Authority(store_authority) = store_response else {
            return Err(CurrentProtocolError::ResponseMismatch);
        };
        if store_authority != authority.wasm_store_authority {
            return Err(CurrentProtocolError::RegistrySequenceConflict(format!(
                "Root {} and Store retain different authority",
                configured.name
            )));
        }
        authorities.push(authority);
    }
    Ok(authorities)
}

#[expect(
    clippy::too_many_arguments,
    reason = "binding keeps every independent artifact and authority input explicit"
)]
fn bind_action(
    root: &Path,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    protocol_intent: &DesiredFleetProtocol,
    action: CurrentFleetProtocolAction,
    target: Principal,
    name: String,
    maximum_execution_burn_cycles: u128,
) -> Result<EnsureAction, CurrentProtocolError> {
    let target_kind = action.target_kind();
    let configured = desired
        .canisters
        .iter()
        .find(|canister| {
            canister.presence == DesiredPresence::Present
                && canister.kind == target_kind
                && retained_principal(desired, state, &canister.name)
                    .is_some_and(|principal| principal == target.to_text())
        })
        .ok_or_else(|| {
            CurrentProtocolError::Configuration(format!(
                "typed protocol target {target} is not one exact configured {target_kind:?}"
            ))
        })?;
    let candid = match configured.kind {
        DesiredCanisterKind::Coordinator => &protocol_intent.coordinator_candid,
        DesiredCanisterKind::Root => &protocol_intent.root_candid,
        DesiredCanisterKind::Store => &protocol_intent.store_candid,
        DesiredCanisterKind::Auxiliary
        | DesiredCanisterKind::Component
        | DesiredCanisterKind::Pool => {
            return Err(CurrentProtocolError::Configuration(
                "typed protocol selected a non-infrastructure target".to_string(),
            ));
        }
    };
    let candid_sha256 = read_sha256(&resolve_path(root, candid))?;
    Ok(EnsureAction::FleetProtocol {
        action: Box::new(action),
        candid: candid.clone(),
        candid_sha256,
        maximum_execution_burn_cycles,
        name,
        principal: target.to_text(),
    })
}

/// Observe exact terminal status for one retained typed protocol action.
#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive observer keeps every closed action beside its terminal predicate"
)]
pub(super) fn observe(
    icp: &IcpCli,
    root: &Path,
    action: &EnsureAction,
) -> Result<EffectObservation, CurrentProtocolError> {
    let resolved = ResolvedProtocolAction::from_action(root, action)?;
    match resolved.action {
        CurrentFleetProtocolAction::ActivateRegistry {
            expected_registry, ..
        }
        | CurrentFleetProtocolAction::JoinRoot {
            expected_registry, ..
        } => {
            let live = query_registry(icp, &resolved.candid_path, resolved.target)?;
            observation(live == *expected_registry, &live)
        }
        CurrentFleetProtocolAction::ActivateRegistryMirror { expected, request } => {
            let Some(status) = query_root_operation(
                icp,
                &resolved.candid_path,
                resolved.target,
                request.operation_id,
            )?
            else {
                return Ok(unavailable_observation());
            };
            let RootOperationStatusResponse::SynchronizeRegistry(status) = status else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            observation(status.activation.as_ref() == Some(expected), &status)
        }
        CurrentFleetProtocolAction::AdoptStore { request } => {
            let Some(status) = query_root_operation(
                icp,
                &resolved.candid_path,
                resolved.target,
                request.operation_id,
            )?
            else {
                return Ok(unavailable_observation());
            };
            let RootOperationStatusResponse::AdoptStore(status) = status else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            let applied = store_adoption_applied(request, Some(&status));
            observation(applied, &status)
        }
        CurrentFleetProtocolAction::BootstrapStore { expected, request } => {
            let Some(status) = query_root_operation(
                icp,
                &resolved.candid_path,
                resolved.target,
                request.operation_id,
            )?
            else {
                return Ok(unavailable_observation());
            };
            let RootOperationStatusResponse::BootstrapStore(status) = status else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            observation(status == *expected, &status)
        }
        CurrentFleetProtocolAction::PrepareStoreChunkSet { request } => {
            let status =
                query_store_staging(icp, &resolved, &request.template_id, &request.version)?;
            let applied = status.chunk_set_present
                && status.expected_chunk_hashes == request.chunk_hashes
                && status.payload_hash.as_deref() == Some(request.payload_hash.as_slice())
                && status.payload_size_bytes == Some(request.payload_size_bytes);
            observation(applied, &status)
        }
        CurrentFleetProtocolAction::PrepareComponentRegistry { expected, request } => {
            let response: Result<RootStatusResponseFragment, CanisterProtocolError> =
                query_with_candid(
                    icp,
                    &resolved.candid_path,
                    resolved.target,
                    protocol::CANIC_STATUS,
                    &RootStatusRequestFragment::ComponentRegistry(request.clone()),
                );
            let response = match response {
                Ok(response) => response,
                Err(error) if component_registry_status_unavailable(&error) => {
                    return Ok(unavailable_observation());
                }
                Err(error) => return Err(error.into()),
            };
            let RootStatusResponseFragment::ComponentRegistry(status) = response else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            observation(component_registry_progresses(expected, &status), &status)
        }
        CurrentFleetProtocolAction::RecoverPoolLedger { request } => {
            let Some(status) = query_root_operation(
                icp,
                &resolved.candid_path,
                resolved.target,
                request.operation_id,
            )?
            else {
                return Ok(unavailable_observation());
            };
            let RootOperationStatusResponse::RecoverPoolLedger(status) = status else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            if status.request != *request {
                return Err(CurrentProtocolError::ResponseMismatch);
            }
            observation(status.receipt.is_some(), &status)
        }
        CurrentFleetProtocolAction::ProvisionComponents { request, plan_hash } => {
            let Some(status) = query_operation(
                icp,
                &resolved.candid_path,
                resolved.target,
                request.operation_id,
            )?
            else {
                return Ok(unavailable_observation());
            };
            if status.operation_id != request.operation_id || status.plan_hash != *plan_hash {
                return Err(CurrentProtocolError::ResponseMismatch);
            }
            let applied = status.phase == FleetComponentProvisioningPhase::RuntimesActivated
                && status.published_fleet_registry.is_some()
                && status.pending_root_failure.is_none();
            component_provisioning_observation(applied, &status)
        }
        CurrentFleetProtocolAction::PublishStoreChunk { request } => {
            let status =
                query_store_staging(icp, &resolved, &request.template_id, &request.version)?;
            let expected = canic_core::cdk::utils::hash::wasm_hash(&request.bytes);
            let applied = status
                .stored_chunk_hashes
                .get(request.chunk_index as usize)
                .is_some_and(|actual| actual.as_ref() == Some(&expected));
            observation(applied, &status)
        }
        CurrentFleetProtocolAction::StageStoreManifest { request } => {
            let status =
                query_store_staging(icp, &resolved, &request.template_id, &request.version)?;
            observation(
                status.manifest.as_ref() == Some(&manifest_response(request)),
                &status,
            )
        }
        CurrentFleetProtocolAction::SynchronizeRegistry { expected, request } => {
            let Some(status) = query_root_operation(
                icp,
                &resolved.candid_path,
                resolved.target,
                request.operation_id,
            )?
            else {
                return Ok(unavailable_observation());
            };
            let RootOperationStatusResponse::SynchronizeRegistry(status) = status else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            observation(status.synchronization == *expected, &status)
        }
    }
}

fn store_adoption_applied(
    request: &FleetSubnetWasmStoreAdoptionRequest,
    status: Option<&canic_core::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse>,
) -> bool {
    status.is_some_and(|status| {
        status.operation_id == request.operation_id
            && status.authority == request.authority
            && status.controllers == expected_store_controllers(&request.authority)
    })
}

fn component_provisioning_observation(
    applied: bool,
    status: &canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
) -> Result<EffectObservation, CurrentProtocolError> {
    let mut durable_progress = status.clone();
    if let Some(failure) = &mut durable_progress.pending_root_failure {
        failure.failed_at_ns = 0;
    }
    let mut observation = observation(applied, &durable_progress)?;
    if !applied && status.pending_root_failure.is_some() {
        observation.retry = EffectRetry::ReplayExactIssuedCommand;
    }
    Ok(observation)
}

/// Issue one exact typed Coordinator request. Terminal completion remains query-owned.
#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive issuer keeps every closed action beside its response binding"
)]
pub(super) fn apply(
    icp: &IcpCli,
    root: &Path,
    action: &EnsureAction,
) -> Result<EffectOutcome, CurrentProtocolError> {
    let resolved = ResolvedProtocolAction::from_action(root, action)?;
    let receipt = match resolved.action {
        CurrentFleetProtocolAction::ActivateRegistry {
            expected_version,
            request,
            ..
        } => {
            let response: CoordinatorCommandResponse = call_with_candid(
                icp,
                &resolved.candid_path,
                resolved.target,
                protocol::CANIC_COMMAND,
                &CoordinatorCommand::ActivateRegistry(request.clone()),
            )?;
            let CoordinatorCommandResponse::ActivateRegistry(response) = response else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            if response.previous_version != request.expected_registry
                || response.version != *expected_version
            {
                return Err(CurrentProtocolError::ResponseMismatch);
            }
            expected_version.content_hash.to_vec()
        }
        CurrentFleetProtocolAction::JoinRoot {
            expected_version,
            request,
            ..
        } => {
            let response: CoordinatorCommandResponse = call_with_candid(
                icp,
                &resolved.candid_path,
                resolved.target,
                protocol::CANIC_COMMAND,
                &CoordinatorCommand::JoinRoot(request.clone()),
            )?;
            let CoordinatorCommandResponse::JoinRoot(response) = response else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            if response.entry != request.entry || response.version != *expected_version {
                return Err(CurrentProtocolError::ResponseMismatch);
            }
            expected_version.content_hash.to_vec()
        }
        CurrentFleetProtocolAction::ProvisionComponents { request, .. } => {
            let response: CoordinatorCommandResponse = call_with_candid(
                icp,
                &resolved.candid_path,
                resolved.target,
                protocol::CANIC_COMMAND,
                &CoordinatorCommand::ProvisionComponents(request.clone()),
            )?;
            operation_receipt(response, request.operation_id)?.to_vec()
        }
        CurrentFleetProtocolAction::AdoptStore { request } => root_operation_call(
            icp,
            &resolved,
            &RootCommandFragment::AdoptStore(request.clone()),
            request.operation_id,
        )?
        .to_vec(),
        CurrentFleetProtocolAction::BootstrapStore { request, .. } => root_operation_call(
            icp,
            &resolved,
            &RootCommandFragment::BootstrapStore(request.clone()),
            request.operation_id,
        )?
        .to_vec(),
        CurrentFleetProtocolAction::PrepareComponentRegistry { expected, request } => {
            let response: RootCommandResponseFragment = call_with_candid(
                icp,
                &resolved.candid_path,
                resolved.target,
                protocol::CANIC_COMMAND,
                &RootCommandFragment::PrepareComponentRegistry(request.clone()),
            )?;
            let RootCommandResponseFragment::PrepareComponentRegistry(response) = response else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            if !component_registry_progresses(expected, &response) {
                return Err(CurrentProtocolError::ResponseMismatch);
            }
            candid::encode_one(response)
                .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?
        }
        CurrentFleetProtocolAction::RecoverPoolLedger { request } => {
            let response: RootCommandResponseFragment = call_with_candid(
                icp,
                &resolved.candid_path,
                resolved.target,
                protocol::CANIC_COMMAND,
                &RootCommandFragment::RecoverPoolLedger(request.clone()),
            )?;
            let RootCommandResponseFragment::RecoverPoolLedger(receipt) = response else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            if receipt.operation_id != request.operation_id || receipt.request != *request {
                return Err(CurrentProtocolError::ResponseMismatch);
            }
            candid::encode_one(receipt)
                .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?
        }
        CurrentFleetProtocolAction::SynchronizeRegistry { request, .. }
        | CurrentFleetProtocolAction::ActivateRegistryMirror { request, .. } => {
            root_operation_call(
                icp,
                &resolved,
                &RootCommandFragment::SynchronizeRegistry(request.clone()),
                request.operation_id,
            )?
            .to_vec()
        }
        CurrentFleetProtocolAction::PrepareStoreChunkSet { request } => {
            let response: StoreCommandResponse = call_with_candid(
                icp,
                &resolved.candid_path,
                resolved.target,
                protocol::CANIC_COMMAND,
                &StoreCommand::PrepareChunkSet(request.clone()),
            )?;
            let StoreCommandResponse::PrepareChunkSet(response) = response else {
                return Err(CurrentProtocolError::ResponseMismatch);
            };
            if response.chunk_hashes != request.chunk_hashes {
                return Err(CurrentProtocolError::ResponseMismatch);
            }
            Sha256::digest(
                candid::encode_one(request)
                    .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?,
            )
            .to_vec()
        }
        CurrentFleetProtocolAction::StageStoreManifest { request } => {
            let response: StoreCommandResponse = call_with_candid(
                icp,
                &resolved.candid_path,
                resolved.target,
                protocol::CANIC_COMMAND,
                &StoreCommand::StageManifest(request.clone()),
            )?;
            if !matches!(response, StoreCommandResponse::StageManifest) {
                return Err(CurrentProtocolError::ResponseMismatch);
            }
            Sha256::digest(
                candid::encode_one(request)
                    .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?,
            )
            .to_vec()
        }
        CurrentFleetProtocolAction::PublishStoreChunk { request } => {
            call_with_candid::<_, ()>(
                icp,
                &resolved.candid_path,
                resolved.target,
                WASM_STORE_PUBLISH_CHUNK,
                request,
            )?;
            canic_core::cdk::utils::hash::wasm_hash(&request.bytes)
        }
    };
    Ok(EffectOutcome {
        created_principal: None,
        post_cycles: None,
        receipt: Some(canic_core::cdk::utils::hash::hex_bytes(receipt)),
    })
}

struct ResolvedProtocolAction<'a> {
    action: &'a CurrentFleetProtocolAction,
    candid_path: PathBuf,
    target: Principal,
}

impl<'a> ResolvedProtocolAction<'a> {
    fn from_action(root: &Path, action: &'a EnsureAction) -> Result<Self, CurrentProtocolError> {
        let EnsureAction::FleetProtocol {
            action,
            candid,
            candid_sha256,
            principal,
            ..
        } = action
        else {
            return Err(CurrentProtocolError::ResponseMismatch);
        };
        let candid_path = resolve_path(root, candid);
        if read_sha256(&candid_path)? != *candid_sha256 {
            return Err(CurrentProtocolError::ResponseMismatch);
        }
        let target = Principal::from_text(principal)
            .map_err(|_| CurrentProtocolError::CoordinatorUnavailable)?;
        Ok(Self {
            action,
            candid_path,
            target,
        })
    }
}

fn observation<T: CandidType>(
    applied: bool,
    value: &T,
) -> Result<EffectObservation, CurrentProtocolError> {
    let bytes = candid::encode_one(value)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    Ok(EffectObservation {
        applied,
        progress_identity: canic_core::cdk::utils::hash::sha256_hex(&bytes),
        retry: EffectRetry::None,
    })
}

fn unavailable_observation() -> EffectObservation {
    EffectObservation {
        applied: false,
        progress_identity: "unavailable".to_string(),
        retry: EffectRetry::None,
    }
}

fn component_registry_status_unavailable(error: &CanisterProtocolError) -> bool {
    error.is_rejected_with(canic_core::diagnostics::codes::STATE_UNAVAILABLE)
}

fn component_registry_progresses(
    expected: &RootComponentRegistryStatusResponse,
    observed: &RootComponentRegistryStatusResponse,
) -> bool {
    let authority_matches =
        ComponentRegistryAuthority::from(observed) == ComponentRegistryAuthority::from(expected);
    let counters_are_monotonic = observed.next_allocation_sequence
        >= expected.next_allocation_sequence
        && observed.reserved_component_instances >= expected.reserved_component_instances
        && observed.committed_component_instances >= expected.committed_component_instances
        && observed.managed_descendants >= expected.managed_descendants
        && observed.known_created_component_canisters >= expected.known_created_component_canisters
        && observed.encoded_bytes >= expected.encoded_bytes;
    authority_matches && counters_are_monotonic
}

#[derive(Eq, PartialEq)]
struct ComponentRegistryAuthority<'a> {
    fleet_subnet_root: Principal,
    prepared_against_registry: &'a canic_core::dto::fleet_registry::FleetRegistryVersion,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    component_topology_digest: canic_core::ids::ComponentTopologyDigest,
}

impl<'a> From<&'a RootComponentRegistryStatusResponse> for ComponentRegistryAuthority<'a> {
    fn from(status: &'a RootComponentRegistryStatusResponse) -> Self {
        Self {
            fleet_subnet_root: status.fleet_subnet_root,
            prepared_against_registry: &status.prepared_against_registry,
            release_set: status.release_set,
            component_topology_digest: status.component_topology_digest,
        }
    }
}

fn manifest_response(request: &TemplateManifestInput) -> TemplateManifestResponse {
    TemplateManifestResponse {
        template_id: request.template_id.clone(),
        role: request.role.clone(),
        version: request.version.clone(),
        payload_hash: request.payload_hash.clone(),
        payload_size_bytes: request.payload_size_bytes,
        store_binding: request.store_binding.clone(),
        chunking_mode: request.chunking_mode,
        manifest_state: request.manifest_state,
        approved_at: request.approved_at,
        created_at: request.created_at,
    }
}

fn query_store_staging(
    icp: &IcpCli,
    resolved: &ResolvedProtocolAction<'_>,
    template_id: &TemplateId,
    version: &TemplateVersion,
) -> Result<TemplateStagingStatusResponse, CurrentProtocolError> {
    let response: StoreStatusResponse = query_with_candid(
        icp,
        &resolved.candid_path,
        resolved.target,
        protocol::CANIC_STATUS,
        &StoreStatusRequest::Template(TemplateLookupRequest {
            template_id: template_id.clone(),
            version: version.clone(),
        }),
    )?;
    let StoreStatusResponse::Template(status) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    Ok(status)
}

fn query_root_operation(
    icp: &IcpCli,
    candid_path: &Path,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<Option<RootOperationStatusResponse>, CurrentProtocolError> {
    let response: Result<RootStatusResponseFragment, CanisterProtocolError> = query_with_candid(
        icp,
        candid_path,
        root,
        protocol::CANIC_STATUS,
        &RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
    );
    let response = match response {
        Ok(response) => response,
        Err(error) if error.is_rejected_with(canic_core::diagnostics::codes::STATE_UNAVAILABLE) => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    let RootStatusResponseFragment::Operation(status) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    Ok(Some(status))
}

fn root_operation_call(
    icp: &IcpCli,
    resolved: &ResolvedProtocolAction<'_>,
    command: &RootCommandFragment,
    operation_id: [u8; 32],
) -> Result<[u8; 32], CurrentProtocolError> {
    let response: RootCommandResponseFragment = call_with_candid(
        icp,
        &resolved.candid_path,
        resolved.target,
        protocol::CANIC_COMMAND,
        command,
    )?;
    let RootCommandResponseFragment::OperationAccepted(receipt) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    if receipt.operation_id != operation_id {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(operation_id)
}

fn operation_receipt(
    response: CoordinatorCommandResponse,
    expected_operation_id: [u8; 32],
) -> Result<[u8; 32], CurrentProtocolError> {
    let CoordinatorCommandResponse::OperationAccepted(receipt) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    if receipt.operation_id != expected_operation_id {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(expected_operation_id)
}

pub(super) fn resolve_placements(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    registry: &FleetRegistry,
) -> Result<Vec<CurrentComponentGroupPlacement>, CurrentProtocolError> {
    let protocol_intent = desired.protocol.as_ref().ok_or_else(|| {
        CurrentProtocolError::Configuration("missing protocol intent".to_string())
    })?;
    let mut roots_by_name = BTreeMap::new();
    for configured in desired.canisters.iter().filter(|canister| {
        canister.presence == DesiredPresence::Present
            && canister.kind == crate::fleet_ensure::model::DesiredCanisterKind::Root
    }) {
        let principal = retained_principal(desired, state, &configured.name)
            .ok_or_else(|| CurrentProtocolError::InvalidPlacement(configured.name.clone()))?;
        let principal = Principal::from_text(principal)
            .map_err(|_| CurrentProtocolError::InvalidPlacement(configured.name.clone()))?;
        roots_by_name.insert(configured.name.as_str(), principal);
    }
    let registered = registry
        .fleet_subnet_roots
        .iter()
        .map(|root| root.fleet_subnet_root)
        .collect::<BTreeSet<_>>();
    let configured = roots_by_name.values().copied().collect::<BTreeSet<_>>();
    if configured != registered {
        return Err(CurrentProtocolError::RegistryNotActive);
    }
    protocol_intent
        .component_group_placements
        .iter()
        .map(|placement| {
            let deployment = placement.deployment.parse().map_err(|_| {
                CurrentProtocolError::InvalidPlacement(placement.deployment.clone())
            })?;
            let fleet_subnet_root = roots_by_name
                .get(placement.root.as_str())
                .copied()
                .ok_or_else(|| CurrentProtocolError::InvalidPlacement(placement.root.clone()))?;
            Ok(CurrentComponentGroupPlacement {
                deployment,
                fleet_subnet_root,
                ordinal: placement.ordinal,
            })
        })
        .collect()
}

/// Compile the sole deterministic genesis-to-Active Registry sequence.
///
/// Root and Store authority comes from the installed canisters themselves. The
/// desired document contributes only role relationships and exact Principals;
/// it cannot restate or widen admission, funding, release-set, or placement
/// authority.
pub fn compile_current_registry_sequence(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    topology: &ComponentTopology,
    current: &FleetRegistry,
    root_authorities: &[FleetSubnetRootAuthority],
) -> Result<CompiledCurrentRegistrySequence, CurrentProtocolError> {
    compile_current_registry_sequence_with_status(
        desired,
        state,
        topology,
        current,
        root_authorities,
        None,
    )
}

/// Compile the current Registry chain while binding an observed provisioning successor.
pub fn compile_current_registry_sequence_with_status(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    topology: &ComponentTopology,
    current: &FleetRegistry,
    root_authorities: &[FleetSubnetRootAuthority],
    component_status: Option<
        &canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
    >,
) -> Result<CompiledCurrentRegistrySequence, CurrentProtocolError> {
    let genesis = FleetRegistryOps::compile_genesis(
        &current.authority.binding.fleet.app,
        current.authority.clone(),
        topology,
        current.admission.clone(),
    )
    .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let entries = compile_current_root_entries(desired, state, current, root_authorities)?;
    let mut previous = genesis.clone();
    let mut joins = Vec::with_capacity(entries.len());
    for entry in entries {
        let request = canic_core::dto::fleet_registry::FleetSubnetRootJoinRequest {
            expected_registry: registry_version(topology, &previous)?,
            entry: entry.clone(),
        };
        let resulting_registry =
            FleetRegistryOps::compile_joining(&current.authority, topology, &previous, entry)
                .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
        joins.push(CompiledCurrentRegistryJoin {
            request,
            resulting_registry: resulting_registry.clone(),
        });
        previous = resulting_registry;
    }
    let activation_request = canic_core::dto::fleet_registry::FleetRegistryActivationRequest {
        expected_registry: registry_version(topology, &previous)?,
    };
    let active_registry = FleetRegistryOps::compile_active(&current.authority, topology, &previous)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let current_stage = identify_registry_stage(
        topology,
        current,
        &genesis,
        &joins,
        &active_registry,
        component_status,
    )?;
    Ok(CompiledCurrentRegistrySequence {
        activation_request,
        active_registry,
        current_stage,
        component_status: component_status.cloned(),
        genesis,
        joins,
    })
}

fn compile_current_root_entries(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    registry: &FleetRegistry,
    root_authorities: &[FleetSubnetRootAuthority],
) -> Result<Vec<FleetSubnetRootEntry>, CurrentProtocolError> {
    let coordinator = desired
        .canisters
        .iter()
        .find(|canister| {
            canister.presence == DesiredPresence::Present
                && canister.kind == crate::fleet_ensure::model::DesiredCanisterKind::Coordinator
        })
        .ok_or(CurrentProtocolError::CoordinatorUnavailable)?;
    let coordinator_principal = retained_principal(desired, state, &coordinator.name)
        .ok_or(CurrentProtocolError::CoordinatorUnavailable)?;
    if coordinator_principal != registry.authority.binding.coordinator.to_string() {
        return Err(CurrentProtocolError::RegistrySequenceConflict(
            "Coordinator identity differs from Registry authority".to_string(),
        ));
    }
    let operator = Principal::from_text(&desired.operator).map_err(|_| {
        CurrentProtocolError::RegistrySequenceConflict("operator is not a Principal".to_string())
    })?;
    let configured_roots = desired
        .canisters
        .iter()
        .filter(|canister| {
            canister.presence == DesiredPresence::Present
                && canister.kind == crate::fleet_ensure::model::DesiredCanisterKind::Root
        })
        .collect::<Vec<_>>();
    if configured_roots.len() != root_authorities.len() {
        return Err(CurrentProtocolError::RegistrySequenceConflict(
            "configured and observed Root sets differ".to_string(),
        ));
    }
    let mut entries = Vec::with_capacity(root_authorities.len());
    let mut observed_roots = BTreeSet::new();
    for root in configured_roots {
        if root.parent.as_deref() != Some(coordinator.name.as_str()) {
            return Err(CurrentProtocolError::RegistrySequenceConflict(format!(
                "Root {} is not attached to the Coordinator",
                root.name
            )));
        }
        let root_principal = retained_principal(desired, state, &root.name)
            .and_then(|principal| Principal::from_text(principal).ok())
            .ok_or_else(|| {
                CurrentProtocolError::RegistrySequenceConflict(format!(
                    "Root {} has no exact Principal",
                    root.name
                ))
            })?;
        let authority = root_authorities
            .iter()
            .find(|authority| authority.binding.fleet_subnet_root == root_principal)
            .ok_or_else(|| {
                CurrentProtocolError::RegistrySequenceConflict(format!(
                    "Root {} has no matching live authority",
                    root.name
                ))
            })?;
        if !observed_roots.insert(root_principal)
            || authority.binding.authority != registry.authority
            || authority.binding.placement_subnet.to_string() != root.subnet
            || authority.wasm_store_authority.authority != registry.authority
            || authority.wasm_store_authority.placement_subnet != authority.binding.placement_subnet
            || authority.wasm_store_authority.fleet_subnet_root != root_principal
            || authority.wasm_store_authority.installation_controller != operator
        {
            return Err(CurrentProtocolError::RegistrySequenceConflict(format!(
                "Root {} authority is not exact",
                root.name
            )));
        }
        require_exact_store(desired, state, root, authority)?;
        entries.push(FleetSubnetRootEntry {
            placement_subnet: authority.binding.placement_subnet,
            fleet_subnet_root: root_principal,
            component_admissions: authority.binding.component_admissions.clone(),
            component_topology_digest: authority.binding.component_topology_digest,
            active_release_set: authority.initial_release_set,
            limits: authority.binding.limits.clone(),
            funding: authority.binding.funding.clone(),
            status: FleetSubnetRootStatus::Joining,
        });
    }
    entries.sort_unstable_by_key(|entry| entry.placement_subnet);
    Ok(entries)
}

fn require_exact_store(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    root: &crate::fleet_ensure::model::DesiredCanister,
    authority: &FleetSubnetRootAuthority,
) -> Result<(), CurrentProtocolError> {
    let stores = desired
        .canisters
        .iter()
        .filter(|canister| {
            canister.presence == DesiredPresence::Present
                && canister.kind == crate::fleet_ensure::model::DesiredCanisterKind::Store
                && canister.parent.as_deref() == Some(root.name.as_str())
        })
        .collect::<Vec<_>>();
    let [store] = stores.as_slice() else {
        return Err(CurrentProtocolError::RegistrySequenceConflict(format!(
            "Root {} does not have exactly one Store",
            root.name
        )));
    };
    let store_principal = retained_principal(desired, state, &store.name)
        .and_then(|principal| Principal::from_text(principal).ok())
        .ok_or_else(|| {
            CurrentProtocolError::RegistrySequenceConflict(format!(
                "Store {} has no exact Principal",
                store.name
            ))
        })?;
    let mut configured_controllers = store
        .controllers
        .iter()
        .map(Principal::from_text)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            CurrentProtocolError::RegistrySequenceConflict(format!(
                "Store {} has an invalid controller",
                store.name
            ))
        })?;
    for controller in &store.controller_canisters {
        let principal = retained_principal(desired, state, controller)
            .and_then(|principal| Principal::from_text(principal).ok())
            .ok_or_else(|| {
                CurrentProtocolError::RegistrySequenceConflict(format!(
                    "Store {} controller {controller} has no exact Principal",
                    store.name
                ))
            })?;
        configured_controllers.push(principal);
    }
    configured_controllers.sort_unstable();
    configured_controllers.dedup();
    if store_principal != authority.wasm_store_authority.wasm_store
        || store.subnet != authority.binding.placement_subnet.to_string()
        || configured_controllers != expected_store_controllers(&authority.wasm_store_authority)
    {
        return Err(CurrentProtocolError::RegistrySequenceConflict(format!(
            "Store {} authority is not exact",
            store.name
        )));
    }
    Ok(())
}

fn expected_store_controllers(authority: &FleetSubnetWasmStoreAuthority) -> Vec<Principal> {
    let mut controllers = vec![
        authority.fleet_subnet_root,
        authority.installation_controller,
    ];
    controllers.sort_unstable();
    controllers.dedup();
    controllers
}

fn identify_registry_stage(
    topology: &ComponentTopology,
    current: &FleetRegistry,
    genesis: &FleetRegistry,
    joins: &[CompiledCurrentRegistryJoin],
    active: &FleetRegistry,
    component_status: Option<
        &canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
    >,
) -> Result<CurrentRegistryStage, CurrentProtocolError> {
    if let Some(published) = component_status.and_then(|status| {
        status
            .published_fleet_registry
            .as_ref()
            .map(|version| (status, version))
    }) {
        let (status, published) = published;
        let current_version = registry_version(topology, current)?;
        let active_version = registry_version(topology, active)?;
        let mut current_without_services = current.clone();
        current_without_services.services.clear();
        current_without_services.revision = active.revision;
        let publication_receipt_matches = current_version == *published
            && status.fleet_registry == active_version
            && status.operation == FleetComponentProvisioningOperation::FreshInstall;
        let infrastructure_authority_matches = current_without_services == *active;
        if publication_receipt_matches && infrastructure_authority_matches {
            return Ok(CurrentRegistryStage::Provisioned);
        }
        return Err(CurrentProtocolError::RegistrySequenceConflict(
            "live Registry differs from its retained Component publication".to_string(),
        ));
    }
    if current == genesis {
        return Ok(CurrentRegistryStage::Genesis);
    }
    if let Some(index) = joins
        .iter()
        .position(|join| &join.resulting_registry == current)
    {
        return Ok(CurrentRegistryStage::Joining(index + 1));
    }
    if current == active {
        return Ok(CurrentRegistryStage::Active);
    }
    Err(CurrentProtocolError::RegistrySequenceConflict(
        "live Registry does not equal genesis, a canonical Joining prefix, or Active".to_string(),
    ))
}

fn registry_version(
    topology: &ComponentTopology,
    registry: &FleetRegistry,
) -> Result<canic_core::dto::fleet_registry::FleetRegistryVersion, CurrentProtocolError> {
    FleetRegistryOps::version(&registry.authority, topology, registry)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))
}

/// Compile one exact Store publication and Root bootstrap from current build evidence.
pub fn compile_current_store_sequence(
    root: &Path,
    topology: &ComponentTopology,
    authority: &FleetSubnetRootAuthority,
    operation_id: [u8; 32],
) -> Result<CompiledCurrentStoreSequence, CurrentProtocolError> {
    let persisted = load_persisted_application_artifact_union(
        root,
        topology,
        authority.initial_release_set.release_build_id,
    )
    .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let mut sequence = compile_current_store_sequence_from_union(
        root,
        topology,
        authority,
        operation_id,
        &persisted.union,
    )?;
    let infrastructure = load_persisted_canic_infrastructure_artifact_manifest(
        root,
        authority.initial_release_set.release_build_id,
    )
    .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let helper = infrastructure
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::PoolLedgerRecovery)
        .ok_or_else(|| {
            CurrentProtocolError::Configuration(
                "current infrastructure manifest omits pool Ledger recovery helper".to_string(),
            )
        })?;
    let bytes = read_infrastructure_artifact(root, helper)?;
    let artifact = PoolLedgerRecoveryArtifact {
        candid_sha256: helper.candid_sha256,
        payload_hash: decode_sha256(&helper.wasm_gz_sha256_hex)?,
        payload_size_bytes: helper.wasm_gz_size_bytes,
        raw_module_hash: decode_sha256(&helper.wasm_sha256_hex)?,
        release_build_id: authority.initial_release_set.release_build_id,
    };
    append_qualified_pool_ledger_recovery_artifact(&mut sequence, artifact, &bytes)?;
    Ok(sequence)
}

/// Append one qualified temporary recovery helper after the exact Root bootstrap.
///
/// Production calls this only after validating the persisted infrastructure
/// manifest and artifact bytes. The public boundary exists for governed
/// PocketIC fixtures to exercise the same ordering and publication compiler.
#[doc(hidden)]
pub fn append_qualified_pool_ledger_recovery_artifact(
    sequence: &mut CompiledCurrentStoreSequence,
    artifact: PoolLedgerRecoveryArtifact,
    bytes: &[u8],
) -> Result<(), CurrentProtocolError> {
    if sequence.pool_ledger_recovery_artifact.is_some() {
        return Err(CurrentProtocolError::Configuration(
            "Store sequence already contains a pool Ledger recovery helper".to_string(),
        ));
    }
    if artifact.release_build_id != sequence.expected_bootstrap.release_set.release_build_id {
        return Err(CurrentProtocolError::Configuration(
            "pool Ledger recovery helper release differs from Root bootstrap".to_string(),
        ));
    }
    let payload_hash = canic_core::cdk::utils::hash::wasm_hash(bytes);
    if bytes.is_empty()
        || artifact.payload_size_bytes != bytes.len() as u64
        || artifact.payload_hash.as_slice() != payload_hash.as_slice()
    {
        return Err(CurrentProtocolError::Configuration(
            "pool Ledger recovery helper bytes differ from qualified evidence".to_string(),
        ));
    }
    let role = CanisterRole::owned("pool_ledger_recovery".to_string());
    if sequence
        .expected_bootstrap
        .catalog
        .iter()
        .any(|entry| entry.role == role)
    {
        return Err(CurrentProtocolError::Configuration(
            "pool Ledger recovery helper must not enter the application catalog".to_string(),
        ));
    }
    let template_id = TemplateId::owned("canic:pool-ledger-recovery".to_string());
    let version = TemplateVersion::owned(artifact.release_build_id.to_string());
    let mut helper_actions = vec![CurrentFleetProtocolAction::StageStoreManifest {
        request: TemplateManifestInput {
            template_id: template_id.clone(),
            role,
            version: version.clone(),
            payload_hash,
            payload_size_bytes: artifact.payload_size_bytes,
            store_binding: WasmStoreBinding::new("bootstrap"),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(0),
            created_at: 0,
        },
    }];
    append_chunk_actions(&mut helper_actions, template_id, version, bytes)?;
    let insertion = sequence
        .actions
        .iter()
        .position(|action| matches!(action, CurrentFleetProtocolAction::BootstrapStore { .. }))
        .map(|index| index + 1)
        .ok_or_else(|| {
            CurrentProtocolError::Configuration("Store sequence omits bootstrap".to_string())
        })?;
    // Root bootstraps the exact application catalog first. The temporary
    // recovery helper remains available in Store, but is not part of that
    // initial application release-set contract.
    sequence
        .actions
        .splice(insertion..insertion, helper_actions);
    sequence.pool_ledger_recovery_artifact = Some(artifact);
    Ok(())
}

/// Compile Store actions from one already-qualified current artifact union.
///
/// This is the deterministic core used after production persistence validation
/// and by the governed production-boundary fixture.
#[expect(
    clippy::too_many_lines,
    reason = "one compiler keeps publication order and its exact artifact bindings together"
)]
pub fn compile_current_store_sequence_from_union(
    root: &Path,
    topology: &ComponentTopology,
    authority: &FleetSubnetRootAuthority,
    operation_id: [u8; 32],
    union: &ApplicationArtifactUnion,
) -> Result<CompiledCurrentStoreSequence, CurrentProtocolError> {
    union
        .validate_against(topology)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    if union.release_build_id != authority.initial_release_set.release_build_id {
        return Err(CurrentProtocolError::Configuration(
            "application artifact union release build differs from Root authority".to_string(),
        ));
    }
    let manifest = FleetSubnetRootReleaseSetManifest::project(topology, &authority.binding, union)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let manifest_bytes = serde_json::to_vec(&manifest.root_store_manifest())
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let manifest_digest: [u8; 32] = Sha256::digest(&manifest_bytes).into();
    if manifest_bytes.is_empty()
        || manifest_bytes.len() as u64 > ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES
        || &manifest_digest != authority.initial_release_set.manifest_digest.as_bytes()
    {
        return Err(CurrentProtocolError::Configuration(
            "root release-set manifest differs from installed authority".to_string(),
        ));
    }
    let version =
        TemplateVersion::owned(authority.initial_release_set.release_build_id.to_string());
    let mut actions = Vec::new();
    append_chunk_actions(
        &mut actions,
        TemplateId::owned(format!(
            "{ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX}{}",
            authority.initial_release_set.manifest_digest
        )),
        version.clone(),
        &manifest_bytes,
    )?;

    let mut artifacts = BTreeMap::<CanisterRole, &ApplicationArtifactEntry>::new();
    for entry in &manifest.entries {
        match artifacts.insert(entry.artifact.role.clone(), &entry.artifact) {
            Some(existing) if existing != &entry.artifact => {
                return Err(CurrentProtocolError::Configuration(
                    "one Store role has conflicting qualified artifacts".to_string(),
                ));
            }
            _ => {}
        }
    }
    let mut catalog = Vec::with_capacity(artifacts.len());
    for (role, artifact) in artifacts {
        let bytes = read_qualified_artifact(root, artifact)?;
        let payload_hash = canic_core::cdk::utils::hash::wasm_hash(&bytes);
        let template_id = TemplateId::owned(format!("{ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX}{role}"));
        actions.push(CurrentFleetProtocolAction::StageStoreManifest {
            request: TemplateManifestInput {
                template_id: template_id.clone(),
                role: role.clone(),
                version: version.clone(),
                payload_hash: payload_hash.clone(),
                payload_size_bytes: bytes.len() as u64,
                store_binding: WasmStoreBinding::new("bootstrap"),
                chunking_mode: TemplateChunkingMode::Chunked,
                manifest_state: TemplateManifestState::Approved,
                approved_at: Some(0),
                created_at: 0,
            },
        });
        append_chunk_actions(&mut actions, template_id, version.clone(), &bytes)?;
        catalog.push(RootStoreCatalogEntry {
            role,
            raw_module_hash: decode_sha256(&artifact.wasm_sha256_hex)?,
            candid_sha256: artifact.candid_sha256,
            protocol_profile_digest: artifact.protocol_profile_digest,
            payload_hash: decode_sha256(&artifact.wasm_gz_sha256_hex)?,
            payload_size_bytes: artifact.wasm_gz_size_bytes,
        });
    }

    let adoption_operation_id = derived_operation_id(
        operation_id,
        b"store-adoption",
        authority.binding.fleet_subnet_root,
    );
    actions.push(CurrentFleetProtocolAction::AdoptStore {
        request: FleetSubnetWasmStoreAdoptionRequest {
            operation_id: adoption_operation_id,
            authority: authority.wasm_store_authority.clone(),
        },
    });
    let bootstrap_request = RootStoreBootstrapRequest {
        operation_id: derived_operation_id(
            operation_id,
            b"store-bootstrap",
            authority.binding.fleet_subnet_root,
        ),
        manifest_payload_size_bytes: manifest_bytes.len() as u64,
    };
    let expected_bootstrap = RootStoreBootstrapResponse {
        fleet_subnet_root: authority.binding.fleet_subnet_root,
        wasm_store: authority.wasm_store_authority.wasm_store,
        release_set: authority.initial_release_set,
        catalog,
    };
    actions.push(CurrentFleetProtocolAction::BootstrapStore {
        expected: expected_bootstrap.clone(),
        request: bootstrap_request.clone(),
    });
    Ok(CompiledCurrentStoreSequence {
        actions,
        bootstrap_request,
        expected_bootstrap,
        pool_ledger_recovery_artifact: None,
    })
}

fn append_chunk_actions(
    actions: &mut Vec<CurrentFleetProtocolAction>,
    template_id: TemplateId,
    version: TemplateVersion,
    bytes: &[u8],
) -> Result<(), CurrentProtocolError> {
    let chunks = bytes
        .chunks(canic_core::CANIC_WASM_CHUNK_BYTES)
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    let chunk_hashes = chunks
        .iter()
        .map(|chunk| canic_core::cdk::utils::hash::wasm_hash(chunk))
        .collect::<Vec<_>>();
    actions.push(CurrentFleetProtocolAction::PrepareStoreChunkSet {
        request: TemplateChunkSetPrepareInput {
            template_id: template_id.clone(),
            version: version.clone(),
            payload_hash: canic_core::cdk::utils::hash::wasm_hash(bytes),
            payload_size_bytes: bytes.len() as u64,
            chunk_hashes,
        },
    });
    for (index, bytes) in chunks.into_iter().enumerate() {
        actions.push(CurrentFleetProtocolAction::PublishStoreChunk {
            request: TemplateChunkInput {
                template_id: template_id.clone(),
                version: version.clone(),
                chunk_index: u32::try_from(index).map_err(|_| {
                    CurrentProtocolError::Configuration(
                        "Store artifact has too many chunks".to_string(),
                    )
                })?,
                bytes,
            },
        });
    }
    Ok(())
}

fn read_qualified_artifact(
    root: &Path,
    artifact: &ApplicationArtifactEntry,
) -> Result<Vec<u8>, CurrentProtocolError> {
    validate_release_artifact_relative_path(&artifact.wasm_gz_relative_path)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let path = root.join(&artifact.wasm_gz_relative_path);
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let canonical_parent = path
        .parent()
        .ok_or_else(|| CurrentProtocolError::Configuration("artifact has no parent".to_string()))?
        .canonicalize()
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(CurrentProtocolError::Configuration(
            "Store artifact escapes the workspace".to_string(),
        ));
    }
    let bytes = crate::durable_io::read_optional_regular_bytes(&path)
        .map_err(|error| match error {
            crate::durable_io::RegularFileReadError::NotRegular => {
                CurrentProtocolError::Configuration(format!(
                    "Store artifact is not a regular no-follow file: {}",
                    path.display()
                ))
            }
            crate::durable_io::RegularFileReadError::Io(source) => {
                CurrentProtocolError::Configuration(source.to_string())
            }
            #[cfg(not(unix))]
            crate::durable_io::RegularFileReadError::UnsupportedPlatform => {
                CurrentProtocolError::Configuration(
                    "Store artifact reads are unsupported on this platform".to_string(),
                )
            }
        })?
        .ok_or_else(|| {
            CurrentProtocolError::Configuration(format!("missing {}", path.display()))
        })?;
    if bytes.len() as u64 != artifact.wasm_gz_size_bytes
        || canic_core::cdk::utils::hash::hex_bytes(Sha256::digest(&bytes))
            != artifact.wasm_gz_sha256_hex
    {
        return Err(CurrentProtocolError::Configuration(format!(
            "Store artifact {} differs from qualified evidence",
            artifact.role
        )));
    }
    Ok(bytes)
}

fn read_infrastructure_artifact(
    root: &Path,
    artifact: &crate::release_set::CanicInfrastructureArtifactEntry,
) -> Result<Vec<u8>, CurrentProtocolError> {
    validate_release_artifact_relative_path(&artifact.wasm_gz_relative_path)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let bytes = fs::read(root.join(&artifact.wasm_gz_relative_path))
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    if bytes.len() as u64 != artifact.wasm_gz_size_bytes
        || canic_core::cdk::utils::hash::sha256_hex(&bytes) != artifact.wasm_gz_sha256_hex
    {
        return Err(CurrentProtocolError::Configuration(
            "pool Ledger recovery helper differs from its infrastructure manifest".to_string(),
        ));
    }
    Ok(bytes)
}

fn decode_sha256(value: &str) -> Result<[u8; 32], CurrentProtocolError> {
    canic_core::cdk::utils::hash::decode_hex(value)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| CurrentProtocolError::Configuration("invalid SHA-256 identity".to_string()))
}

fn derived_operation_id(operation_id: [u8; 32], phase: &[u8], subject: Principal) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(CURRENT_PROTOCOL_OPERATION_DOMAIN);
    hasher.update(operation_id);
    hasher.update((phase.len() as u64).to_be_bytes());
    hasher.update(phase);
    hasher.update((subject.as_slice().len() as u64).to_be_bytes());
    hasher.update(subject.as_slice());
    hasher.finalize().into()
}

/// Compile checked-in topology and exact active Registry authority into one request.
pub fn compile_current_component_provisioning(
    configuration: &ComponentDeploymentConfiguration,
    registry: &FleetRegistry,
    operation_id: [u8; 32],
    placements: &[CurrentComponentGroupPlacement],
) -> Result<CompiledCurrentComponentProvisioning, CurrentProtocolError> {
    let by_root = compile_placement_assignments(placements, configuration)?;
    let mut batches = compile_root_batches(registry, configuration, by_root)?;
    batches.sort_unstable_by_key(|batch| batch.root.fleet_subnet_root);
    let mut directory_confirmation_roots = registry
        .fleet_subnet_roots
        .iter()
        .map(|entry| entry.fleet_subnet_root)
        .collect::<Vec<_>>();
    directory_confirmation_roots.sort_unstable();
    let fleet_registry = FleetRegistryOps::version(
        &registry.authority,
        &configuration.component_topology,
        registry,
    )
    .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let configuration_digest = configuration
        .digest()
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    let plan = FleetComponentProvisioningPlan {
        fleet: registry.authority.binding.fleet.clone(),
        fleet_registry,
        configuration_digest,
        operation: FleetComponentProvisioningOperation::FreshInstall,
        directory_confirmation_roots,
        batches,
    };
    let plan_hash = ComponentProvisioningPlanOps::hash_compiled(configuration, registry, &plan)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    Ok(CompiledCurrentComponentProvisioning {
        plan_hash,
        request: FleetComponentProvisioningPrepareRequest { operation_id, plan },
    })
}

fn compile_placement_assignments(
    placements: &[CurrentComponentGroupPlacement],
    configuration: &ComponentDeploymentConfiguration,
) -> Result<BTreeMap<Principal, Vec<(ComponentGroupDeploymentId, u32)>>, CurrentProtocolError> {
    let expected_placements = configuration
        .deployment_topology
        .component_group_deployments
        .iter()
        .flat_map(|deployment| {
            (0..deployment.initial_placements)
                .map(move |ordinal| (deployment.deployment.clone(), ordinal))
        })
        .collect::<BTreeSet<_>>();
    let mut actual_placements = BTreeSet::new();
    let mut by_root = BTreeMap::<Principal, Vec<_>>::new();
    for placement in placements {
        if !actual_placements.insert((placement.deployment.clone(), placement.ordinal)) {
            return Err(CurrentProtocolError::InvalidPlacement(format!(
                "duplicate {}:{}",
                placement.deployment, placement.ordinal
            )));
        }
        by_root
            .entry(placement.fleet_subnet_root)
            .or_default()
            .push((placement.deployment.clone(), placement.ordinal));
    }
    if actual_placements != expected_placements {
        return Err(CurrentProtocolError::InvalidPlacement(
            "placements do not exactly cover configured initial placements".to_string(),
        ));
    }
    Ok(by_root)
}

fn compile_root_batches(
    registry: &FleetRegistry,
    configuration: &ComponentDeploymentConfiguration,
    mut by_root: BTreeMap<Principal, Vec<(ComponentGroupDeploymentId, u32)>>,
) -> Result<Vec<FleetSubnetRootProvisioningBatch>, CurrentProtocolError> {
    let mut batches = Vec::with_capacity(registry.fleet_subnet_roots.len());
    for entry in &registry.fleet_subnet_roots {
        let mut placements = by_root
            .remove(&entry.fleet_subnet_root)
            .unwrap_or_default()
            .into_iter()
            .map(|(deployment_id, ordinal)| {
                compile_placement(configuration, deployment_id, ordinal)
            })
            .collect::<Result<Vec<_>, CurrentProtocolError>>()?;
        placements.sort_unstable_by(|left, right| left.group_placement.cmp(&right.group_placement));
        batches.push(FleetSubnetRootProvisioningBatch {
            root: root_binding(registry, entry),
            active_release_set: entry.active_release_set,
            placements,
        });
    }
    if !by_root.is_empty() {
        return Err(CurrentProtocolError::RegistryNotActive);
    }
    Ok(batches)
}

fn compile_placement(
    configuration: &ComponentDeploymentConfiguration,
    deployment_id: ComponentGroupDeploymentId,
    ordinal: u32,
) -> Result<ComponentGroupPlacementPlan, CurrentProtocolError> {
    let deployment = configuration
        .deployment_topology
        .component_group_deployments
        .iter()
        .find(|deployment| deployment.deployment == deployment_id)
        .ok_or_else(|| CurrentProtocolError::InvalidPlacement(deployment_id.to_string()))?;
    Ok(ComponentGroupPlacementPlan {
        group_placement: ComponentGroupPlacementId {
            deployment: deployment_id,
            ordinal,
        },
        component_group: deployment.component_group.clone(),
        entries: deployment
            .members
            .iter()
            .map(|member| ComponentGroupPlanEntry {
                member_path: member.member_path.clone(),
                component_spec: member.component_spec.clone(),
                spec_hash: member.component_spec_hash,
                purpose: member.purpose.clone(),
                labels: member.labels.clone(),
                limits: member.limits.clone(),
            })
            .collect(),
    })
}

fn root_binding(registry: &FleetRegistry, root: &FleetSubnetRootEntry) -> FleetSubnetRootBinding {
    FleetSubnetRootBinding {
        authority: registry.authority.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
        funding: root.funding.clone(),
    }
}

pub(super) fn query_registry(
    icp: &IcpCli,
    candid: &Path,
    coordinator: Principal,
) -> Result<FleetRegistry, CurrentProtocolError> {
    let response: CoordinatorStatusResponse = query_with_candid(
        icp,
        candid,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::Registry,
    )?;
    let CoordinatorStatusResponse::Registry(registry) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    Ok(registry)
}

fn query_operation(
    icp: &IcpCli,
    candid: &Path,
    coordinator: Principal,
    operation_id: [u8; 32],
) -> Result<
    Option<canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse>,
    CurrentProtocolError,
> {
    let response: Result<CoordinatorStatusResponse, CanisterProtocolError> = query_with_candid(
        icp,
        candid,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::Operation(OperationStatusRequest { operation_id }),
    );
    let response = match response {
        Ok(response) => response,
        Err(error) if error.is_rejected_with(canic_core::diagnostics::codes::STATE_UNAVAILABLE) => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    let CoordinatorStatusResponse::Operation(
        CoordinatorOperationStatusResponse::ComponentProvisioning(status),
    ) = response
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    Ok(Some(status))
}

fn operation_bytes(operation_id: &str) -> Result<[u8; 32], CurrentProtocolError> {
    canic_core::cdk::utils::hash::decode_hex(operation_id)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(CurrentProtocolError::InvalidOperationIdentity)
}

fn retained_principal(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    name: &str,
) -> Option<String> {
    state
        .pending_principals
        .get(name)
        .or_else(|| state.principals.get(name))
        .cloned()
        .or_else(|| {
            desired
                .canisters
                .iter()
                .find(|canister| canister.name == name)
                .and_then(|canister| canister.principal.clone())
        })
}

fn read_sha256(path: &Path) -> Result<String, CurrentProtocolError> {
    let bytes = std::fs::read(path).map_err(|source| CurrentProtocolError::ReadCandid {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(canic_core::cdk::utils::hash::hex_bytes(Sha256::digest(
        bytes,
    )))
}

fn resolve_path(root: &Path, configured: &str) -> PathBuf {
    let configured = Path::new(configured);
    if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        root.join(configured)
    }
}
