//! Seal supplied custody and compile initialization through the existing Ensure effect contract.
//!
//! Workflow owns observation budgets, approval and persistence before effects.

pub(in crate::fleet_ensure) mod automatic;
pub(in crate::fleet_ensure) mod convergence;
pub(in crate::fleet_ensure) mod declarations;
pub(in crate::fleet_ensure) mod inspection;
pub(in crate::fleet_ensure) mod publication;
pub(in crate::fleet_ensure) mod registration;
pub(in crate::fleet_ensure) mod survey;
pub(in crate::fleet_ensure) mod terminal;
#[cfg(test)]
pub(in crate::fleet_ensure::ops) mod tests;

use crate::{
    fleet_ensure::{
        model::{
            CanisterRuntimeStatus, DesiredFleet, FleetEnsurePlan, FleetEnsurePlanScope,
            FleetEnsureStateRecord, FleetObservation, LiveCanister,
            infrastructure_bootstrap::{
                BootstrapCoordinatorSelection, InfrastructureBootstrapRecord,
            },
        },
        ops::{
            EnsureStateError, canic_init,
            capacity_import::{
                admission::observer::{inventory, management},
                journal::CapacityImportJournalError,
            },
            resolve_desired_artifacts,
        },
        policy::{EnsurePolicyError, infrastructure_bootstrap},
        view::infrastructure_bootstrap::InfrastructureBootstrapObservation,
    },
    icp::IcpCli,
};
use candid::{CandidType, Nat, Principal};
use canic_core::{
    cdk::utils::hash::{decode_hex, hex_bytes},
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::fleet_registry::FleetRegistry,
    ids::CanonicalNetworkId,
    shared_support::fleet_admission_policy::bind_initial_fleet_admission_policy,
};
use sha2_host::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use thiserror::Error;

/// A bootstrap failure preserves supplied identities and all retained effect receipts.
#[derive(Debug, Error)]
pub enum InfrastructureBootstrapError {
    #[error(
        "bootstrap inspection allowance exhausted; retained effects and original balances remain authoritative"
    )]
    InspectionBudget,
    #[error("infrastructure bootstrap source, plan or current authority differs")]
    Integrity,
    #[error("bootstrap observation is unavailable: {0}")]
    Observation(String),
    #[error(transparent)]
    Policy(#[from] EnsurePolicyError),
    #[error(transparent)]
    State(#[from] EnsureStateError),
    #[error(transparent)]
    Inspection(#[from] CapacityImportJournalError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Bind original observations without rebasing them during apply or restart.
pub fn seal_sources(
    mut record: InfrastructureBootstrapRecord,
) -> Result<InfrastructureBootstrapRecord, InfrastructureBootstrapError> {
    verify_declarations(&record)?;
    record.source_sha256 = source_digest(&record)?;
    Ok(record)
}

fn verify_declarations(
    record: &InfrastructureBootstrapRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    let digest: [u8; 32] = Sha256::digest(record.declarations_toml.as_bytes()).into();
    if record.declarations_toml.len() > 256 * 1024 || digest != record.declarations_sha256 {
        return Err(invalid());
    }
    let declarations: declarations::BootstrapDeclarations =
        toml::from_str(&record.declarations_toml).map_err(|_| invalid())?;
    if declarations.schema_version != 1
        || Principal::from_text(&declarations.operator).ok() != Some(record.operator)
        || declarations.network_root_key_sha256 != hex_bytes(record.network_root_key_sha256)
        || declarations.canisters.len() != record.sources.len()
        || declarations.root_owned.len() != record.held_sources.len()
    {
        return Err(invalid());
    }
    let mut seen = BTreeSet::new();
    for declaration in &declarations.canisters {
        let binding = declaration.binding().map_err(|_| invalid())?;
        if !seen.insert(binding.canister_id)
            || !declaration.no_external_obligations
            || !declaration.no_other_fleet_ownership
            || declaration.evidence.trim().is_empty()
        {
            return Err(invalid());
        }
        let source = record
            .sources
            .values()
            .find(|source| source.sample.binding.canister_id == binding.canister_id)
            .ok_or_else(invalid)?;
        let mut expected = source.sample.binding.clone();
        expected.stopped = true;
        expected.snapshots_size_bytes = 0;
        if expected != binding || source.disposition != declaration.disposition(digest) {
            return Err(invalid());
        }
    }
    for declaration in &declarations.root_owned {
        let binding = declaration.binding()?;
        if !seen.insert(binding.canister) {
            return Err(invalid());
        }
        let source = record
            .held_sources
            .values()
            .find(|source| source.custody.canister == binding.canister)
            .ok_or_else(invalid)?;
        if source.custody != binding
            || source.disposition != declaration.disposition(digest)
            || !binding.controllers.contains(&source.root)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn source_digest(
    record: &InfrastructureBootstrapRecord,
) -> Result<[u8; 32], InfrastructureBootstrapError> {
    let mut projection = record.clone();
    projection.source_sha256 = [0; 32];
    let mut hash = Sha256::new();
    hash.update(b"canic.infrastructure-bootstrap.sources.v1\0");
    hash.update(serde_json::to_vec(&projection)?);
    Ok(hash.finalize().into())
}

/// Produce exact install/funding/controller effects from current artifacts and sealed source custody.
pub fn prepare(
    root: &Path,
    desired: &DesiredFleet,
    record: &InfrastructureBootstrapRecord,
    desired_sha256: &str,
    time: u64,
) -> Result<FleetEnsurePlan, InfrastructureBootstrapError> {
    if source_digest(record)? != record.source_sha256 {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    verify_declarations(record)?;
    publication::validate_seed(desired, record)?;
    verify_ready_coordinator(root, desired, record)?;
    let observation = original_observation(desired, record);
    let plan = infrastructure_bootstrap::compile(
        desired,
        &resolve_desired_artifacts(root, desired)?,
        record,
        &observation,
        desired_sha256,
        time,
    )?;
    publication::validate_capacity(&plan)?;
    Ok(plan)
}

/// Reject altered phase shape before the generic Ensure driver can issue an effect.
pub(in crate::fleet_ensure) fn verify_plan(
    root: &Path,
    plan: &FleetEnsurePlan,
) -> Result<(), InfrastructureBootstrapError> {
    let record = plan
        .infrastructure_bootstrap
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?
        .desired();
    if plan.scope != FleetEnsurePlanScope::InfrastructureBootstrap
        || prepare(
            root,
            desired,
            record,
            &plan.desired_sha256,
            plan.planned_at_time,
        )? != *plan
    {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(())
}

/// Original samples become the reviewed numerical input; no management call is issued here.
pub(in crate::fleet_ensure) fn original_observation(
    desired: &DesiredFleet,
    record: &InfrastructureBootstrapRecord,
) -> FleetObservation {
    let view = InfrastructureBootstrapObservation {
        held_sources: record
            .held_sources
            .iter()
            .map(|(name, source)| (name.clone(), source.custody.clone()))
            .collect(),
        canisters: desired
            .canisters
            .iter()
            .map(|configured| {
                (
                    configured.name.clone(),
                    record
                        .sources
                        .get(&configured.name)
                        .map(|source| source.sample.clone()),
                )
            })
            .collect(),
        coordinator_registry: None,
        operator_cycles: record.operator_cycles,
        ledger_fee_cycles: record.ledger_fee_cycles,
    };
    fleet_observation(&view)
}

pub(in crate::fleet_ensure) fn fleet_observation(
    view: &InfrastructureBootstrapObservation,
) -> FleetObservation {
    FleetObservation {
        additional_controlled_cycles: BTreeMap::new(),
        estate_funding_domains: BTreeMap::new(),
        protocol_ready: BTreeMap::new(),
        operator_cycles: view.operator_cycles,
        ledger_fee_cycles: view.ledger_fee_cycles,
        canisters: view
            .canisters
            .iter()
            .map(|(name, sample)| {
                (
                    name.clone(),
                    sample.as_ref().map(|sample| LiveCanister {
                        canister_version: Some(sample.binding.canister_version),
                        controllers: sample
                            .binding
                            .controllers
                            .iter()
                            .map(Principal::to_text)
                            .collect(),
                        cycles: sample.cycles,
                        module_sha256: sample.binding.module_sha256.map(hex_bytes),
                        principal: sample.binding.canister_id.to_text(),
                        reinstall_required: false,
                        root_owned_lifecycle: None,
                        status: if sample.binding.stopped {
                            CanisterRuntimeStatus::Stopped
                        } else {
                            CanisterRuntimeStatus::Running
                        },
                    }),
                )
            })
            .collect(),
    }
}

fn verify_ready_coordinator(
    root: &Path,
    desired: &DesiredFleet,
    record: &InfrastructureBootstrapRecord,
) -> Result<(), InfrastructureBootstrapError> {
    if record.coordinator != BootstrapCoordinatorSelection::Ready {
        return Ok(());
    }
    let encoded = record
        .coordinator_registry_candid_hex
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let bytes = decode_hex(encoded).map_err(|_| InfrastructureBootstrapError::Integrity)?;
    let observed: FleetRegistry =
        candid::decode_one(&bytes).map_err(|_| InfrastructureBootstrapError::Integrity)?;
    let principals = desired
        .canisters
        .iter()
        .filter_map(|configured| {
            configured
                .principal
                .as_ref()
                .map(|id| (configured.name.clone(), id.clone()))
        })
        .collect();
    if observed != expected_registry(root, desired, &principals)? {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(())
}

fn expected_registry(
    root: &Path,
    desired: &DesiredFleet,
    principals: &BTreeMap<String, String>,
) -> Result<FleetRegistry, InfrastructureBootstrapError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let authorities = canic_init::compile_root_authorities(root, desired, principals)
        .map_err(|_| InfrastructureBootstrapError::Integrity)?;
    let authority = authorities
        .first()
        .ok_or(InfrastructureBootstrapError::Integrity)?
        .1
        .binding
        .authority
        .clone();
    let admission =
        bind_initial_fleet_admission_policy(authority.binding.fleet.clone(), &bootstrap.admission)
            .map_err(|_| InfrastructureBootstrapError::Integrity)?;
    FleetRegistryOps::compile_genesis(
        &bootstrap.app,
        authority,
        &bootstrap
            .component_deployment_configuration
            .component_topology,
        admission,
    )
    .map_err(|_| InfrastructureBootstrapError::Integrity)
}

/// Require exact current Coordinator genesis before dependent Root or Store effects.
pub(in crate::fleet_ensure::ops) fn verify_coordinator(
    icp: &IcpCli,
    root: &Path,
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
    state: &FleetEnsureStateRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let principals = desired
        .canisters
        .iter()
        .map(|configured| {
            configured
                .principal
                .as_ref()
                .or_else(|| state.pending_principals.get(&configured.name))
                .or_else(|| state.principals.get(&configured.name))
                .map(|id| (configured.name.clone(), id.clone()))
                .ok_or(InfrastructureBootstrapError::Integrity)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    verify_registry(
        icp,
        desired,
        source,
        &expected_registry(root, desired, &principals)?,
    )
}

/// Check the exact current Registry against authenticated operator/network selection.
pub(in crate::fleet_ensure::ops) fn verify_registry(
    icp: &IcpCli,
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
    expected: &FleetRegistry,
) -> Result<(), InfrastructureBootstrapError> {
    let agent = authenticated_agent(icp, desired, source)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let observed = runtime.block_on(inventory::registry(
        &agent,
        expected.authority.binding.coordinator,
    ))?;
    if observed != *expected {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(())
}

#[derive(CandidType)]
struct LedgerAccount {
    owner: Principal,
    subaccount: Option<Vec<u8>>,
}

/// Read exact management custody; the caller has reserved the phase's observation allowance.
pub(in crate::fleet_ensure::ops) fn observe(
    icp: &IcpCli,
    desired: &DesiredFleet,
    record: &InfrastructureBootstrapRecord,
    state: &FleetEnsureStateRecord,
) -> Result<InfrastructureBootstrapObservation, InfrastructureBootstrapError> {
    let agent = authenticated_agent(icp, desired, record)?;
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| InfrastructureBootstrapError::Observation(error.to_string()))?;
    let mut canisters = BTreeMap::new();
    let mut held_sources = BTreeMap::new();
    for configured in &desired.canisters {
        if let Some(source) = record.held_sources.get(&configured.name) {
            let custody = runtime.block_on(declarations::observe_held(&agent, source))?;
            held_sources.insert(configured.name.clone(), custody);
            canisters.insert(configured.name.clone(), None);
            continue;
        }
        let id = configured
            .principal
            .as_ref()
            .or_else(|| state.pending_principals.get(&configured.name))
            .or_else(|| state.principals.get(&configured.name));
        let sample = match id {
            Some(id) => Some(runtime.block_on(management::observe(
                &agent,
                Principal::from_text(id).map_err(|_| InfrastructureBootstrapError::Integrity)?,
            ))?),
            None if configured.name == bootstrap.coordinator
                && record.coordinator == BootstrapCoordinatorSelection::Create =>
            {
                None
            }
            None => return Err(InfrastructureBootstrapError::Integrity),
        };
        canisters.insert(configured.name.clone(), sample);
    }
    let coordinator_registry = if record.coordinator == BootstrapCoordinatorSelection::Ready {
        let id = canisters
            .get(&bootstrap.coordinator)
            .and_then(Option::as_ref)
            .ok_or(InfrastructureBootstrapError::Integrity)?
            .binding
            .canister_id;
        Some(runtime.block_on(inventory::registry(&agent, id))?)
    } else {
        None
    };
    let fee: Nat = icp
        .canister_query_candid(&desired.cycles_ledger, "icrc1_fee", &(), None)
        .map_err(|error| InfrastructureBootstrapError::Observation(error.to_string()))?;
    let balance: Nat = icp
        .canister_query_candid(
            &desired.cycles_ledger,
            "icrc1_balance_of",
            &LedgerAccount {
                owner: record.operator,
                subaccount: None,
            },
            None,
        )
        .map_err(|error| InfrastructureBootstrapError::Observation(error.to_string()))?;
    Ok(InfrastructureBootstrapObservation {
        held_sources,
        canisters,
        coordinator_registry,
        operator_cycles: balance
            .0
            .try_into()
            .map_err(|_| InfrastructureBootstrapError::Integrity)?,
        ledger_fee_cycles: fee
            .0
            .try_into()
            .map_err(|_| InfrastructureBootstrapError::Integrity)?,
    })
}

fn authenticated_agent(
    icp: &IcpCli,
    desired: &DesiredFleet,
    record: &InfrastructureBootstrapRecord,
) -> Result<ic_agent::Agent, InfrastructureBootstrapError> {
    let agent = icp
        .authenticated_agent_with_response_limit(2 * 1024 * 1024)
        .map_err(|error| InfrastructureBootstrapError::Observation(error.to_string()))?;
    let root_key = agent.read_root_key();
    let key_hash: [u8; 32] = Sha256::digest(&root_key).into();
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    if agent.get_principal().ok() != Some(record.operator)
        || key_hash != record.network_root_key_sha256
        || CanonicalNetworkId::from_der_root_trust_anchor(&root_key).ok()
            != Some(bootstrap.canonical_network_id)
    {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(agent)
}

/// Fresh pre-effect evidence must preserve every original binding and bounded cycle baseline.
pub(in crate::fleet_ensure) fn verify_initial(
    plan: &FleetEnsurePlan,
    observed: &InfrastructureBootstrapObservation,
) -> Result<(), InfrastructureBootstrapError> {
    let source = plan
        .infrastructure_bootstrap
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?
        .desired();
    let allowance = desired
        .maximum_observation_burn_cycles
        .parse::<canic_core::cdk::types::Cycles>()
        .map_err(|_| InfrastructureBootstrapError::Integrity)?
        .to_u128();
    let held = source
        .held_sources
        .iter()
        .map(|(name, source)| (name.clone(), source.custody.clone()))
        .collect::<BTreeMap<_, _>>();
    if observed.held_sources != held
        || observed.canisters.len() != desired.canisters.len()
        || observed.operator_cycles < source.operator_cycles
        || observed.ledger_fee_cycles != source.ledger_fee_cycles
    {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    for configured in &desired.canisters {
        let actual = observed
            .canisters
            .get(&configured.name)
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        match (source.sources.get(&configured.name), actual) {
            (None, None)
                if configured.kind == crate::fleet_ensure::model::DesiredCanisterKind::Pool
                    && source.held_sources.contains_key(&configured.name) => {}
            (None, None)
                if configured.kind
                    == crate::fleet_ensure::model::DesiredCanisterKind::Coordinator
                    && source.coordinator == BootstrapCoordinatorSelection::Create => {}
            (Some(original), Some(actual)) => {
                let before = original
                    .sample
                    .cycles
                    .checked_add(original.sample.reserved_cycles)
                    .ok_or(InfrastructureBootstrapError::Integrity)?;
                let after = actual
                    .cycles
                    .checked_add(actual.reserved_cycles)
                    .ok_or(InfrastructureBootstrapError::Integrity)?;
                let mut expected_binding = original.sample.binding.clone();
                if !expected_binding.stopped
                    && actual.binding.canister_version >= expected_binding.canister_version
                {
                    expected_binding.canister_version = actual.binding.canister_version;
                }
                if expected_binding != actual.binding
                    || before
                        .checked_sub(after)
                        .is_none_or(|debit| debit > allowance)
                {
                    return Err(InfrastructureBootstrapError::Integrity);
                }
            }
            _ => return Err(InfrastructureBootstrapError::Integrity),
        }
    }
    let actual_registry = observed
        .coordinator_registry
        .as_ref()
        .map(candid::encode_one)
        .transpose()
        .map_err(|_| InfrastructureBootstrapError::Integrity)?
        .map(hex_bytes);
    if source.coordinator_registry_candid_hex != actual_registry {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(())
}

/// Project only initialized infrastructure into terminal accounting after exact custody checks.
pub(in crate::fleet_ensure) fn terminal_observation(
    workspace: &Path,
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    observed: &InfrastructureBootstrapObservation,
) -> Result<(FleetObservation, u128), InfrastructureBootstrapError> {
    let source = plan
        .infrastructure_bootstrap
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let held = source
        .held_sources
        .iter()
        .map(|(name, source)| (name.clone(), source.custody.clone()))
        .collect::<BTreeMap<_, _>>();
    if observed.held_sources != held {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?
        .desired();
    let artifacts = resolve_desired_artifacts(workspace, desired)?;
    let principals = desired
        .canisters
        .iter()
        .filter_map(|configured| {
            configured
                .principal
                .as_ref()
                .or_else(|| state.pending_principals.get(&configured.name))
                .or_else(|| state.principals.get(&configured.name))
                .map(|id| (configured.name.clone(), id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut terminal = fleet_observation(observed);
    terminal
        .canisters
        .retain(|name, _| plan.canisters.iter().any(|canister| canister.name == *name));
    if terminal.canisters.len() != plan.canisters.len() {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    let mut total = 0_u128;
    for target in &plan.canisters {
        let configured = desired
            .canisters
            .iter()
            .find(|candidate| candidate.name == target.name)
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let actual = observed
            .canisters
            .get(&target.name)
            .and_then(Option::as_ref)
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let named_controllers = configured
            .controller_canisters
            .iter()
            .map(|name| {
                principals
                    .get(name)
                    .cloned()
                    .ok_or(InfrastructureBootstrapError::Integrity)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut controllers = configured
            .controllers
            .iter()
            .cloned()
            .chain(named_controllers)
            .map(|id| Principal::from_text(id).map_err(|_| InfrastructureBootstrapError::Integrity))
            .collect::<Result<Vec<_>, _>>()?;
        controllers.sort_unstable();
        controllers.dedup();
        let minimum = configured
            .minimum_cycles
            .parse::<canic_core::cdk::types::Cycles>()
            .map_err(|_| InfrastructureBootstrapError::Integrity)?
            .to_u128();
        if principals.get(&target.name) != Some(&actual.binding.canister_id.to_text())
            || configured.subnet != actual.binding.subnet.to_string()
            || actual.binding.controllers != controllers
            || actual.binding.stopped
            || actual.binding.snapshots_size_bytes != 0
            || actual.cycles < minimum
            || artifacts.wasm_sha256_by_canister.get(&target.name)
                != actual.binding.module_sha256.map(hex_bytes).as_ref()
        {
            return Err(InfrastructureBootstrapError::Integrity);
        }
        total = total
            .checked_add(actual.cycles)
            .and_then(|n| n.checked_add(actual.reserved_cycles))
            .ok_or(InfrastructureBootstrapError::Integrity)?;
    }
    Ok((terminal, total))
}
