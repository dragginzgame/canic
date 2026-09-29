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

#[expect(
    clippy::too_many_lines,
    reason = "the single infrastructure journey binds physical observations, interruption and effect-free replay"
)]
pub(super) fn assert_journey(input: ReinstallJourney<'_>) {
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
        canic_core::ids::CanonicalNetworkId::from_der_root_trust_anchor(
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
    let reviewed = infrastructure_bootstrap::review(
        &canic_host::fleet_ensure::dto::infrastructure_bootstrap::InfrastructureBootstrapReviewRequest {
            workspace: input.adapter_root, desired: &desired,
            coordinator: BootstrapCoordinatorSelection::Initialize,
            declarations_toml: &declarations_toml,
            seed: Path::new("bootstrap-estate.toml"),
            planned_at_time: 1_800_000_000_000_000_001,
        }, &mut platform, &icp,
    ).unwrap();
    desired = reviewed
        .reviewed_desired
        .as_ref()
        .unwrap()
        .desired()
        .clone();
    let source = reviewed.infrastructure_bootstrap.as_ref().unwrap().as_ref();
    let digest = reviewed.desired_sha256.clone();
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
    let interrupted = fleet_ensure_workflow::apply(
        input.adapter_root,
        &desired,
        &digest,
        &desired.fleet,
        &reviewed.plan_sha256,
        &mut resumed,
    );
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
                == canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active)
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
    assert_initial_import(&input, &desired, source, &icp);
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
    let request = CapacityImportReviewRequest {
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
    let planned = review::plan(input.adapter_root, &request, icp).unwrap();
    assert_eq!(planned.operation.as_ref().unwrap().review.publication_kind,
        canic_host::fleet_ensure::model::capacity_import::operation::CapacityImportPublicationKind::InitializeEstate);
    for reviewed in &planned.plan.sources {
        let original = source
            .sources
            .values()
            .find(|source| source.sample.binding.canister_id == reviewed.binding.canister_id)
            .unwrap();
        assert_eq!(reviewed.observed_cycles, original.sample.cycles);
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
        management_creation_fee_cycles = (desired.management_creation_fee_cycles.parse::<canic_core::cdk::types::Cycles>().unwrap().to_config_string())
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
