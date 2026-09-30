//! Compile fresh reset authority from completed physical evidence and current configuration.
//!
//! The existing Fleet journal owns every remote effect; no predecessor desired state is synthesized.

pub(in crate::fleet_ensure) mod terminal;
#[cfg(test)]
pub(in crate::fleet_ensure) mod tests;

use crate::fleet_ensure::{
    CompletedEstateMembershipView, CompletedSourceInspectionView,
    model::{
        DesiredCanisterKind, DesiredFleet, EstateFundingDomainObservation, FleetEnsureCompletion,
        FleetEnsureJournalRecord, FleetEnsureStateRecord, FleetObservation,
        FleetReinstallAssetRecord, FleetReinstallRecord, LiveCanister, RootManagementBinding,
        RootOwnedCanisterLifecycle,
        completed_handoff::{
            CompletedEstateResetRecord,
            preparation::{CompletedPreparationJournalRecord, CompletedPreparationReviewRecord},
        },
    },
    ops::{self, EnsurePaths, EnsureStateError, completed_preparation as preparation},
    policy::{self, EnsurePolicyError},
    view::completed_reset::CompletedResetTargetView,
};
use canic_core::{
    cdk::{types::Cycles, utils::hash::sha256_hex},
    ids::FleetBinding,
};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Failure to bind the selected release to one exact prepared physical estate.
#[derive(Debug, Error)]
pub enum CompletedResetError {
    #[error(
        "completed reset requires unchanged prepared custody, complete current identities and typed initialization"
    )]
    Conflict,
    #[error(transparent)]
    Preparation(#[from] Box<preparation::CompletedPreparationError>),
    #[error(transparent)]
    State(#[from] EnsureStateError),
    #[error(transparent)]
    Policy(#[from] EnsurePolicyError),
    #[error("current reset initialization failed: {0}")]
    Init(#[from] Box<ops::canic_init::CanicInitError>),
    #[error("completed reset observation failed: {0}")]
    Observation(#[from] Box<ops::retained_contract::CompletedMembershipError>),
    #[error("completed reset source failed: {0}")]
    Source(#[from] Box<ops::retained_contract::RetainedContractError>),
    #[error("completed reset runtime failed: {0}")]
    Runtime(#[from] std::io::Error),
}

pub(in crate::fleet_ensure) fn compile(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    desired_sha256: &str,
    time: u64,
    source: &CompletedSourceInspectionView,
    observed: &CompletedEstateMembershipView,
) -> Result<CompletedResetTargetView, CompletedResetError> {
    let review = preparation::review(paths)
        .map_err(Box::new)?
        .ok_or(CompletedResetError::Conflict)?;
    let prepared = preparation::journal(paths, &review)
        .map_err(Box::new)?
        .filter(|journal| journal.prepared)
        .ok_or(CompletedResetError::Conflict)?;
    preparation::revalidate(paths, &review, source, observed).map_err(Box::new)?;
    let completed = capture(desired, source, observed, review, prepared)?;
    let observation = observation(desired, &completed)?;
    let state = state(desired)?;
    let operation = sha256_hex(
        format!(
            "canic:completed-reset:v1:{desired_sha256}:{}:{time}",
            completed.preparation.review_sha256
        )
        .as_bytes(),
    );
    let mut authorities = Vec::new();
    let mut assets = Vec::new();
    for configured in &desired.canisters {
        let source_name = &completed.source_names[&configured.name];
        let binding = &completed.preparation.custody.canisters[source_name].binding;
        if configured.kind == DesiredCanisterKind::Pool {
            assets.push(FleetReinstallAssetRecord {
                controllers: controllers(&binding.controllers),
                module_sha256: binding.module_sha256.clone(),
                principal: binding.principal.to_text(),
                root: configured
                    .parent
                    .clone()
                    .ok_or(CompletedResetError::Conflict)?,
                subnet: binding.subnet.to_string(),
            });
        } else {
            authorities.push(RootManagementBinding {
                controllers: controllers(&binding.controllers),
                module_sha256: binding
                    .module_sha256
                    .clone()
                    .ok_or(CompletedResetError::Conflict)?,
                name: configured.name.clone(),
                principal: binding.principal.to_text(),
                subnet: binding.subnet.to_string(),
            });
        }
    }
    let intent = FleetReinstallRecord {
        target_artifacts_sha256: Some(ops::reinstall::target_artifacts_sha256(
            &paths.workspace,
            desired,
        )?),
        source: None,
        activation_reset: None,
        operation_id: operation.clone(),
        source_operation_id: completed.preparation.source.operation_id.clone(),
        completed_reset: Some(Box::new(completed)),
        authorities,
        assets,
    };
    let artifacts = ops::resolve_desired_artifacts(&paths.workspace, desired)?;
    let plan = policy::compile_plan(
        desired,
        &artifacts,
        &[],
        desired_sha256,
        &desired.fleet,
        &observation,
        time,
        &operation,
        Some(&intent),
    )?;
    if plan.continuation.is_none() {
        return Err(CompletedResetError::Conflict);
    }
    validate_initialization(paths, desired, &operation, &state, &artifacts)?;
    let journal = initial_journal(&plan, &intent, &observation)?;
    Ok(CompletedResetTargetView {
        plan,
        journal,
        state,
    })
}

fn initial_journal(
    plan: &crate::fleet_ensure::model::FleetEnsurePlan,
    intent: &FleetReinstallRecord,
    observation: &FleetObservation,
) -> Result<FleetEnsureJournalRecord, CompletedResetError> {
    Ok(FleetEnsureJournalRecord {
        bootstrap_registration_recovery: None,
        funding_observations: BTreeMap::new(),
        funding_reviews: Vec::new(),
        successor_phases: Vec::new(),
        completion: FleetEnsureCompletion::InProgress,
        estate_funding_required: None,
        effects: Vec::new(),
        fleet: plan.fleet.clone(),
        initial_controlled_cycles: plan.conservation.observed_controlled_cycles,
        initial_estate_funding_cycles_by_root: intent
            .completed_reset
            .as_ref()
            .ok_or(CompletedResetError::Conflict)?
            .root_ledger_cycles
            .clone(),
        initial_operator_cycles: observation.operator_cycles,
        operation_id: plan.operation_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        schema_version: 1,
        stalled_observations: 0,
    })
}

fn validate_initialization(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    operation: &str,
    state: &FleetEnsureStateRecord,
    artifacts: &crate::fleet_ensure::model::DesiredFleetArtifacts,
) -> Result<(), CompletedResetError> {
    for canister in desired
        .canisters
        .iter()
        .filter(|canister| canister.kind != DesiredCanisterKind::Pool)
    {
        ops::canic_init::compile_arguments(&ops::canic_init::CanicInitRequest {
            desired,
            init: canister
                .canic_init
                .as_ref()
                .ok_or(CompletedResetError::Conflict)?,
            operation_id: operation,
            principals: &state.principals,
            root: &paths.workspace,
            wasm: canister
                .wasm
                .as_deref()
                .ok_or(CompletedResetError::Conflict)?,
            wasm_sha256: artifacts
                .wasm_sha256_by_canister
                .get(&canister.name)
                .ok_or(CompletedResetError::Conflict)?,
        })
        .map_err(Box::new)?;
    }
    Ok(())
}

fn capture(
    desired: &DesiredFleet,
    source: &CompletedSourceInspectionView,
    observed: &CompletedEstateMembershipView,
    preparation: CompletedPreparationReviewRecord,
    prepared: CompletedPreparationJournalRecord,
) -> Result<CompletedEstateResetRecord, CompletedResetError> {
    validate_identity(desired, source, &preparation)?;
    let mut source_names = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut root_ledger_cycles = BTreeMap::new();
    let mut other_ledger_cycles = BTreeMap::new();
    for canister in &desired.canisters {
        let principal = canister
            .principal
            .as_deref()
            .ok_or(CompletedResetError::Conflict)?;
        let (name, old) = source
            .inventory
            .canisters
            .iter()
            .find(|(_, old)| old.principal.to_text() == principal)
            .ok_or(CompletedResetError::Conflict)?;
        let role = if canister.kind == DesiredCanisterKind::Pool {
            matches!(
                old.kind,
                DesiredCanisterKind::Pool | DesiredCanisterKind::Component
            )
        } else {
            canister.kind == old.kind
        };
        if !role
            || canister.subnet != old.subnet.to_string()
            || !seen.insert(principal)
            || (canister.kind != DesiredCanisterKind::Pool
                && !preparation.custody.canisters[name]
                    .binding
                    .controllers
                    .contains(&preparation.custody.operator))
        {
            return Err(CompletedResetError::Conflict);
        }
        if matches!(
            canister.kind,
            DesiredCanisterKind::Pool | DesiredCanisterKind::Store
        ) {
            let root = canister
                .parent
                .as_ref()
                .and_then(|parent| desired.canisters.iter().find(|c| &c.name == parent))
                .and_then(|c| c.principal.as_deref())
                .ok_or(CompletedResetError::Conflict)?;
            if old
                .root
                .as_ref()
                .and_then(|name| source.inventory.canisters.get(name))
                .map(|c| c.principal.to_text())
                .as_deref()
                != Some(root)
            {
                return Err(CompletedResetError::Conflict);
            }
        }
        if canister.kind == DesiredCanisterKind::Root {
            root_ledger_cycles.insert(
                canister.name.clone(),
                observed
                    .ledger()
                    .root_accounts()
                    .get(name)
                    .ok_or(CompletedResetError::Conflict)?
                    .cycles,
            );
        } else {
            other_ledger_cycles.insert(
                principal.to_string(),
                observed
                    .ledger()
                    .other_canister_accounts()
                    .get(name)
                    .ok_or(CompletedResetError::Conflict)?
                    .cycles,
            );
        }
        source_names.insert(canister.name.clone(), name.clone());
    }
    if seen.len() != source.inventory.canisters.len() {
        return Err(CompletedResetError::Conflict);
    }
    Ok(CompletedEstateResetRecord {
        maximum_terminal_observations: crate::fleet_ensure::model::completed_handoff::COMPLETED_RESET_MAXIMUM_TERMINAL_OBSERVATIONS,
        preparation,
        prepared,
        source_names,
        operator_cycles: observed.ledger().operator().cycles,
        root_ledger_cycles,
        other_ledger_cycles,
    })
}

fn validate_identity(
    desired: &DesiredFleet,
    source: &CompletedSourceInspectionView,
    preparation: &CompletedPreparationReviewRecord,
) -> Result<(), CompletedResetError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(CompletedResetError::Conflict)?;
    let fleet = FleetBinding {
        app: bootstrap.app.clone(),
        fleet: canic_core::ids::FleetKey {
            canonical_network_id: bootstrap.canonical_network_id,
            fleet_id: bootstrap.fleet_id,
        },
    };
    if fleet != source.inventory.fleet
        || desired.operator != source.inventory.receipts.source_operator
        || desired.cycles_ledger != source.inventory.receipts.cycles_ledger
        || bootstrap.fresh_estate
        || desired.environment != preparation.environment
        || desired.fleet != preparation.fleet
    {
        return Err(CompletedResetError::Conflict);
    }
    Ok(())
}

pub(in crate::fleet_ensure) fn observation(
    desired: &DesiredFleet,
    completed: &CompletedEstateResetRecord,
) -> Result<FleetObservation, CompletedResetError> {
    let mut canisters = BTreeMap::new();
    let mut estate_funding_domains = BTreeMap::new();
    for configured in &desired.canisters {
        let name = completed
            .source_names
            .get(&configured.name)
            .ok_or(CompletedResetError::Conflict)?;
        let binding = &completed
            .preparation
            .custody
            .canisters
            .get(name)
            .ok_or(CompletedResetError::Conflict)?
            .binding;
        let balance = completed
            .prepared
            .inspections
            .get(name)
            .and_then(|record| record.balance)
            .ok_or(CompletedResetError::Conflict)?;
        canisters.insert(
            configured.name.clone(),
            Some(LiveCanister {
                canister_version: None,
                controllers: controllers(&binding.controllers),
                cycles: balance.native_cycles,
                module_sha256: binding.module_sha256.clone(),
                principal: binding.principal.to_text(),
                reinstall_required: false,
                status: balance.status,
                // A complete source survey admits only idle or allocated terminal assets.
                root_owned_lifecycle: (configured.kind == DesiredCanisterKind::Pool).then_some(
                    if binding.module_sha256.is_some() {
                        RootOwnedCanisterLifecycle::Workload
                    } else {
                        RootOwnedCanisterLifecycle::Idle
                    },
                ),
            }),
        );
        if configured.kind == DesiredCanisterKind::Root {
            estate_funding_domains.insert(
                configured.name.clone(),
                EstateFundingDomainObservation {
                    balance_cycles: Some(
                        *completed
                            .root_ledger_cycles
                            .get(&configured.name)
                            .ok_or(CompletedResetError::Conflict)?,
                    ),
                    cycles_ledger: desired.cycles_ledger.clone(),
                    pool: None,
                    root_principal: Some(binding.principal.to_text()),
                },
            );
        }
    }
    Ok(FleetObservation {
        canisters,
        estate_funding_domains,
        additional_controlled_cycles: BTreeMap::new(),
        operator_cycles: completed.operator_cycles,
        ledger_fee_cycles: desired
            .ledger_fee_cycles
            .parse::<Cycles>()
            .map_err(|_| CompletedResetError::Conflict)?
            .to_u128(),
        protocol_ready: BTreeMap::new(),
    })
}

fn state(desired: &DesiredFleet) -> Result<FleetEnsureStateRecord, CompletedResetError> {
    Ok(FleetEnsureStateRecord {
        active_registry: None,
        completed_reinstall_action_sha256: BTreeMap::new(),
        completed_reinstall_operation_id: None,
        completed_reinstalls: BTreeMap::new(),
        fleet: desired.fleet.clone(),
        pending_principals: BTreeMap::new(),
        principals: desired
            .canisters
            .iter()
            .map(|c| {
                Ok((
                    c.name.clone(),
                    c.principal.clone().ok_or(CompletedResetError::Conflict)?,
                ))
            })
            .collect::<Result<_, CompletedResetError>>()?,
        retained_cycles_by_principal: BTreeMap::new(),
        schema_version: 1,
        topology: BTreeMap::new(),
    })
}

fn controllers(principals: &[candid::Principal]) -> Vec<String> {
    let mut values = principals
        .iter()
        .map(candid::Principal::to_text)
        .collect::<Vec<_>>();
    values.sort();
    values
}

pub(in crate::fleet_ensure) fn terminal_balance(
    status: canic_core::dto::canister::CanisterStatusResponse,
) -> Option<
    crate::fleet_ensure::model::completed_handoff::preparation::CompletedPreparationBalanceRecord,
> {
    Some(crate::fleet_ensure::model::completed_handoff::preparation::CompletedPreparationBalanceRecord {
        status: preparation::runtime_status(status.status),
        native_cycles: u128::try_from(status.cycles.0).ok()?,
        reserved_cycles: u128::try_from(status.reserved_cycles.0).ok()?,
    })
}
