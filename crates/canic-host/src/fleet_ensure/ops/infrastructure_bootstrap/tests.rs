//! Exercise initialization review against the generated current release fixture.

use super::*;
use crate::fleet_ensure::{
    model::{
        DesiredCanisterKind, EnsureAction, InstallMode,
        capacity_import::{CapacityImportSourceBinding, survey::CapacityImportSampleRecord},
        infrastructure_bootstrap::InfrastructureBootstrapSourceRecord,
    },
    ops::capacity_import::admission::{CapacityImportDeclaration, CapacityImportDispositionKind},
};
use canic_core::{cdk::types::Cycles, ids::SubnetId};

pub(in crate::fleet_ensure::ops) fn qualify_initialization(root: &Path, desired: &DesiredFleet) {
    let mut desired = desired.clone();
    desired.maximum_observation_burn_cycles = "1000000000".into();
    let mut source = source_record(&desired);
    reseal(&mut desired, &mut source);
    let plan = prepare(root, &desired, &source, "supplied-infrastructure", 43).unwrap();
    verify_plan(root, &plan).unwrap();
    assert_eq!(plan.scope, FleetEnsurePlanScope::InfrastructureBootstrap);
    assert!(plan.protocol_actions.is_empty());
    assert_eq!(plan.canisters[0].name, "coordinator");
    assert_eq!(plan.canisters[1].name, "root-0");
    assert_eq!(plan.canisters[2].name, "store-0");
    assert!(
        plan.canisters
            .iter()
            .all(|canister| !canister.name.contains("pool"))
    );
    assert!(
        plan.canisters
            .iter()
            .flat_map(|target| &target.actions)
            .all(|action| !matches!(action, EnsureAction::Create { .. }))
    );
    for target in &plan.canisters {
        assert!(matches!(
            target.actions.first(),
            Some(EnsureAction::Stop { .. })
        ));
        let install = target
            .actions
            .iter()
            .find(|action| matches!(action, EnsureAction::Install { .. }))
            .unwrap();
        let EnsureAction::Install { mode, .. } = install else {
            unreachable!()
        };
        assert_eq!(*mode, InstallMode::Install);
        assert_eq!(
            target
                .actions
                .iter()
                .any(|action| matches!(action, EnsureAction::Uninstall { .. })),
            target.name != "root-0"
        );
    }
    let expected = plan
        .canisters
        .iter()
        .map(|target| {
            let sample = &source.sources[&target.name].sample;
            sample.cycles + sample.reserved_cycles
        })
        .sum::<u128>();
    assert_eq!(plan.conservation.observed_controlled_cycles, expected);
    let encoded = serde_json::to_vec(&plan).unwrap();
    let restored: FleetEnsurePlan = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, plan);
    verify_plan(root, &restored).unwrap();

    qualify_survey_identity(root, &desired, &source);
    qualify_declarations(&source);
    qualify_initial_observation(&plan, &source);
    qualify_inspection_budget(root, &plan);
    qualify_creation(root, &desired, &source);
    qualify_funding_order(root, &desired, &source);
    qualify_ready_coordinator(root, &desired, &source);
    qualify_terminal_publication(root, &plan, &source);

    let mut missing = desired.clone();
    missing
        .canisters
        .iter_mut()
        .find(|canister| canister.name == "root-0")
        .unwrap()
        .principal = None;
    assert!(prepare(root, &missing, &source, "missing-root", 43).is_err());
    let mut foreign = source.clone();
    foreign
        .sources
        .get_mut("root-0")
        .unwrap()
        .sample
        .binding
        .subnet = SubnetId::from_principal(Principal::from_slice(&[99]));
    reseal(&mut desired, &mut foreign);
    assert!(matches!(
        prepare(root, &desired, &foreign, "foreign-root", 43),
        Err(InfrastructureBootstrapError::Policy(
            EnsurePolicyError::InfrastructureBootstrap(_)
        ))
    ));
}

fn source_record(desired: &DesiredFleet) -> InfrastructureBootstrapRecord {
    let operator = Principal::from_text(&desired.operator).unwrap();
    let sources = desired.canisters.iter().map(|configured| {
        let sample = CapacityImportSampleRecord {
            binding: CapacityImportSourceBinding {
                canister_id: Principal::from_text(configured.principal.as_ref().unwrap()).unwrap(),
                subnet: SubnetId::from_principal(Principal::from_text(&configured.subnet).unwrap()),
                controllers: vec![operator],
                module_sha256: (configured.kind != DesiredCanisterKind::Root).then_some([38; 32]),
                canister_version: 7,
                stopped: configured.kind == DesiredCanisterKind::Pool,
                snapshots_size_bytes: 0,
            },
            cycles: 300_000_000_000_000,
            reserved_cycles: 1_000_000_000_000,
        };
        (configured.name.clone(), InfrastructureBootstrapSourceRecord {
            sample, disposition: crate::fleet_ensure::model::capacity_import::CapacityImportDisposition::AbsenceEvidence { evidence_sha256: [0; 32] },
        })
    }).collect();
    InfrastructureBootstrapRecord {
        schema_version: 1,
        operator,
        network_root_key_sha256: [43; 32],
        coordinator: BootstrapCoordinatorSelection::Initialize,
        estate_seed: None,
        coordinator_registry_candid_hex: None,
        declarations_toml: String::new(),
        declarations_sha256: [0; 32],
        sources,
        operator_cycles: 1_000_000_000_000_000,
        ledger_fee_cycles: desired
            .ledger_fee_cycles
            .parse::<Cycles>()
            .unwrap()
            .to_u128(),
        source_sha256: [0; 32],
    }
}

fn reseal(desired: &mut DesiredFleet, source: &mut InfrastructureBootstrapRecord) {
    let document = CapacityImportDeclarations {
        schema_version: 1,
        operator: source.operator.to_text(),
        network_root_key_sha256: hex_bytes(source.network_root_key_sha256),
        canisters: source
            .sources
            .values()
            .map(|source| {
                let binding = &source.sample.binding;
                CapacityImportDeclaration {
                    canister: binding.canister_id.to_text(),
                    subnet: binding.subnet.to_string(),
                    controllers: binding.controllers.iter().map(Principal::to_text).collect(),
                    module_sha256: binding
                        .module_sha256
                        .map_or_else(|| "empty".to_string(), hex_bytes),
                    canister_version: binding.canister_version,
                    disposition: CapacityImportDispositionKind::Absence,
                    no_external_obligations: true,
                    no_other_fleet_ownership: true,
                    evidence: "operator-owned staging capacity with no outstanding obligations"
                        .to_string(),
                }
            })
            .collect(),
    };
    source.declarations_toml = toml::to_string(&document).unwrap();
    source.declarations_sha256 = Sha256::digest(source.declarations_toml.as_bytes()).into();
    for entry in source.sources.values_mut() {
        entry.disposition = crate::fleet_ensure::model::capacity_import::CapacityImportDisposition::AbsenceEvidence { evidence_sha256: source.declarations_sha256 };
    }
    *source = seal_sources(source.clone()).unwrap();
    for root in &mut desired.bootstrap.as_mut().unwrap().roots {
        root.capacity_import_bootstrap
            .as_mut()
            .unwrap()
            .review_sha256 = source.source_sha256;
    }
}

fn qualify_declarations(source: &InfrastructureBootstrapRecord) {
    let mut altered = source.clone();
    altered.declarations_toml.push('\n');
    assert!(matches!(
        seal_sources(altered),
        Err(InfrastructureBootstrapError::Integrity)
    ));
    let mut altered = source.clone();
    altered.declarations_toml = altered.declarations_toml.replace(
        "no_other_fleet_ownership = true",
        "no_other_fleet_ownership = false",
    );
    altered.declarations_sha256 = Sha256::digest(altered.declarations_toml.as_bytes()).into();
    assert!(matches!(
        seal_sources(altered),
        Err(InfrastructureBootstrapError::Integrity)
    ));
}

fn qualify_initial_observation(plan: &FleetEnsurePlan, source: &InfrastructureBootstrapRecord) {
    let observed = InfrastructureBootstrapObservation {
        canisters: source
            .sources
            .iter()
            .map(|(name, entry)| (name.clone(), Some(entry.sample.clone())))
            .collect(),
        coordinator_registry: None,
        operator_cycles: source.operator_cycles,
        ledger_fee_cycles: source.ledger_fee_cycles,
    };
    verify_initial(plan, &observed).unwrap();
    let mut drifted = observed.clone();
    drifted
        .canisters
        .get_mut("root-0")
        .unwrap()
        .as_mut()
        .unwrap()
        .binding
        .canister_version += 1;
    assert!(matches!(
        verify_initial(plan, &drifted),
        Err(InfrastructureBootstrapError::Integrity)
    ));
    let mut drifted = observed;
    drifted
        .canisters
        .get_mut("root-0")
        .unwrap()
        .as_mut()
        .unwrap()
        .reserved_cycles += 1;
    assert!(matches!(
        verify_initial(plan, &drifted),
        Err(InfrastructureBootstrapError::Integrity)
    ));
}

fn qualify_inspection_budget(root: &Path, plan: &FleetEnsurePlan) {
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        root,
        &plan.environment,
        "bootstrap-budget-proof",
    );
    let _lock = crate::fleet_ensure::ops::lock_operation(&paths).unwrap();
    for phase in [
        inspection::InspectionPhase::Review,
        inspection::InspectionPhase::Apply,
        inspection::InspectionPhase::Terminal,
    ] {
        inspection::reserve(&paths, plan, phase).unwrap();
        inspection::reserve(&paths, plan, phase).unwrap();
        assert!(matches!(
            inspection::reserve(&paths, plan, phase),
            Err(InfrastructureBootstrapError::InspectionBudget)
        ));
    }
    let action = &plan.canisters[0].actions[0];
    for _ in 0..8 {
        inspection::reserve_effect(&paths, plan, action).unwrap();
    }
    assert!(matches!(
        inspection::reserve_effect(&paths, plan, action),
        Err(InfrastructureBootstrapError::InspectionBudget)
    ));
    let mut changed = plan.clone();
    changed.plan_sha256 = "new-review-time".to_string();
    assert!(matches!(
        inspection::reserve(&paths, &changed, inspection::InspectionPhase::Review),
        Err(InfrastructureBootstrapError::Integrity)
    ));
}

fn qualify_creation(root: &Path, desired: &DesiredFleet, source: &InfrastructureBootstrapRecord) {
    let mut desired = desired.clone();
    let mut source = source.clone();
    source.coordinator = BootstrapCoordinatorSelection::Create;
    source.sources.remove("coordinator");
    let coordinator = desired
        .canisters
        .iter_mut()
        .find(|canister| canister.kind == DesiredCanisterKind::Coordinator)
        .unwrap();
    coordinator.principal = None;
    coordinator.initial_cycles = "300T".to_string();
    reseal(&mut desired, &mut source);
    let plan = prepare(root, &desired, &source, "explicit-coordinator-creation", 44).unwrap();
    let creates = plan
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .filter_map(|action| {
            if let EnsureAction::Create { name, .. } = action {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(creates, ["coordinator"]);
    assert!(
        plan.conservation.maximum_operator_debit_cycles
            > plan.conservation.maximum_new_funding_cycles
    );
}

fn qualify_funding_order(
    root: &Path,
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
) {
    let mut desired = desired.clone();
    let mut source = source.clone();
    source.sources.get_mut("coordinator").unwrap().sample.cycles = 40_000_000_000_000;
    reseal(&mut desired, &mut source);
    let plan = prepare(root, &desired, &source, "fund-stopped-coordinator", 45).unwrap();
    let actions = &plan.canisters[0].actions;
    assert!(matches!(actions[0], EnsureAction::Stop { .. }));
    assert!(matches!(actions[1], EnsureAction::Fund { .. }));
    source.sources.get_mut("coordinator").unwrap().sample.cycles = 0;
    reseal(&mut desired, &mut source);
    assert!(matches!(
        prepare(root, &desired, &source, "insufficient-native-headroom", 45),
        Err(InfrastructureBootstrapError::Policy(
            EnsurePolicyError::InfrastructureBootstrap(_)
        ))
    ));
}

fn qualify_ready_coordinator(
    root: &Path,
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
) {
    let mut desired = desired.clone();
    let mut source = source.clone();
    let bootstrap = desired.bootstrap.as_ref().unwrap();
    let principals = desired
        .canisters
        .iter()
        .map(|configured| {
            (
                configured.name.clone(),
                configured.principal.clone().unwrap(),
            )
        })
        .collect();
    let authority = canic_init::compile_root_authorities(root, &desired, &principals).unwrap()[0]
        .1
        .binding
        .authority
        .clone();
    let admission =
        bind_initial_fleet_admission_policy(authority.binding.fleet.clone(), &bootstrap.admission)
            .unwrap();
    let mut registry = FleetRegistryOps::compile_genesis(
        &bootstrap.app,
        authority,
        &bootstrap
            .component_deployment_configuration
            .component_topology,
        admission,
    )
    .unwrap();
    source.coordinator = BootstrapCoordinatorSelection::Ready;
    source.coordinator_registry_candid_hex =
        Some(hex_bytes(candid::encode_one(&registry).unwrap()));
    let artifacts = resolve_desired_artifacts(root, &desired).unwrap();
    let coordinator = source.sources.get_mut("coordinator").unwrap();
    coordinator.sample.binding.module_sha256 = Some(
        decode_hex(&artifacts.wasm_sha256_by_canister["coordinator"])
            .unwrap()
            .try_into()
            .unwrap(),
    );
    reseal(&mut desired, &mut source);
    let plan = prepare(root, &desired, &source, "ready-coordinator", 46).unwrap();
    assert!(plan.canisters[0].actions.is_empty());
    registry.revision += 1;
    source.coordinator_registry_candid_hex =
        Some(hex_bytes(candid::encode_one(&registry).unwrap()));
    reseal(&mut desired, &mut source);
    assert!(matches!(
        prepare(root, &desired, &source, "wrong-coordinator-registry", 46),
        Err(InfrastructureBootstrapError::Integrity)
    ));
}

// This is a persistence proof with synthetic terminal evidence, not an IC execution fixture.
#[expect(
    clippy::too_many_lines,
    reason = "the persistence proof keeps exact terminal evidence and interrupted publication together"
)]
fn qualify_terminal_publication(
    root: &Path,
    plan: &FleetEnsurePlan,
    source: &InfrastructureBootstrapRecord,
) {
    use crate::fleet_ensure::model::{
        ActualCycleConservation, EffectRecord, EffectState, FleetEnsureCompletion,
        FleetEnsureJournalRecord,
    };
    let desired = plan.reviewed_desired.as_ref().unwrap().desired();
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        root,
        &plan.environment,
        "bootstrap-terminal-proof",
    );
    let _lock = crate::fleet_ensure::ops::lock_operation(&paths).unwrap();
    let mut state = crate::fleet_ensure::ops::read_state(&paths, &plan.fleet).unwrap();
    let canisters = plan
        .canisters
        .iter()
        .map(|target| {
            let configured = desired
                .canisters
                .iter()
                .find(|canister| canister.name == target.name)
                .unwrap();
            let mut sample = source.sources[&target.name].sample.clone();
            sample.binding.module_sha256 = target.actions.iter().find_map(|action| {
                if let EnsureAction::Install { wasm_sha256, .. } = action {
                    Some(decode_hex(wasm_sha256).unwrap().try_into().unwrap())
                } else {
                    None
                }
            });
            sample.binding.controllers = configured
                .controllers
                .iter()
                .map(|id| Principal::from_text(id).unwrap())
                .collect();
            sample.binding.controllers.sort_unstable();
            sample.binding.stopped = false;
            sample.cycles += target
                .actions
                .iter()
                .filter_map(|action| {
                    if let EnsureAction::Fund { amount, .. } = action {
                        Some(*amount)
                    } else {
                        None
                    }
                })
                .sum::<u128>();
            state
                .principals
                .insert(target.name.clone(), sample.binding.canister_id.to_text());
            (target.name.clone(), Some(sample))
        })
        .collect();
    let observed = InfrastructureBootstrapObservation {
        canisters,
        coordinator_registry: None,
        operator_cycles: source.operator_cycles,
        ledger_fee_cycles: source.ledger_fee_cycles,
    };
    let effects = plan
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .map(|action| EffectRecord {
            maintenance_attempts: 0,
            publication_attempts: 0,
            action_sha256: crate::fleet_ensure::ops::action_sha256(action),
            created_principal: None,
            destination_post_cycles: None,
            destination_pre_cycles: None,
            post_cycles: None,
            pre_cycles: None,
            pre_canister_version: Some(7),
            progress_identity: None,
            receipt: matches!(action, EnsureAction::Fund { .. })
                .then(|| "synthetic-persistence-receipt".to_string()),
            state: EffectState::Applied,
        })
        .collect();
    let mut journal = FleetEnsureJournalRecord {
        schema_version: 1,
        operation_id: plan.operation_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        fleet: plan.fleet.clone(),
        completion: FleetEnsureCompletion::InProgress,
        effects,
        successor_phases: vec![],
        funding_observations: BTreeMap::new(),
        funding_reviews: vec![],
        estate_funding_required: None,
        initial_controlled_cycles: plan.conservation.observed_controlled_cycles,
        initial_estate_funding_cycles_by_root: BTreeMap::new(),
        initial_operator_cycles: source.operator_cycles,
        stalled_observations: 1,
    };
    let actual = ActualCycleConservation {
        estate_funding_cycles: 0,
        exact_estate_creation_fee_cycles: 0,
        exact_unavoidable_fee_cycles: plan.conservation.maximum_unavoidable_fee_cycles,
        final_controlled_cycles: plan.conservation.observed_controlled_cycles
            + plan.conservation.maximum_new_funding_cycles,
        observed_starting_cycles: plan.conservation.observed_controlled_cycles,
        observed_net_cycle_debit_cycles: 0,
        observed_net_cycle_credit_cycles: 0,
        operator_debit_cycles: plan.conservation.maximum_operator_debit_cycles,
        received_new_funding_cycles: plan.conservation.maximum_new_funding_cycles,
    };
    let phase = registration::compile(root, plan, &state, actual.final_controlled_cycles).unwrap();
    state.active_registry = Some(registration::registry(&phase).unwrap());
    let template = journal.effects.last().unwrap().clone();
    journal
        .effects
        .extend(phase.protocol_actions.iter().map(|action| {
            let mut effect = template.clone();
            effect.action_sha256 = crate::fleet_ensure::ops::action_sha256(action);
            effect
        }));
    journal = crate::fleet_ensure::ops::continuation::candidate_journal(&journal, &phase, 0);
    terminal::retain(&paths, plan, &journal, &state, actual.clone(), &observed).unwrap();
    assert_eq!(
        terminal::read(&paths, plan, &journal, &state).unwrap(),
        Some(actual.clone())
    );
    journal.completion = FleetEnsureCompletion::Converged;
    journal.stalled_observations = 0;
    assert_eq!(
        terminal::read(&paths, plan, &journal, &state).unwrap(),
        Some(actual)
    );
    let mut invalid = journal.clone();
    invalid.effects.pop();
    assert!(matches!(
        terminal::read(&paths, plan, &invalid, &state),
        Err(InfrastructureBootstrapError::Integrity)
    ));
    let receipt_path = paths
        .plan
        .with_file_name("infrastructure-bootstrap-receipts")
        .join(format!("{}.json", plan.plan_sha256));
    let original = std::fs::read(&receipt_path).unwrap();
    let mut tampered: crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapTerminalRecord = serde_json::from_slice(&original).unwrap();
    tampered.actual.received_new_funding_cycles += 1;
    std::fs::write(&receipt_path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    assert!(matches!(
        terminal::read(&paths, plan, &journal, &state),
        Err(InfrastructureBootstrapError::Integrity)
    ));
    std::fs::write(&receipt_path, original).unwrap();
}

fn qualify_survey_identity(
    root: &Path,
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
) {
    use crate::fleet_ensure::ops::{
        capacity_import::journal::CapacityImportJournalStore,
        infrastructure_bootstrap::survey::BootstrapSurvey,
    };
    let paths = crate::fleet_ensure::ops::EnsurePaths::under(
        root,
        &desired.environment,
        "bootstrap-survey-proof",
    );
    let _owner = CapacityImportJournalStore::open(&paths).unwrap();
    let first = BootstrapSurvey::begin(
        &paths,
        desired,
        source.coordinator,
        &source.declarations_toml,
    )
    .unwrap();
    let again = BootstrapSurvey::begin(
        &paths,
        desired,
        source.coordinator,
        &source.declarations_toml,
    )
    .unwrap();
    assert_eq!(first.request_sha256(), again.request_sha256());
    assert_eq!(first.canisters(), again.canisters());
    assert!(again.completed().is_none());
    let changed = format!("{}\n", source.declarations_toml);
    assert!(matches!(
        BootstrapSurvey::begin(&paths, desired, source.coordinator, &changed),
        Err(InfrastructureBootstrapError::Integrity)
    ));
}
