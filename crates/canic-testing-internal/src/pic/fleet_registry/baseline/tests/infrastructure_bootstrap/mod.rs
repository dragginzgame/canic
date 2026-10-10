//! Exercise supplied infrastructure through production Ensure, including a lost install reply.

use super::*;
use canic_host::fleet_ensure::{
    model::{
        capacity_import::{CapacityImportSourceBinding, survey::CapacityImportSampleRecord},
        infrastructure_bootstrap::BootstrapCoordinatorSelection,
    },
    ops::{
        EnsurePaths,
        capacity_import::admission::{
            CapacityImportDeclaration, CapacityImportDeclarations, CapacityImportDispositionKind,
        },
        read_state,
    },
    workflow::infrastructure_bootstrap,
};
use std::collections::BTreeMap;

#[test]
pub(super) fn supplied_infrastructure_initializes_and_recovers() {
    assert_literal_zero_host_journey(FundingJourney::InfrastructureBootstrap, 1);
}

#[test]
pub(super) fn registration_budget_recovery_preserves_applied_effects() {
    assert_literal_zero_host_journey(FundingJourney::RegistrationRecovery, 1);
}

#[expect(
    clippy::too_many_lines,
    reason = "the single infrastructure journey binds physical observations, interruption and effect-free replay"
)]
pub(super) fn assert_journey(input: ReinstallJourney<'_>, recovery: bool) {
    let operator = Principal::from_text(&input.desired.operator).unwrap();
    let mut desired = input.desired.clone();
    let mut names = BTreeMap::from([
        ("coordinator".to_string(), input.coordinator),
        ("root".to_string(), input.root),
        ("store".to_string(), input.store),
    ]);
    names.extend(
        input
            .pools
            .iter()
            .enumerate()
            .map(|(index, id)| (format!("pool-{index}"), *id)),
    );
    for canister in &mut desired.canisters {
        canister.principal = Some(names[&canister.name].to_text());
        input
            .pic
            .set_controllers(names[&canister.name], Some(operator), vec![operator])
            .unwrap();
    }
    // The two old modules contain disposable stable memory. Root remains empty.
    for id in [input.coordinator, input.store] {
        input
            .pic
            .install_canister(id, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), Some(operator));
        input.pic.set_stable_memory(
            id,
            vec![0x38; 65_536],
            ic_testkit::pocket_ic::common::rest::BlobCompression::NoCompression,
        );
    }
    let network_key: [u8; 32] = wasm_hash(&input.pic.root_key().unwrap())
        .try_into()
        .unwrap();
    let mut samples = BTreeMap::new();
    for (name, id) in &names {
        let status = input.pic.canister_status(*id, Some(operator)).unwrap();
        let mut controllers = status.settings.controllers;
        controllers.sort_unstable();
        samples.insert(
            name.clone(),
            CapacityImportSampleRecord {
                binding: CapacityImportSourceBinding {
                    canister_id: *id,
                    subnet: SubnetId::from_principal(input.pic.get_subnet(*id).unwrap()),
                    controllers,
                    module_sha256: status.module_hash.map(|hash| hash.try_into().unwrap()),
                    canister_version: status.version,
                    stopped: false,
                    snapshots_size_bytes: 0,
                },
                cycles: status.cycles.0.try_into().unwrap(),
                reserved_cycles: status.reserved_cycles.0.try_into().unwrap(),
            },
        );
    }
    assert!(samples["root"].binding.module_sha256.is_none());
    // Current review includes the missing budget credit. The recovery case
    // separately seeds a retained approval issued without that complete quote.
    for target in &mut desired.canisters {
        if ["coordinator", "root", "store"].contains(&target.name.as_str()) {
            target.minimum_cycles = samples[&target.name].cycles.to_string();
        }
    }
    let declarations = CapacityImportDeclarations {
        schema_version: 1,
        operator: operator.to_text(),
        network_root_key_sha256: hex_bytes(network_key),
        canisters: samples
            .values()
            .map(|sample| CapacityImportDeclaration {
                canister: sample.binding.canister_id.to_text(),
                subnet: sample.binding.subnet.to_string(),
                controllers: sample
                    .binding
                    .controllers
                    .iter()
                    .map(Principal::to_text)
                    .collect(),
                module_sha256: sample
                    .binding
                    .module_sha256
                    .map_or_else(|| "empty".to_string(), hex_bytes),
                canister_version: sample.binding.canister_version,
                disposition: CapacityImportDispositionKind::Absence,
                no_external_obligations: true,
                no_other_fleet_ownership: true,
                evidence: "disposable PocketIC staging capacity".to_string(),
            })
            .collect(),
    };
    let declarations_toml = toml::to_string(&declarations).unwrap();
    let bootstrap = desired.bootstrap.as_mut().unwrap();
    bootstrap.canonical_network_id =
        canic_contracts::ids::CanonicalNetworkId::from_der_root_trust_anchor(
            &input.pic.root_key().unwrap(),
        )
        .unwrap();
    bootstrap.fresh_estate = false;
    let executable = input.icp_wrapper.to_str().unwrap();
    let icp = canic_host::icp::IcpCli::new(executable, Some(desired.environment.clone()))
        .with_cwd(input.adapter_root)
        .with_local_replica(Some(input.local_replica.clone()));
    let source = infrastructure_bootstrap::survey(
        input.adapter_root,
        &desired,
        BootstrapCoordinatorSelection::Initialize,
        &declarations_toml,
        &icp,
    )
    .unwrap();
    let unavailable_icp = canic_host::icp::IcpCli::new(
        "/missing/bootstrap-survey-icp",
        Some(desired.environment.clone()),
    );
    assert_eq!(
        infrastructure_bootstrap::survey(
            input.adapter_root,
            &desired,
            BootstrapCoordinatorSelection::Initialize,
            &declarations_toml,
            &unavailable_icp,
        )
        .unwrap(),
        source
    );
    write_import_inputs(&input, &desired, operator, declarations);
    let mut platform = literal_zero_journey_platform(
        &desired,
        input.icp_wrapper,
        input.adapter_root,
        input.local_replica.clone(),
        true,
    );
    let reviewed = if recovery {
        seed_retained_budget_fixture(&input, &desired, &source)
    } else {
        infrastructure_bootstrap::review(
        &canic_host::fleet_ensure::dto::infrastructure_bootstrap::InfrastructureBootstrapReviewRequest {
            workspace: input.adapter_root, desired: &desired,
            coordinator: BootstrapCoordinatorSelection::Initialize,
            declarations_toml: &declarations_toml,
            seed: Path::new("bootstrap-estate.toml"),
            planned_at_time: 1_800_000_000_000_000_001,
        }, &mut platform, &icp,
    ).unwrap()
    };
    desired = reviewed
        .reviewed_desired
        .as_ref()
        .unwrap()
        .desired()
        .clone();
    let source = reviewed.infrastructure_bootstrap.as_ref().unwrap().as_ref();
    let digest = reviewed.desired_sha256.clone();
    if !recovery {
        let initialization = canic_host::fleet_ensure::ops::infrastructure_bootstrap::prepare(
            input.adapter_root,
            &desired,
            source,
            &digest,
            reviewed.planned_at_time,
        )
        .unwrap();
        assert!(
            reviewed.conservation.maximum_new_funding_cycles
                > initialization.conservation.maximum_new_funding_cycles
        );
        assert_eq!(
            reviewed.conservation.maximum_execution_burn_cycles,
            initialization.conservation.maximum_execution_burn_cycles
        );
    }
    std::fs::write(input.adapter_root.join("lose-install-response"), b"1").unwrap();
    // The existing wrapper also loses one controller response; pre-mark that separate fault.
    std::fs::write(input.adapter_root.join("lost-controller-response"), b"1").unwrap();
    let first = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reviewed.plan_sha256,
        &mut platform,
    );
    assert!(
        matches!(first, Err(EnsureWorkflowError::Platform(_))),
        "expected lost install reply: {first:?}"
    );
    assert!(
        input.adapter_root.join("lost-install-response").exists(),
        "{first:?}"
    );
    std::fs::write(input.adapter_root.join("lose-registration-response"), b"1").unwrap();
    let mut resumed = literal_zero_journey_platform(
        &desired,
        input.icp_wrapper,
        input.adapter_root,
        input.local_replica.clone(),
        true,
    );
    let mut interrupted = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reviewed.plan_sha256,
        &mut resumed,
    );
    if recovery {
        let paths = EnsurePaths::under(input.adapter_root, &desired.environment, &desired.fleet);
        let plan_bytes = std::fs::read(&paths.plan).unwrap();
        let before = canic_host::fleet_ensure::ops::read_journal(&paths)
            .unwrap()
            .unwrap();
        assert!(
            matches!(interrupted, Err(EnsureWorkflowError::InfrastructureBootstrap(
            canic_host::fleet_ensure::ops::infrastructure_bootstrap::InfrastructureBootstrapError::RegistrationBudget { execution_shortfall_cycles, .. }
        )) if execution_shortfall_cycles > 0),
            "{interrupted:?}"
        );
        // Exhaust the original registration allowance; recovery must extend it,
        // never reset counters or discard the paid prefix.
        let blocked = fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan_sha256,
            &mut resumed,
        );
        assert!(matches!(blocked, Err(EnsureWorkflowError::InfrastructureBootstrap(
            canic_host::fleet_ensure::ops::infrastructure_bootstrap::InfrastructureBootstrapError::RegistrationBudget { .. }
        ))), "{blocked:?}");
        let review = infrastructure_bootstrap::registration_recovery::review(
            input.adapter_root,
            &desired.environment,
            &desired.fleet,
            &reviewed.plan_sha256,
            1_800_000_000_000_000_101,
            &mut resumed,
        )
        .unwrap();
        assert!(review.additional_funding_cycles > 0);
        assert!(
            review.maximum_execution_burn_cycles
                > reviewed.conservation.maximum_execution_burn_cycles
        );
        assert_eq!(review.plan_sha256, reviewed.plan_sha256);
        let repeated = infrastructure_bootstrap::registration_recovery::review(
            input.adapter_root,
            &desired.environment,
            &desired.fleet,
            &reviewed.plan_sha256,
            1_800_000_000_000_000_999,
            &mut resumed,
        )
        .unwrap();
        assert_eq!(repeated, review);
        let wrong = infrastructure_bootstrap::registration_recovery::approve(
            input.adapter_root,
            &desired.environment,
            &desired.fleet,
            &reviewed.plan_sha256,
            &"ff".repeat(32),
            &mut resumed,
        );
        assert!(matches!(wrong, Err(EnsureWorkflowError::InfrastructureBootstrap(
            canic_host::fleet_ensure::ops::infrastructure_bootstrap::InfrastructureBootstrapError::RegistrationApproval { .. }
        ))));
        infrastructure_bootstrap::registration_recovery::approve(
            input.adapter_root,
            &desired.environment,
            &desired.fleet,
            &reviewed.plan_sha256,
            &review.review_sha256,
            &mut resumed,
        )
        .unwrap();
        std::fs::write(input.adapter_root.join("lose-funding-response"), []).unwrap();
        let lost = fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan_sha256,
            &mut resumed,
        );
        assert!(
            matches!(lost, Err(EnsureWorkflowError::Platform(_))),
            "{lost:?}"
        );
        assert!(input.adapter_root.join("lost-funding-response").exists());
        let after = canic_host::fleet_ensure::ops::read_journal(&paths)
            .unwrap()
            .unwrap();
        assert_eq!(&after.effects[..before.effects.len()], &before.effects);
        assert_eq!(
            after.initial_controlled_cycles,
            before.initial_controlled_cycles
        );
        assert_eq!(
            after.initial_operator_cycles,
            before.initial_operator_cycles
        );
        assert_eq!(std::fs::read(&paths.plan).unwrap(), plan_bytes);
        interrupted = fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan_sha256,
            &mut resumed,
        );
    }
    assert!(
        matches!(interrupted, Err(EnsureWorkflowError::Platform(_))),
        "{interrupted:?}"
    );
    assert!(
        input
            .adapter_root
            .join("lost-registration-response")
            .exists(),
        "{interrupted:?}"
    );
    let mut resumed = literal_zero_journey_platform(
        &desired,
        input.icp_wrapper,
        input.adapter_root,
        input.local_replica.clone(),
        true,
    );
    let completed = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reviewed.plan_sha256,
        &mut resumed,
    )
    .unwrap();
    assert!(completed.terminal);
    assert!(completed.actual_conservation.is_some());
    let log = std::fs::read_to_string(input.adapter_root.join("reinstall-mutations.log")).unwrap();
    for id in [input.coordinator, input.store] {
        let memory = input.pic.get_stable_memory(id);
        assert!(
            !memory
                .windows(4096)
                .any(|page| page.iter().all(|byte| *byte == 0x38))
        );
    }
    for id in [input.coordinator, input.root, input.store] {
        assert_eq!(
            log.lines()
                .filter(|line| line.contains(&id.to_text()))
                .count(),
            1
        );
    }
    let paths = EnsurePaths::under(input.adapter_root, &desired.environment, &desired.fleet);
    let state = read_state(&paths, &desired.fleet).unwrap();
    assert!(
        state
            .active_registry
            .as_ref()
            .unwrap()
            .fleet_subnet_roots
            .iter()
            .all(|entry| entry.status
                == canic_contracts::dto::fleet_registry::FleetSubnetRootStatus::Active)
    );
    let transport = canic_host::fleet_ensure::ops::capacity_import::transport::CapacityImportTransport::from_icp(&icp).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let context = runtime
        .block_on(transport.root_context(input.root))
        .unwrap();
    assert_eq!(
        context.bootstrap.unwrap().review_sha256,
        source.source_sha256
    );
    assert!(context.active_import.is_none());

    for id in input.pools {
        let name = names
            .iter()
            .find(|(_, expected)| *expected == id)
            .unwrap()
            .0;
        assert!(!state.principals.contains_key(name));
        assert!(!state.topology.contains_key(name));
        assert_eq!(
            input
                .pic
                .canister_status(*id, Some(operator))
                .unwrap()
                .version,
            samples[name].binding.canister_version
        );
    }
    let retained = std::fs::read(&paths.journal).unwrap();
    let mut unavailable = IcpEnsurePlatform::new(
        desired.clone(),
        "/missing/bootstrap-replay-icp",
        input.adapter_root,
    );
    for _ in 0..2 {
        let replay = fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &digest,
            &desired.fleet,
            &reviewed.plan_sha256,
            &mut unavailable,
        )
        .unwrap();
        assert_eq!(replay.effects_applied, 0);
    }
    assert_eq!(std::fs::read(&paths.journal).unwrap(), retained);
    let seed = source.estate_seed.as_ref().unwrap();
    let seed_path = input.adapter_root.join(&seed.relative_path);
    std::fs::write(&seed_path, "operator changed the seed after review\n").unwrap();
    let interrupted = infrastructure_bootstrap::apply(
        input.adapter_root,
        &desired.environment,
        &desired.fleet,
        &reviewed.plan_sha256,
        &mut unavailable,
    );
    assert!(matches!(interrupted, Err(EnsureWorkflowError::InfrastructureBootstrap(
        canic_host::fleet_ensure::ops::infrastructure_bootstrap::InfrastructureBootstrapError::Integrity
    ))));
    std::fs::write(&seed_path, &seed.original).unwrap();
    let published = infrastructure_bootstrap::apply(
        input.adapter_root,
        &desired.environment,
        &desired.fleet,
        &reviewed.plan_sha256,
        &mut unavailable,
    )
    .unwrap();
    assert!(published.completed);
    assert_eq!(published.coordinator, input.coordinator);
    assert_eq!(
        infrastructure_bootstrap::apply(
            input.adapter_root,
            &desired.environment,
            &desired.fleet,
            &reviewed.plan_sha256,
            &mut unavailable,
        )
        .unwrap(),
        published
    );
    assert_initial_import(&input, &desired, source, &icp, !recovery);
    assert_workload_convergence(&input, &desired, &reviewed.operation_id);
    assert_eq!(
        infrastructure_bootstrap::apply(
            input.adapter_root,
            &desired.environment,
            &desired.fleet,
            &reviewed.plan_sha256,
            &mut unavailable,
        )
        .unwrap(),
        published
    );
}

/// Seed an already reviewed small ceiling before issuance, then leave its bytes immutable.
/// Production fresh review must never produce this retained-incident fixture.
fn seed_retained_budget_fixture(
    input: &ReinstallJourney<'_>,
    desired: &DesiredFleet,
    source: &canic_host::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapRecord,
) -> canic_host::fleet_ensure::model::FleetEnsurePlan {
    use canic_host::fleet_ensure::{
        model::infrastructure_bootstrap::InfrastructureBootstrapSeedRecord,
        ops::{infrastructure_bootstrap as bootstrap_ops, write_plan},
        policy::expected_plan_sha256,
    };
    let paths = EnsurePaths::under(input.adapter_root, &desired.environment, &desired.fleet);
    assert!(!paths.plan.exists());
    assert!(!paths.journal.exists());
    for entry in source.sources.values() {
        let status = input
            .pic
            .canister_status(entry.sample.binding.canister_id, Some(source.operator))
            .unwrap();
        assert_eq!(status.version, entry.sample.binding.canister_version);
        assert_eq!(
            status.module_hash,
            entry.sample.binding.module_sha256.map(|hash| hash.to_vec())
        );
    }
    let original =
        std::fs::read_to_string(input.adapter_root.join("bootstrap-estate.toml")).unwrap();
    let mut source = source.clone();
    source.estate_seed = Some(InfrastructureBootstrapSeedRecord {
        relative_path: "bootstrap-estate.toml".into(),
        before_sha256: wasm_hash(original.as_bytes()).try_into().unwrap(),
        original,
    });
    let source = bootstrap_ops::seal_sources(source).unwrap();
    let mut desired = desired.clone();
    for root in &mut desired.bootstrap.as_mut().unwrap().roots {
        let mut sources = root
            .canister_pool_imports
            .iter()
            .map(|name| source.sources[name].sample.binding.canister_id)
            .collect::<Vec<_>>();
        sources.sort_unstable();
        root.capacity_import_bootstrap = Some(
            canic_host::fleet_ensure::model::capacity_import::CapacityImportBootstrapRecord {
                review_sha256: source.source_sha256,
                operator: source.operator,
                sources,
            },
        );
    }
    let digest = canic_core::cdk::utils::hash::sha256_hex(
        toml::to_string_pretty(&desired).unwrap().as_bytes(),
    );
    let mut plan = bootstrap_ops::prepare(
        input.adapter_root,
        &desired,
        &source,
        &digest,
        1_800_000_000_000_000_001,
    )
    .unwrap();
    let available =
        plan.conservation.observed_controlled_cycles + plan.conservation.maximum_new_funding_cycles;
    let floors = plan
        .canisters
        .iter()
        .map(|target| {
            let configured = desired
                .canisters
                .iter()
                .find(|entry| entry.name == target.name)
                .unwrap();
            configured
                .minimum_cycles
                .parse::<canic_contracts::cycles::Cycles>()
                .unwrap()
                .to_u128()
                + source.sources[&target.name].sample.reserved_cycles
        })
        .sum::<u128>();
    let spendable = available.checked_sub(floors).unwrap();
    assert!(plan.conservation.maximum_execution_burn_cycles > spendable);
    plan.conservation.maximum_execution_burn_cycles = spendable;
    plan.conservation.expected_post_operation_cycles = floors;
    plan.plan_sha256 = expected_plan_sha256(&plan);
    write_plan(&paths, &plan).unwrap();
    plan
}

fn assert_workload_convergence(
    input: &ReinstallJourney<'_>,
    initialized: &DesiredFleet,
    setup_operation_id: &str,
) {
    let mut desired = initialized.clone();
    for root in &mut desired.bootstrap.as_mut().unwrap().roots {
        root.capacity_import_bootstrap = None;
    }
    let digest = desired_sha256(&desired);
    let mut platform = literal_zero_journey_platform(
        &desired,
        input.icp_wrapper,
        input.adapter_root,
        input.local_replica.clone(),
        true,
    );
    let reviewed = fleet_ensure_workflow::plan(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        1_800_000_000_000_000_009,
        &mut platform,
    )
    .expect("review ordinary convergence after bootstrap import");
    assert_eq!(reviewed.plan.operation_id, setup_operation_id);
    assert!(
        planned_actions(&reviewed.plan)
            .iter()
            .all(|action| !matches!(
                action,
                EnsureAction::Create { .. }
                    | EnsureAction::Install { .. }
                    | EnsureAction::Uninstall { .. }
            ))
    );
    let completed = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reviewed.plan.plan_sha256,
        &mut platform,
    )
    .expect("provision imported identities and activate Root");
    assert!(completed.terminal);
    let pool = root_pool_status_as(
        input.pic,
        input.root,
        Principal::from_text(&desired.operator).unwrap(),
    );
    assert_eq!((pool.workload, pool.ready), (1, 1));
    assert!(pool.pending_creation.is_none());
    for id in input.pools {
        assert!(pool.entries.iter().any(|entry| entry.canister_id == *id));
    }
    let replay = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reviewed.plan.plan_sha256,
        &mut platform,
    )
    .unwrap();
    assert!(replay.terminal);
    assert_eq!(replay.effects_applied, 0);
}

fn assert_initial_import(
    input: &ReinstallJourney<'_>,
    desired: &DesiredFleet,
    source: &canic_host::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapRecord,
    icp: &canic_host::icp::IcpCli,
    recognize_funding: bool,
) {
    use canic_host::fleet_ensure::{
        dto::capacity_import::CapacityImportReviewRequest, workflow::capacity_import::review,
    };
    let maximum_root_paid_calls =
        canic_core::control_plane_support::policy::pool_import::recommended_calls(
            input.pools.len(),
        )
        .unwrap();
    let context = super::capacity_import::context(
        input.pic,
        input.root,
        Principal::from_text(&desired.operator).unwrap(),
    );
    let mut request = CapacityImportReviewRequest {
        funding_credits: Vec::new(),
        environment: desired.environment.clone(),
        fleet: desired.fleet.clone(),
        canisters: input.pools.to_vec(),
        root: Some(input.root),
        declarations: "bootstrap-pools.toml".into(),
        policy: "bootstrap-policy.toml".into(),
        seed: "bootstrap-estate.toml".into(),
        maximum_source_debit_cycles: 100_000_000_000,
        maximum_root_debit_cycles:
            canic_core::control_plane_support::policy::pool_import::required_debit(
                context.maximum_call_debit_cycles,
                maximum_root_paid_calls,
            )
            .unwrap(),
        maximum_root_paid_calls,
    };
    if recognize_funding {
        add_reviewed_import_funding(
            input,
            desired,
            source,
            icp,
            &mut request,
            context
                .binding
                .limits
                .canister_pool
                .canister_cycles
                .to_u128(),
        );
    }
    let planned = review::plan(input.adapter_root, &request, icp).unwrap();
    assert_eq!(planned.operation.as_ref().unwrap().review.publication_kind,
        canic_host::fleet_ensure::model::capacity_import::operation::CapacityImportPublicationKind::InitializeEstate);
    for reviewed in &planned.plan.sources {
        let original = source
            .sources
            .values()
            .find(|source| source.sample.binding.canister_id == reviewed.binding.canister_id)
            .unwrap();
        let credited = request
            .funding_credits
            .iter()
            .find(|credit| credit.canister == reviewed.binding.canister_id)
            .map_or(0, |credit| credit.cycles);
        assert_eq!(reviewed.observed_cycles, original.sample.cycles + credited);
        assert_eq!(
            reviewed.observed_reserved_cycles,
            original.sample.reserved_cycles
        );
    }
    let digest = planned.operation.unwrap().review.review_sha256;
    let completed = review::apply(
        input.adapter_root,
        &desired.environment,
        &desired.fleet,
        digest,
        icp,
    )
    .unwrap();
    assert!(canic_host::fleet_ensure::ops::capacity_import::publication::completed(&completed));
    for id in input.pools {
        let status = input.pic.canister_status(*id, Some(input.root)).unwrap();
        assert_eq!(status.module_hash, None);
        assert_eq!(status.settings.controllers, vec![input.root]);
    }
    let unavailable = canic_host::icp::IcpCli::new(
        "/missing/import-replay-icp",
        Some(desired.environment.clone()),
    );
    assert_eq!(
        review::apply(
            input.adapter_root,
            &desired.environment,
            &desired.fleet,
            digest,
            &unavailable
        )
        .unwrap(),
        completed
    );
}

fn add_reviewed_import_funding(
    input: &ReinstallJourney<'_>,
    desired: &DesiredFleet,
    source: &canic_host::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapRecord,
    icp: &canic_host::icp::IcpCli,
    request: &mut canic_host::fleet_ensure::dto::capacity_import::CapacityImportReviewRequest,
    minimum: u128,
) {
    use canic_host::fleet_ensure::{
        dto::capacity_import::CapacityImportFundingCreditRequest,
        ops::capacity_import::{CapacityImportReviewError, journal::CapacityImportJournalError},
        policy::capacity_import::CapacityImportPolicyError,
        workflow::capacity_import::review,
    };
    let paths = EnsurePaths::under(input.adapter_root, &desired.environment, &desired.fleet);
    let original_plan = std::fs::read(&paths.plan).unwrap();
    let original_journal = std::fs::read(&paths.journal).unwrap();
    request.maximum_source_debit_cycles = input
        .pools
        .iter()
        .map(|id| {
            source
                .sources
                .values()
                .find(|source| source.sample.binding.canister_id == *id)
                .unwrap()
                .sample
                .cycles
        })
        .max()
        .unwrap();
    let failure = review::plan(input.adapter_root, request, icp).unwrap_err();
    assert!(
        matches!(failure, CapacityImportJournalError::Review(CapacityImportReviewError::Policy(
        CapacityImportPolicyError::InsufficientCycles { required_cycles, available_cycles, shortfall_cycles, .. }
    )) if required_cycles > available_cycles && shortfall_cycles == required_cycles - available_cycles),
        "{failure:?}"
    );
    let mut credits = Vec::new();
    for id in input.pools {
        let original = &source
            .sources
            .values()
            .find(|source| source.sample.binding.canister_id == *id)
            .unwrap()
            .sample;
        let cycles =
            minimum + request.maximum_source_debit_cycles - original.cycles + 100_000_000_000;
        input.pic.add_cycles(*id, cycles);
        credits.push(CapacityImportFundingCreditRequest {
            canister: *id,
            cycles,
        });
    }
    assert!(matches!(
        review::plan(input.adapter_root, request, icp),
        Err(CapacityImportJournalError::Review(
            CapacityImportReviewError::Policy(CapacityImportPolicyError::InsufficientCycles { .. })
        ))
    ));
    request.funding_credits = credits;
    let funded = review::plan(input.adapter_root, request, icp).unwrap();
    assert_eq!(funded.plan.funding_credits.len(), input.pools.len());
    for credit in &funded.plan.funding_credits {
        let original = &source
            .sources
            .values()
            .find(|source| source.sample.binding.canister_id == credit.before.binding.canister_id)
            .unwrap()
            .sample;
        assert_eq!(&credit.before, original);
        assert_eq!(credit.observed.binding, original.binding);
    }
    assert_eq!(std::fs::read(&paths.plan).unwrap(), original_plan);
    assert_eq!(std::fs::read(&paths.journal).unwrap(), original_journal);
}

fn write_import_inputs(
    input: &ReinstallJourney<'_>,
    desired: &DesiredFleet,
    operator: Principal,
    mut declarations: CapacityImportDeclarations,
) {
    let bootstrap = desired.bootstrap.as_ref().unwrap();
    let subnet = bootstrap.roots[0].placement_subnet.into_principal();
    let policy = generated_journey_policy(
        operator,
        subnet,
        1,
        bootstrap.roots[0].limits.canister_pool.maximum_size as usize - 1,
        input.config,
    );
    let ids = input
        .pools
        .iter()
        .map(Principal::to_text)
        .collect::<Vec<_>>();
    let seed = toml::toml! {
        schema_version = 1
        fleet_id = (bootstrap.fleet_id.to_string())
        fresh_estate = false
        coordinator = (input.coordinator.to_text())
        cycles_ledger = (desired.cycles_ledger.clone())
        management_creation_fee_cycles = (desired.management_creation_fee_cycles.parse::<canic_contracts::cycles::Cycles>().unwrap().to_config_string())
        [[roots]]
        placement_subnet = (subnet.to_text())
        root = (input.root.to_text())
        store = (input.store.to_text())
        pool_imports = ids
    };
    declarations.canisters.retain(|entry| {
        input
            .pools
            .contains(&Principal::from_text(&entry.canister).unwrap())
    });
    std::fs::write(input.adapter_root.join("bootstrap-policy.toml"), policy).unwrap();
    std::fs::write(
        input.adapter_root.join("bootstrap-estate.toml"),
        toml::to_string_pretty(&seed).unwrap(),
    )
    .unwrap();
    std::fs::write(
        input.adapter_root.join("bootstrap-pools.toml"),
        toml::to_string_pretty(&declarations).unwrap(),
    )
    .unwrap();
}
