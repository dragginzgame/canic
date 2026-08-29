use super::*;
use crate::{
    fleet_ensure::{
        model::{
            CanisterRuntimeStatus, CurrentFleetProtocolAction, EffectRecord, EnsureAction,
            FleetEnsureStateRecord, FleetObservation, LiveCanister, RootOwnedCanisterLifecycle,
        },
        ops::{
            EffectObservation, EffectOutcome, EffectRetry, EnsurePaths, EnsurePlatform,
            IcpEnsurePlatform, IcpEnsurePlatformError, write_state,
        },
        workflow,
    },
    network::{NetworkEnrollmentOptions, enroll_network},
    release_build::{finalize_release_build_from_manifest, plan_release_build_for_profile},
    release_set::{
        ApplicationArtifactBuildOutput, ApplicationArtifactBuildTarget, ApplicationArtifactUnion,
        CanicInfrastructureArtifactManifest, CurrentReleaseSetManifest,
    },
    test_support::temp_dir,
};
use canic_control_plane::{
    dto::template::TemplateChunkSetPrepareInput,
    ids::{TemplateId, TemplateVersion},
};
use canic_core::{
    cdk::utils::hash::{hex_bytes, sha256_hex},
    dto::pool::{CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus},
    dto::{
        component_provisioning::{
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetComponentProvisioningPrepareRequest,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        CanisterRole, ComponentDeploymentConfigurationDigest, FleetCoordinatorBinding, FleetKey,
        FleetRegistryAuthority, FleetSubnetRootReleaseSet, ReleaseBuildNonce, ReleaseSetDigest,
    },
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey},
};
use flate2::{Compression, GzBuilder};
#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
use std::{collections::BTreeSet, fs, io, io::Write as _};

#[test]
fn estate_seed_retains_explicit_fleet_id_independent_from_operator() {
    let fleet_id = "a5".repeat(32);
    let coordinator = Principal::from_slice(&[2]).to_text();
    let seed: EstateSeed = toml::from_str(&format!(
        r#"
schema_version = 1
fleet_id = "{fleet_id}"
coordinator = "{coordinator}"
roots = []
"#
    ))
    .expect("estate seed with retained Fleet ID");

    assert_eq!(seed.fleet_id.to_string(), fleet_id);
}

#[test]
fn treasury_adoption_requires_one_observed_seeded_identity() {
    let seed = EstateSeed {
        schema_version: 1,
        fleet_id: "a6".repeat(32).parse().expect("Fleet ID"),
        fresh_estate: false,
        coordinator: Principal::from_slice(&[3]).to_text(),
        treasury: None,
        cycles_ledger: mainnet_cycles_ledger(),
        management_creation_fee_cycles: None,
        roots: Vec::new(),
    };
    let observed = BTreeMap::<String, ObservedCanister>::new();
    let treasury = seed
        .treasury
        .as_ref()
        .map_or(seed.coordinator.as_str(), |treasury| {
            treasury.principal.as_str()
        });

    assert!(!observed.contains_key(treasury));
}

#[test]
fn release_build_network_must_match_the_selected_environment() {
    let release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([7; 32]));

    require_release_build_network(
        release_build_id,
        BuildNetwork::Ic,
        "staging",
        BuildNetwork::Ic,
    )
    .expect("IC artifact is admitted for an IC environment");
    assert!(matches!(
        require_release_build_network(
            release_build_id,
            BuildNetwork::Local,
            "staging",
            BuildNetwork::Ic,
        ),
        Err(FleetGenerateError::Release(reason))
            if reason.contains("targets local") && reason.contains("requires ic")
    ));
}

#[test]
fn retained_identities_and_controller_sets_are_exact() {
    let principal = Principal::from_slice(&[5]).to_text();
    let foreign = Principal::from_slice(&[6]).to_text();
    let mut identities = BTreeSet::new();

    insert_seed_identity(&mut identities, "Coordinator", &principal).expect("first role");
    assert!(matches!(
        insert_seed_identity(&mut identities, "Root", &principal),
        Err(FleetGenerateError::SeedTopology(_))
    ));
    require_exact_controllers(&principal, vec![principal.clone()], vec![principal.clone()])
        .expect("exact controller set");
    assert!(matches!(
        require_exact_controllers(
            &principal,
            vec![principal.clone(), foreign],
            vec![principal.clone()]
        ),
        Err(FleetGenerateError::ControllerMismatch { .. })
    ));
}

#[test]
fn root_policy_drift_requires_the_reviewed_reinstall() {
    let operator = Principal::from_slice(&[11]);
    let source = multi_component_source(
        &operator.to_text(),
        &principal_text(12),
        &principal_text(13),
    );
    let root = source.fleet_subnet_roots.first().expect("Root policy");
    let expected = RootDesiredPolicy {
        component_admissions: Vec::new(),
        component_topology_digest: canic_core::ids::ComponentTopologyDigest::from_bytes([14; 32]),
        funding: root_funding(source.funding_profile, &root.root_funding),
        installation_controller: operator,
        limits: root_limits(root),
    };
    let mut retained = expected.clone();
    retained.limits.canister_pool.canister_cycles = Cycles::new(2_000_000_000_000);

    require_root_policy_convergence(
        "retained-root",
        &retained,
        &expected,
        Some(&"15".repeat(32)),
        &"16".repeat(32),
    )
    .expect("old pool policy converges through current Root reinstall");
    assert!(matches!(
        require_root_policy_convergence(
            "retained-root",
            &retained,
            &expected,
            Some(&"16".repeat(32)),
            &"16".repeat(32),
        ),
        Err(FleetGenerateError::SeedTopology(_))
    ));
    require_root_policy_convergence(
        "retained-root",
        &expected,
        &expected,
        Some(&"16".repeat(32)),
        &"16".repeat(32),
    )
    .expect("matching current policy needs no reinstall");
}

#[test]
fn multi_root_generation_joins_topology_by_typed_subnet_identity() {
    let (first_subnet, second_subnet) = divergent_principal_order_pair();
    let first_text = first_subnet.to_text();
    let second_text = second_subnet.to_text();
    let operator = Principal::from_slice(&[41]).to_text();
    let coordinator_subnet = Principal::from_slice(&[42]).to_text();
    let source_template = multi_component_source(&operator, &coordinator_subnet, &first_text)
        .fleet_subnet_roots
        .remove(0);
    let mut sources = vec![source_template.clone(), source_template];
    sources[0].placement_subnet.clone_from(&first_text);
    sources[1].placement_subnet.clone_from(&second_text);
    sources.sort_by(|left, right| left.placement_subnet.cmp(&right.placement_subnet));
    let mut seeds = vec![
        RootSeed {
            placement_subnet: first_text,
            root: Principal::from_slice(&[43]).to_text(),
            store: Principal::from_slice(&[44]).to_text(),
            pool_imports: Vec::new(),
        },
        RootSeed {
            placement_subnet: second_text,
            root: Principal::from_slice(&[45]).to_text(),
            store: Principal::from_slice(&[46]).to_text(),
            pool_imports: Vec::new(),
        },
    ];
    seeds.sort_by(|left, right| left.placement_subnet.cmp(&right.placement_subnet));
    let mut planned = sources
        .iter()
        .enumerate()
        .map(|(index, source)| PlannedFleetSubnetRootTopology {
            placement_subnet: parse_subnet("test Root", &source.placement_subnet)
                .expect("typed test Subnet"),
            component_admissions: Vec::new(),
            component_topology_digest: canic_core::ids::ComponentTopologyDigest::from_bytes(
                [u8::try_from(index + 1).expect("bounded test index"); 32],
            ),
            limits: root_limits(source),
        })
        .collect::<Vec<_>>();
    planned.sort_by_key(|root| root.placement_subnet);

    let text_order = sources
        .iter()
        .map(|root| root.placement_subnet.clone())
        .collect::<Vec<_>>();
    let typed_order = planned
        .iter()
        .map(|root| root.placement_subnet.to_string())
        .collect::<Vec<_>>();
    assert_ne!(
        text_order, typed_order,
        "fixture must exercise divergent orders"
    );

    let bindings =
        bind_root_generation_inputs(&sources, &seeds, &planned).expect("exact typed Subnet join");
    assert_eq!(bindings.len(), 2);
    for binding in bindings {
        assert_eq!(
            binding.source.placement_subnet,
            binding.planned.placement_subnet.to_string()
        );
        assert_eq!(
            binding.seed.placement_subnet,
            binding.planned.placement_subnet.to_string()
        );
    }
}

#[test]
fn separate_treasury_seed_carries_exact_placement() {
    let coordinator = Principal::from_slice(&[8]).to_text();
    let treasury = Principal::from_slice(&[9]).to_text();
    let subnet = Principal::from_slice(&[10]).to_text();
    let fleet_id = "a9".repeat(32);
    let seed: EstateSeed = toml::from_str(&format!(
        r#"
schema_version = 1
fleet_id = "{fleet_id}"
coordinator = "{coordinator}"
roots = []

[treasury]
principal = "{treasury}"
subnet = "{subnet}"
"#
    ))
    .expect("typed treasury seed");

    assert_eq!(
        seed.treasury,
        Some(TreasurySeed {
            principal: treasury,
            subnet,
        })
    );
}

#[test]
fn protected_policy_and_estate_seed_have_distinct_authority_shapes() {
    let principal = Principal::from_slice(&[7]).to_text();
    let source = format!(
        r#"
schema_version = 1
funding_profile = "preview_multi_subnet"
operator = "{principal}"

[admission]
principals = ["{principal}"]

[coordinator.subnet]
kind = "explicit"
subnet = "{principal}"
acknowledge_fiduciary_cost = false

[coordinator.creation_funding]
kind = "cycles"
cycles = "140T"

[coordinator.root_funding]
minimum_reserve_cycles = "80T"
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 2
maximum_automatic_cycles = "60T"

[[fleet_subnet_roots]]
placement_subnet = "{principal}"
component_admissions = {{ app = 1 }}

[fleet_subnet_roots.component_group_placements]
app = [0]

[fleet_subnet_roots.canister_pool]
minimum_size = 2
maximum_size = 2
canister_cycles = "5T"

[fleet_subnet_roots.root_funding]
request_threshold = "10T"
target_balance = "30T"
cooldown_secs = 2592000
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 2
maximum_automatic_cycles = "60T"

[fleet_subnet_roots.limits]
maximum_component_instances = 8
maximum_registry_bytes = 1048576
maximum_wasm_store_bytes = 1048576
maximum_group_placements = 8

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 2592000
maximum_cycles = "30T"

[fleet_subnet_roots.root_creation_funding]
kind = "cycles"
cycles = "30T"

[fleet_subnet_roots.wasm_store_creation_funding]
kind = "cycles"
cycles = "10T"
"#
    );
    let source: FleetSource = toml::from_str(&source).expect("protected policy shape");
    let seed: EstateSeed = toml::from_str(&format!(
        r#"
schema_version = 1
fleet_id = "{}"
coordinator = "{principal}"

[[roots]]
placement_subnet = "{principal}"
root = "{principal}"
store = "{principal}"
pool_imports = []
"#,
        "a7".repeat(32)
    ))
    .expect("estate identity seed shape");

    assert_eq!(source.fleet_subnet_roots.len(), 1);
    assert_eq!(seed.treasury, None);
    assert_eq!(seed.roots.len(), 1);
}

#[test]
fn generation_rejects_component_demand_above_pool_target_before_observation() {
    let root = temp_dir("fleet-generate-pool-capacity");
    let app_config = root.join("apps/demo/canic.toml");
    let source_path = root.join("deployments/staging.toml");
    let seed_path = root.join("deployments/staging.estate.toml");
    fs::create_dir_all(app_config.parent().expect("App config parent"))
        .expect("create App config parent");
    fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source parent");
    fs::write(&app_config, multi_component_config()).expect("write App config");
    let operator = principal_text(20);
    let coordinator = principal_text(21);
    let fleet_root = principal_text(22);
    let store = principal_text(23);
    let pool_one = principal_text(24);
    let pool_two = principal_text(25);
    let placement = principal_text(26);
    let coordinator_subnet = principal_text(27);
    let source = multi_component_source_toml(&operator, &coordinator_subnet, &placement).replacen(
        "canister_cycles = \"5T\"",
        "canister_cycles = \"4800000000000\"",
        1,
    );
    fs::write(&source_path, source).expect("write insufficient pool source");
    fs::write(
        &seed_path,
        retained_estate_seed_toml(
            "a8".repeat(32).parse().expect("Fleet ID"),
            &coordinator,
            &placement,
            &fleet_root,
            &store,
            [&pool_one, &pool_two],
        ),
    )
    .expect("write estate seed");
    let request = FleetGenerateRequest {
        app_config: &app_config,
        environment: "local",
        fleet: "staging",
        icp_executable: "must-not-run",
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([9; 32])),
        root: &root,
        seed: &seed_path,
        source: &source_path,
    };

    assert!(matches!(
        generate_desired_fleet(&request),
        Err(FleetGenerateError::ComponentPoolCapacity(
            RootPoolCapacityError::Insufficient {
                component_spec,
                pool_target_cycles: 4_800_000_000_000,
                required_cycles: 5_000_000_000_000,
                root,
            }
        )) if component_spec.as_str() == "app" && root == fleet_root
    ));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one retained-estate journey keeps generation, convergence, conservation, and replay together"
)]
fn generated_multi_component_retained_estate_plans_applies_and_replays_without_effect() {
    let root = temp_dir("fleet-generate-retained-journey");
    let app_config = root.join("apps/demo/canic.toml");
    fs::create_dir_all(app_config.parent().expect("App config parent"))
        .expect("create App config parent");
    fs::write(&app_config, multi_component_config()).expect("write App config");
    let canonical_network_id = enroll_test_network(&root);
    let config = AppConfigSnapshot::load(&app_config).expect("load App config");
    let operator = principal_text(20);
    let coordinator = principal_text(21);
    let fleet_root = principal_text(22);
    let store = principal_text(23);
    let pool_one = principal_text(24);
    let pool_two = principal_text(25);
    let placement = principal_text(26);
    let coordinator_subnet = principal_text(27);
    let source = multi_component_source(&operator, &coordinator_subnet, &placement);
    let seed = EstateSeed {
        schema_version: 1,
        fleet_id: "a8".repeat(32).parse().expect("Fleet ID"),
        fresh_estate: false,
        coordinator: coordinator.clone(),
        treasury: None,
        cycles_ledger: mainnet_cycles_ledger(),
        management_creation_fee_cycles: None,
        roots: vec![RootSeed {
            placement_subnet: placement.clone(),
            root: fleet_root.clone(),
            store: store.clone(),
            pool_imports: vec![pool_one.clone(), pool_two.clone()],
        }],
    };
    validate_identity_seed(&source, &seed).expect("retained identities");
    let root_inputs = source
        .fleet_subnet_roots
        .iter()
        .map(|root| PlannedFleetSubnetRootTopologyInput {
            placement_subnet: parse_subnet("Root", &root.placement_subnet).expect("Root Subnet"),
            component_admissions: root
                .component_admissions
                .iter()
                .map(
                    |(component_spec, maximum_root_instances)| RootComponentAdmissionInput {
                        component_spec: component_spec.clone(),
                        maximum_root_instances: *maximum_root_instances,
                    },
                )
                .collect(),
            limits: root_limits(root),
        })
        .collect();
    let topology =
        plan_initial_fleet_topology(config.model(), root_inputs).expect("initial Fleet topology");
    let release_build =
        plan_release_build_for_profile(&root, crate::build_profile::CanisterBuildProfile::Fast)
            .expect("plan retained release build");
    let release_build_id = release_build.record.release_build_id;
    persist_test_release_authority(&root, &config, release_build_id);
    let source_path = root.join("deployments/retained-multi-component.toml");
    let seed_path = root.join("deployments/retained-multi-component.estate.toml");
    fs::create_dir_all(source_path.parent().expect("deployment parent"))
        .expect("create deployment parent");
    fs::write(
        &source_path,
        multi_component_source_toml(&operator, &coordinator_subnet, &placement),
    )
    .expect("write protected Fleet source");
    fs::write(
        &seed_path,
        retained_estate_seed_toml(
            seed.fleet_id,
            &coordinator,
            &placement,
            &fleet_root,
            &store,
            [&pool_one, &pool_two],
        ),
    )
    .expect("write retained estate seed");
    let retained_authority = retained_root_authority(
        canonical_network_id,
        config.model().app_id().clone(),
        seed.fleet_id,
        &source,
        &topology.fleet_subnet_roots[0],
        &operator,
        &coordinator,
        &coordinator_subnet,
        &fleet_root,
        &store,
        release_build_id,
    );
    let retained_pool = retained_pool_response(
        retained_authority.binding.limits.canister_pool.clone(),
        &store,
        &pool_one,
        &pool_two,
    );
    let icp = write_fake_icp(
        &root,
        FakeIcpFixture {
            authority: &retained_authority,
            coordinator: &coordinator,
            coordinator_module_hash: &"83".repeat(32),
            fleet_root: &fleet_root,
            operator: &operator,
            pool: &retained_pool,
            public_cycle_balance: None,
            root_module_hash: &"84".repeat(32),
            root_status_error: None,
            store: &store,
            store_module_hash: &"85".repeat(32),
        },
    );
    let request = FleetGenerateRequest {
        app_config: &app_config,
        environment: "local",
        fleet: "retained-multi-component",
        icp_executable: icp.to_str().expect("fake ICP path"),
        release_build_id,
        root: &root,
        seed: &seed_path,
        source: &source_path,
    };
    let generated = generate_desired_fleet(&request).expect("generate from live retained estate");
    assert_eq!(generated.observed_canisters, 5);
    assert_eq!(generated.observed_controlled_cycles, 319_900_000_000_000);
    assert_eq!(generated.release_build_id, release_build_id);
    let desired = generated.desired;
    let observed = [
        (&coordinator, 270_000_000_000_000_u128, &coordinator_subnet),
        (&fleet_root, 30_000_000_000_000, &placement),
        (&store, 10_000_000_000_000, &placement),
        (&pool_one, 4_900_000_000_000, &placement),
        (&pool_two, 5_000_000_000_000, &placement),
    ]
    .into_iter()
    .map(|(principal, cycles, subnet)| {
        (
            principal.clone(),
            ObservedCanister {
                cycles,
                module_sha256: None,
                subnet: subnet.clone(),
            },
        )
    })
    .collect::<BTreeMap<_, _>>();
    assert_eq!(
        desired
            .canisters
            .iter()
            .filter(|canister| canister.kind == DesiredCanisterKind::Pool)
            .count(),
        2,
        "both paid pool assets remain explicitly retained"
    );
    assert_eq!(desired.ledger_fee_cycles, "100000000");
    assert_eq!(desired.management_creation_fee_cycles, "0");

    let fresh_seed_path = root.join("deployments/fresh-multi-component.estate.toml");
    let fresh_id = initialize_fresh_estate_seed(&FreshEstateSeedRequest {
        cycles_ledger: &mainnet_cycles_ledger(),
        management_creation_fee_cycles: 500_000_000_000,
        seed: &fresh_seed_path,
        source: &source_path,
    })
    .expect("initialize durable fresh estate seed");
    let repeated_id = initialize_fresh_estate_seed(&FreshEstateSeedRequest {
        cycles_ledger: &mainnet_cycles_ledger(),
        management_creation_fee_cycles: 500_000_000_000,
        seed: &fresh_seed_path,
        source: &source_path,
    })
    .expect("replay durable fresh estate seed");
    assert_eq!(repeated_id, fresh_id);
    let fresh = generate_desired_fleet(&FleetGenerateRequest {
        app_config: &app_config,
        environment: "local",
        fleet: "fresh-multi-component",
        icp_executable: icp.to_str().expect("fake ICP path"),
        release_build_id,
        root: &root,
        seed: &fresh_seed_path,
        source: &source_path,
    })
    .expect("generate literally empty estate");
    assert_eq!(fresh.observed_canisters, 0);
    assert_eq!(fresh.observed_controlled_cycles, 0);
    assert_eq!(fresh.desired.treasury, "coordinator");
    assert_eq!(fresh.desired.management_creation_fee_cycles, "500000000000");
    assert!(
        fresh
            .desired
            .bootstrap
            .as_ref()
            .is_some_and(|bootstrap| bootstrap.fresh_estate)
    );
    assert!(
        fresh
            .desired
            .canisters
            .iter()
            .all(|canister| canister.principal.is_none())
    );
    let fresh_store = fresh
        .desired
        .canisters
        .iter()
        .find(|canister| canister.kind == DesiredCanisterKind::Store)
        .expect("fresh Store");
    assert_eq!(fresh_store.controller_canisters, ["root-0"]);
    let fresh_pools = fresh
        .desired
        .canisters
        .iter()
        .filter(|canister| canister.kind == DesiredCanisterKind::Pool)
        .collect::<Vec<_>>();
    assert_eq!(fresh_pools.len(), 2);
    assert!(
        fresh_pools
            .iter()
            .all(|canister| canister.controller_canisters == ["root-0"])
    );
    let fresh_artifacts =
        crate::fleet_ensure::ops::resolve_desired_artifacts(&root, &fresh.desired)
            .expect("resolve fresh release artifacts");
    let fresh_plan = crate::fleet_ensure::policy::compile_plan(
        &fresh.desired,
        &fresh_artifacts,
        &[],
        &"74".repeat(32),
        &fresh.desired.fleet,
        &FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            canisters: fresh
                .desired
                .canisters
                .iter()
                .map(|canister| (canister.name.clone(), None))
                .collect(),
            ledger_fee_cycles: 100_000_000,
            operator_cycles: u128::MAX,
            protocol_ready: BTreeMap::new(),
        },
        1_800_000_000_000_000_000,
    )
    .expect("compile fresh estate creation plan");
    assert!(
        fresh_plan
            .canisters
            .iter()
            .all(|canister| canister.disposition
                == crate::fleet_ensure::model::CanisterDisposition::Create)
    );
    let created = fresh_plan
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .filter_map(|action| match action {
            EnsureAction::Create {
                ledger,
                name,
                requested_initial_cycles,
                ..
            } => Some((ledger, name, *requested_initial_cycles)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(created.len(), fresh.desired.canisters.len());
    assert!(
        created
            .iter()
            .all(|(ledger, _, _)| { *ledger == &fresh.desired.cycles_ledger })
    );
    for pool in &fresh_pools {
        let (_, _, funded) = created
            .iter()
            .find(|(_, name, _)| name.as_str() == pool.name.as_str())
            .expect("fresh pool creation is reviewed directly");
        assert_eq!(funded.to_string(), pool.initial_cycles);
    }
    let requested = created
        .iter()
        .try_fold(0_u128, |total, (_, _, amount)| total.checked_add(*amount))
        .expect("fresh funding sum");
    let expected_fees = u128::try_from(created.len())
        .expect("bounded canister count")
        .checked_mul(500_100_000_000)
        .expect("fresh fee sum");
    assert_eq!(
        fresh_plan.conservation.maximum_new_funding_cycles,
        requested
    );
    assert_eq!(
        fresh_plan.conservation.maximum_unavoidable_fee_cycles,
        expected_fees
    );
    assert_eq!(
        fresh_plan.conservation.maximum_operator_debit_cycles,
        requested + expected_fees
    );
    assert!(fresh_plan.canisters.iter().all(|canister| {
        canister
            .actions
            .iter()
            .all(|action| !matches!(action, EnsureAction::Fund { .. }))
    }));
    assert!(matches!(
        initialize_fresh_estate_seed(&FreshEstateSeedRequest {
            cycles_ledger: &mainnet_cycles_ledger(),
            management_creation_fee_cycles: 1,
            seed: &fresh_seed_path,
            source: &source_path,
        }),
        Err(FleetGenerateError::FreshSeedConflict(_))
    ));

    let source_digest = "42".repeat(32);
    let mut no_apply_desired = desired.clone();
    no_apply_desired.fleet = "retained-multi-component-no-apply".to_string();
    let mut no_apply_platform =
        RetainedEnsurePlatform::new(&no_apply_desired, &observed, &pool_one)
            .with_terminal_observation_protocol();
    let no_apply = workflow::plan(
        &root,
        &no_apply_desired,
        &source_digest,
        &no_apply_desired.fleet,
        1_800_000_000_000_000_000,
        &mut no_apply_platform,
    )
    .expect("plan public generated estate with the complete typed protocol");
    assert_eq!(
        no_apply.plan.plan_sha256,
        crate::fleet_ensure::policy::expected_plan_sha256(&no_apply.plan)
    );
    assert_eq!(no_apply.plan.plan_sha256.len(), 64);
    let provision_index = no_apply
        .plan
        .protocol_actions
        .iter()
        .position(|action| {
            matches!(
                action,
                EnsureAction::FleetProtocol { action, .. }
                    if matches!(action.as_ref(), CurrentFleetProtocolAction::ProvisionComponents { .. })
            )
        })
        .expect("Component provisioning action");
    assert!(
        no_apply.plan.protocol_actions[..provision_index]
            .iter()
            .any(|action| matches!(action, EnsureAction::FleetProtocol { .. })),
        "a non-Component typed protocol action precedes Component provisioning"
    );
    assert_eq!(
        no_apply_platform.mutations, 0,
        "public generation and planning remain effect-free"
    );

    let mut recovery_desired = desired.clone();
    recovery_desired.fleet = "retained-multi-component-recovery".to_string();
    let artifacts = crate::fleet_ensure::ops::resolve_desired_artifacts(&root, &recovery_desired)
        .expect("resolve current infrastructure artifacts");
    let mut recovery_platform =
        RetainedEnsurePlatform::new(&recovery_desired, &observed, &pool_one);
    for configured in &recovery_desired.canisters {
        let live = recovery_platform
            .live
            .get_mut(configured.principal.as_deref().expect("retained Principal"))
            .expect("retained live canister");
        if matches!(
            configured.kind,
            DesiredCanisterKind::Coordinator
                | DesiredCanisterKind::Root
                | DesiredCanisterKind::Store
        ) {
            live.module_sha256 = artifacts
                .wasm_sha256_by_canister
                .get(&configured.name)
                .cloned();
            live.reinstall_required = true;
        } else if configured.kind == DesiredCanisterKind::Pool {
            live.root_owned_lifecycle = Some(RootOwnedCanisterLifecycle::Retained);
        }
    }
    let recovery = workflow::plan(
        &root,
        &recovery_desired,
        &source_digest,
        &recovery_desired.fleet,
        1_800_000_000_000_000_050,
        &mut recovery_platform,
    )
    .expect("plan exact same-module infrastructure recovery");
    let ordered = workflow::ordered_actions(&recovery.plan);
    let store_index = ordered
        .iter()
        .position(|action| action.name() == "store-0")
        .expect("Store reinstall");
    let root_index = ordered
        .iter()
        .position(|action| action.name() == "root-0")
        .expect("Root reinstall");
    assert!(store_index < root_index);
    assert_eq!(ordered.len(), 3);
    assert!(ordered.iter().all(|action| matches!(
        action,
        EnsureAction::Install {
            mode: crate::fleet_ensure::model::InstallMode::Reinstall,
            ..
        }
    )));
    assert_eq!(recovery.plan.conservation.maximum_new_funding_cycles, 0);
    assert_eq!(recovery.plan.conservation.maximum_operator_debit_cycles, 0);

    let mut production_recovery_desired = recovery_desired.clone();
    production_recovery_desired.fleet = "retained-multi-component-live-recovery".to_string();
    let coordinator_hash = artifacts
        .wasm_sha256_by_canister
        .get("coordinator")
        .expect("Coordinator artifact");
    let root_hash = artifacts
        .wasm_sha256_by_canister
        .get("root-0")
        .expect("Root artifact");
    let store_hash = artifacts
        .wasm_sha256_by_canister
        .get("store-0")
        .expect("Store artifact");
    write_fake_icp(
        &root,
        FakeIcpFixture {
            authority: &retained_authority,
            coordinator: &coordinator,
            coordinator_module_hash: coordinator_hash,
            fleet_root: &fleet_root,
            operator: &operator,
            pool: &retained_pool,
            public_cycle_balance: Some((&pool_one, 4_800_000_000_000)),
            root_module_hash: root_hash,
            root_status_error: Some(canic_core::diagnostics::codes::STATE_CONFLICT),
            store: &store,
            store_module_hash: store_hash,
        },
    );
    let state = retained_ensure_state(&production_recovery_desired, &observed, &artifacts);
    write_state(
        &EnsurePaths::under(
            &root,
            &production_recovery_desired.environment,
            &production_recovery_desired.fleet,
        ),
        &state,
    )
    .expect("retain exact current Fleet evidence");
    let mut production_platform = IcpEnsurePlatform::new(
        production_recovery_desired.clone(),
        icp.to_str().expect("fake ICP path"),
        &root,
    );
    let production_recovery = workflow::plan(
        &root,
        &production_recovery_desired,
        &source_digest,
        &production_recovery_desired.fleet,
        1_800_000_000_000_000_100,
        &mut production_platform,
    )
    .expect("plan conflicted Root recovery from retained exact balances");
    let production_actions = workflow::ordered_actions(&production_recovery.plan);
    assert_eq!(
        production_actions
            .iter()
            .filter(|action| matches!(
                action,
                EnsureAction::Install {
                    mode: crate::fleet_ensure::model::InstallMode::Reinstall,
                    ..
                }
            ))
            .count(),
        3,
    );
    assert!(matches!(
        production_actions.get(1),
        Some(EnsureAction::FleetProtocol { action, .. })
            if matches!(action.as_ref(), CurrentFleetProtocolAction::AdoptStore { .. })
    ));
    assert_eq!(
        production_recovery
            .plan
            .conservation
            .maximum_new_funding_cycles,
        0
    );
    assert_eq!(
        production_recovery
            .plan
            .conservation
            .maximum_operator_debit_cycles,
        0
    );
    let pool_names = production_recovery_desired
        .canisters
        .iter()
        .filter(|canister| canister.kind == DesiredCanisterKind::Pool)
        .map(|canister| canister.name.as_str())
        .collect::<BTreeSet<_>>();
    let retained_assets = production_recovery
        .plan
        .canisters
        .iter()
        .filter(|canister| pool_names.contains(canister.name.as_str()))
        .map(|canister| canister.observed_cycles)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        retained_assets,
        BTreeSet::from([4_800_000_000_000, 5_000_000_000_000])
    );

    let mut pending_pool = retained_pool;
    let pending = pending_pool
        .entries
        .iter_mut()
        .find(|asset| asset.canister_id.to_text() == pool_two)
        .expect("idle retained pool asset");
    pending.status = CanisterPoolAssetStatus::PendingReset;
    pending.cycles = Cycles::new(0);
    let mut pending_desired = recovery_desired.clone();
    pending_desired.fleet = "retained-multi-component-pending-reset".to_string();
    write_fake_icp(
        &root,
        FakeIcpFixture {
            authority: &retained_authority,
            coordinator: &coordinator,
            coordinator_module_hash: coordinator_hash,
            fleet_root: &fleet_root,
            operator: &operator,
            pool: &pending_pool,
            public_cycle_balance: Some((&pool_one, 4_800_000_000_000)),
            root_module_hash: root_hash,
            root_status_error: None,
            store: &store,
            store_module_hash: store_hash,
        },
    );
    write_state(
        &EnsurePaths::under(&root, &pending_desired.environment, &pending_desired.fleet),
        &retained_ensure_state(&pending_desired, &observed, &artifacts),
    )
    .expect("retain exact pre-reset pool balances");
    let mut pending_platform = IcpEnsurePlatform::new(
        pending_desired.clone(),
        icp.to_str().expect("fake ICP path"),
        &root,
    );
    let pending_plan = workflow::plan(
        &root,
        &pending_desired,
        &source_digest,
        &pending_desired.fleet,
        1_800_000_000_000_000_200,
        &mut pending_platform,
    )
    .expect("plan pending-reset pool from retained exact balance");
    let retained_pending = pending_plan
        .plan
        .canisters
        .iter()
        .find(|canister| canister.principal.as_deref() == Some(pool_two.as_str()))
        .expect("pending retained asset plan");
    assert_eq!(retained_pending.observed_cycles, 5_000_000_000_000);
    assert!(retained_pending.actions.is_empty());
    assert_eq!(pending_plan.plan.conservation.maximum_new_funding_cycles, 0);

    let mut drifted_desired = pending_desired.clone();
    drifted_desired.fleet = "retained-controller-drift".to_string();
    drifted_desired
        .canisters
        .iter_mut()
        .find(|canister| canister.kind == DesiredCanisterKind::Root)
        .expect("desired Root")
        .controllers = vec![principal_text(99)];
    write_state(
        &EnsurePaths::under(&root, &drifted_desired.environment, &drifted_desired.fleet),
        &retained_ensure_state(&drifted_desired, &observed, &artifacts),
    )
    .expect("retain drift-negative evidence");
    let mut drifted_platform = IcpEnsurePlatform::new(
        drifted_desired.clone(),
        icp.to_str().expect("fake ICP path"),
        &root,
    );
    let error = workflow::plan(
        &root,
        &drifted_desired,
        &source_digest,
        &drifted_desired.fleet,
        1_800_000_000_000_000_300,
        &mut drifted_platform,
    )
    .expect_err("Root controller drift must reject retained balance evidence");
    assert!(
        matches!(
            &error,
            workflow::EnsureWorkflowError::Platform(IcpEnsurePlatformError::CurrentProtocol(_))
        ),
        "unexpected controller-drift error: {error:?}"
    );

    let mut platform = RetainedEnsurePlatform::new(&desired, &observed, &pool_one);
    let planned = workflow::plan(
        &root,
        &desired,
        &source_digest,
        &desired.fleet,
        1_800_000_000_000_000_000,
        &mut platform,
    )
    .expect("plan retained estate");
    let applied = workflow::apply(
        &root,
        &desired,
        &source_digest,
        &desired.fleet,
        &planned.plan.plan_sha256,
        &mut platform,
    )
    .expect("apply retained estate");
    assert!(applied.terminal);
    assert!(planned.plan.canisters.iter().all(|canister| {
        canister
            .actions
            .iter()
            .all(|action| !matches!(action, EnsureAction::Create { .. }))
    }));
    assert!(planned.plan.canisters.iter().all(|canister| {
        canister
            .actions
            .iter()
            .all(|action| !matches!(action, EnsureAction::Fund { .. }))
    }));
    assert_eq!(
        platform.mutations, 3,
        "only retained infrastructure reinstalls"
    );
    assert_eq!(platform.total_cycles(), 319_900_000_000_000);

    let second = workflow::plan(
        &root,
        &desired,
        &source_digest,
        &desired.fleet,
        1_800_000_000_000_000_100,
        &mut platform,
    )
    .expect("plan converged retained estate");
    assert!(
        second
            .plan
            .canisters
            .iter()
            .all(|canister| canister.actions.is_empty())
    );
    let replay = workflow::apply(
        &root,
        &desired,
        &source_digest,
        &desired.fleet,
        &second.plan.plan_sha256,
        &mut platform,
    )
    .expect("effect-free replay");
    assert!(replay.terminal);
    assert_eq!(replay.effects_applied, 0);
    assert_eq!(platform.mutations, 3);
    let current =
        crate::fleet_ensure::resolve_current_fleet(&root, &desired.environment, &desired.fleet)
            .expect("resolve terminal retained Fleet");
    let workload = current
        .registry
        .entries
        .iter()
        .find(|entry| entry.pid == pool_one)
        .expect("pool identity becomes terminal workload");
    assert_eq!(workload.role.as_deref(), Some("app"));
    assert!(workload.module_hash.is_some());
    fs::remove_dir_all(root).expect("remove retained journey root");
}

struct RetainedEnsurePlatform {
    desired: DesiredFleet,
    ledger_fee_cycles: u128,
    live: BTreeMap<String, LiveCanister>,
    mutations: u32,
    terminal_observation_protocol: bool,
}

impl RetainedEnsurePlatform {
    fn new(
        desired: &DesiredFleet,
        observed: &BTreeMap<String, ObservedCanister>,
        workload: &str,
    ) -> Self {
        let live = desired
            .canisters
            .iter()
            .map(|canister| {
                let principal = canister.principal.clone().expect("retained Principal");
                let observed = observed.get(&principal).expect("retained observation");
                let root_owned_lifecycle = (canister.kind == DesiredCanisterKind::Pool).then_some(
                    if principal == workload {
                        RootOwnedCanisterLifecycle::Workload
                    } else {
                        RootOwnedCanisterLifecycle::Idle
                    },
                );
                let status = if canister.kind == DesiredCanisterKind::Pool {
                    if principal == workload {
                        CanisterRuntimeStatus::Running
                    } else {
                        CanisterRuntimeStatus::Stopped
                    }
                } else {
                    CanisterRuntimeStatus::Running
                };
                (
                    principal.clone(),
                    LiveCanister {
                        canister_version: Some(1),
                        controllers: canister.controllers.clone(),
                        cycles: observed.cycles,
                        module_sha256: canister.wasm.as_ref().map(|_| "00".repeat(32)),
                        principal,
                        reinstall_required: false,
                        root_owned_lifecycle,
                        status,
                    },
                )
            })
            .collect();
        Self {
            desired: desired.clone(),
            ledger_fee_cycles: desired.ledger_fee_cycles.parse().expect("ledger fee"),
            live,
            mutations: 0,
            terminal_observation_protocol: false,
        }
    }

    fn with_terminal_observation_protocol(mut self) -> Self {
        self.terminal_observation_protocol = true;
        self
    }

    fn total_cycles(&self) -> u128 {
        self.live.values().map(|canister| canister.cycles).sum()
    }
}

impl EnsurePlatform for RetainedEnsurePlatform {
    type Error = io::Error;

    fn bind_reviewed_desired(&mut self, desired: &DesiredFleet) -> Result<(), Self::Error> {
        self.desired = desired.clone();
        Ok(())
    }

    fn observe(
        &mut self,
        _operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        Ok(FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            canisters: self
                .desired
                .canisters
                .iter()
                .map(|canister| {
                    (
                        canister.name.clone(),
                        canister
                            .principal
                            .as_ref()
                            .and_then(|principal| self.live.get(principal).cloned()),
                    )
                })
                .collect(),
            ledger_fee_cycles: self.ledger_fee_cycles,
            operator_cycles: 1_000_000_000_000_000,
            protocol_ready: BTreeMap::new(),
        })
    }

    fn protocol_actions(
        &mut self,
        operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Vec<EnsureAction>, Self::Error> {
        if !self.terminal_observation_protocol {
            return Ok(Vec::new());
        }
        Ok(terminal_observation_protocol_actions(
            &self.desired,
            operation_id,
        ))
    }

    fn observe_effect(
        &mut self,
        _operation_id: &str,
        action: &EnsureAction,
        _record: &EffectRecord,
        _state: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error> {
        let EnsureAction::Install {
            principal,
            wasm_sha256,
            ..
        } = action
        else {
            return Err(io::Error::other("retained journey permits only reinstall"));
        };
        let applied = self
            .live
            .get(principal)
            .and_then(|canister| canister.module_sha256.as_deref())
            == Some(wasm_sha256);
        Ok(EffectObservation {
            applied,
            progress_identity: format!("install:{principal}:{applied}"),
            retry: EffectRetry::None,
        })
    }

    fn terminal_inventory(
        &mut self,
        _operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<crate::fleet_ensure::ops::TerminalFleetInventory, Self::Error> {
        let workload = self
            .live
            .values()
            .find(|canister| {
                canister.root_owned_lifecycle == Some(RootOwnedCanisterLifecycle::Workload)
            })
            .expect("retained workload");
        let configured = self
            .desired
            .canisters
            .iter()
            .find(|canister| canister.principal.as_deref() == Some(&workload.principal))
            .expect("configured workload identity");
        let parent = configured
            .parent
            .as_ref()
            .and_then(|name| {
                self.desired
                    .canisters
                    .iter()
                    .find(|canister| canister.name == *name)
            })
            .and_then(|canister| canister.principal.clone())
            .expect("workload Root");
        Ok(crate::fleet_ensure::ops::TerminalFleetInventory {
            active_registry: None,
            controlled_cycles_by_principal: BTreeMap::from([(
                workload.principal.clone(),
                workload.cycles,
            )]),
            entries: vec![crate::registry::RegistryEntry {
                module_hash: Some("71".repeat(32)),
                parent_pid: Some(parent),
                pid: workload.principal.clone(),
                protocol_binding: Some(crate::protocol_binding::RegistryProtocolBinding {
                    release_identity: "current".to_string(),
                    role: CanisterRole::from("app"),
                    capabilities: BTreeSet::new(),
                    candid_sha256: [72; 32],
                    protocol_profile_digest: ProtocolProfileDigest::from_bytes([73; 32]),
                }),
                role: Some("app".to_string()),
            }],
        })
    }

    fn action_cycles(
        &mut self,
        action: &EnsureAction,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        let EnsureAction::Install { principal, .. } = action else {
            return Ok(None);
        };
        Ok(self.live.get(principal).map(|canister| canister.cycles))
    }

    fn action_destination_cycles(
        &mut self,
        _action: &EnsureAction,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        Ok(None)
    }

    fn apply(
        &mut self,
        _operation_id: &str,
        action: &EnsureAction,
        _record: &EffectRecord,
        _state: &FleetEnsureStateRecord,
    ) -> Result<EffectOutcome, Self::Error> {
        let EnsureAction::Install {
            principal,
            wasm_sha256,
            ..
        } = action
        else {
            return Err(io::Error::other("retained journey permits only reinstall"));
        };
        let canister = self
            .live
            .get_mut(principal)
            .ok_or_else(|| io::Error::other("missing retained canister"))?;
        canister.module_sha256 = Some(wasm_sha256.clone());
        self.mutations += 1;
        Ok(EffectOutcome {
            created_principal: None,
            post_cycles: Some(canister.cycles),
            receipt: Some(format!("installed:{principal}")),
        })
    }
}

fn terminal_observation_protocol_actions(
    desired: &DesiredFleet,
    operation_id: &str,
) -> Vec<EnsureAction> {
    let bootstrap = desired.bootstrap.as_ref().expect("generated bootstrap");
    let protocol = desired.protocol.as_ref().expect("generated protocol");
    let coordinator = desired
        .canisters
        .iter()
        .find(|canister| canister.kind == DesiredCanisterKind::Coordinator)
        .and_then(|canister| canister.principal.clone())
        .expect("generated Coordinator Principal");
    let store = desired
        .canisters
        .iter()
        .find(|canister| canister.kind == DesiredCanisterKind::Store)
        .and_then(|canister| canister.principal.clone())
        .expect("generated Store Principal");
    let operation_id = canic_core::cdk::utils::hash::decode_hex(operation_id)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .expect("Fleet ensure operation ID");
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: bootstrap.canonical_network_id,
            fleet_id: bootstrap.fleet_id,
        },
        app: bootstrap.app.clone(),
    };
    let registry_authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet.clone(),
            coordinator_subnet: bootstrap.coordinator_subnet,
            coordinator: coordinator.parse().expect("Coordinator Principal"),
        },
        epoch: 1,
    };
    vec![
        EnsureAction::FleetProtocol {
            action: Box::new(CurrentFleetProtocolAction::PrepareStoreChunkSet {
                request: TemplateChunkSetPrepareInput {
                    template_id: TemplateId::from("root"),
                    version: TemplateVersion::from("current"),
                    payload_hash: vec![1; 32],
                    payload_size_bytes: 1,
                    chunk_hashes: vec![vec![2; 32]],
                },
            }),
            candid: protocol.store_candid.clone(),
            candid_sha256: "11".repeat(32),
            maximum_execution_burn_cycles: 1,
            name: "store-chunk-preparation".to_string(),
            principal: store,
        },
        EnsureAction::FleetProtocol {
            action: Box::new(CurrentFleetProtocolAction::ProvisionComponents {
                request: FleetComponentProvisioningPrepareRequest {
                    operation_id,
                    plan: FleetComponentProvisioningPlan {
                        fleet,
                        fleet_registry: FleetRegistryVersion {
                            authority: registry_authority,
                            revision: 1,
                            content_hash: [3; 32],
                        },
                        configuration_digest: ComponentDeploymentConfigurationDigest::from_bytes(
                            [4; 32],
                        ),
                        operation: FleetComponentProvisioningOperation::FreshInstall,
                        directory_confirmation_roots: Vec::new(),
                        batches: Vec::new(),
                    },
                },
                plan_hash: [5; 32],
            }),
            candid: protocol.coordinator_candid.clone(),
            candid_sha256: "12".repeat(32),
            maximum_execution_burn_cycles: 1,
            name: "fleet-component-provisioning".to_string(),
            principal: coordinator,
        },
    ]
}

fn retained_ensure_state(
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
    artifacts: &crate::fleet_ensure::model::DesiredFleetArtifacts,
) -> FleetEnsureStateRecord {
    FleetEnsureStateRecord {
        active_registry: None,
        completed_reinstalls: BTreeMap::new(),
        fleet: desired.fleet.clone(),
        pending_principals: BTreeMap::new(),
        principals: desired
            .canisters
            .iter()
            .filter_map(|canister| {
                canister
                    .principal
                    .clone()
                    .map(|principal| (canister.name.clone(), principal))
            })
            .collect(),
        retained_cycles_by_principal: observed
            .iter()
            .map(|(principal, canister)| (principal.clone(), canister.cycles))
            .collect(),
        schema_version: crate::fleet_ensure::model::FLEET_ENSURE_SCHEMA_VERSION,
        topology: desired
            .canisters
            .iter()
            .map(|canister| {
                (
                    canister.name.clone(),
                    crate::fleet_ensure::model::FleetEnsureTopologyRecord {
                        kind: canister.kind,
                        module_hash: artifacts
                            .wasm_sha256_by_canister
                            .get(&canister.name)
                            .cloned(),
                        parent: canister.parent.clone(),
                        protocol_binding: canister.protocol_binding.clone(),
                        role: canister
                            .protocol_binding
                            .as_ref()
                            .map(|binding| binding.role.to_string()),
                    },
                )
            })
            .collect(),
    }
}

fn multi_component_config() -> &'static str {
    r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"
fleet_admission = true

[component_specs.app]
component_role = "app"
maximum_instances = 1
initial_cycles = "5T"

[component_groups.app.components.app]
component_spec = "app"

[component_group_deployments.app]
component_group = "app"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#
}

fn multi_component_source(
    operator: &str,
    coordinator_subnet: &str,
    placement: &str,
) -> FleetSource {
    FleetSource {
        schema_version: 1,
        funding_profile: FleetFundingProfile::PreviewMultiSubnet,
        operator: operator.to_string(),
        admission: AdmissionSource {
            principals: vec![operator.to_string()],
        },
        coordinator: CoordinatorSource {
            subnet: ExplicitSubnetSource {
                kind: "explicit".to_string(),
                subnet: coordinator_subnet.to_string(),
                acknowledge_fiduciary_cost: false,
            },
            creation_funding: CyclesCreationSource {
                kind: "cycles".to_string(),
                cycles: Cycles::new(270_000_000_000_000),
            },
            root_funding: CoordinatorFundingSource {
                minimum_reserve_cycles: Cycles::new(210_000_000_000_000),
                window_secs: 7_776_000,
                maximum_cycles: Cycles::new(30_000_000_000_000),
                maximum_automatic_grants: 2,
                maximum_automatic_cycles: Cycles::new(60_000_000_000_000),
            },
        },
        fleet_subnet_roots: vec![RootSource {
            placement_subnet: placement.to_string(),
            acknowledge_fiduciary_cost: false,
            component_group_placements: BTreeMap::from([(
                "app".parse().expect("deployment"),
                vec![0],
            )]),
            component_admissions: BTreeMap::from([("app".parse().expect("Component Spec"), 1)]),
            canister_pool: PoolSource {
                minimum_size: 2,
                maximum_size: 2,
                canister_cycles: Cycles::new(5_000_000_000_000),
                imports: Vec::new(),
            },
            root_funding: RootFundingSource {
                request_threshold: Cycles::new(10_000_000_000_000),
                target_balance: Cycles::new(30_000_000_000_000),
                cooldown_secs: 2_592_000,
                window_secs: 7_776_000,
                maximum_cycles: Cycles::new(30_000_000_000_000),
                maximum_automatic_grants: 2,
                maximum_automatic_cycles: Cycles::new(60_000_000_000_000),
            },
            limits: LimitsSource {
                maximum_component_instances: 1,
                maximum_registry_bytes: 16_777_216,
                maximum_wasm_store_bytes: 40_000_000,
                maximum_group_placements: 1,
                cycles_funding: CyclesFundingSource {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(15_000_000_000_000),
                },
            },
            root_creation_funding: CyclesCreationSource {
                kind: "cycles".to_string(),
                cycles: Cycles::new(30_000_000_000_000),
            },
            wasm_store_creation_funding: CyclesCreationSource {
                kind: "cycles".to_string(),
                cycles: Cycles::new(10_000_000_000_000),
            },
        }],
    }
}

fn multi_component_source_toml(
    operator: &str,
    coordinator_subnet: &str,
    placement: &str,
) -> String {
    format!(
        r#"
schema_version = 1
funding_profile = "preview_multi_subnet"
operator = "{operator}"

[admission]
principals = ["{operator}"]

[coordinator.subnet]
kind = "explicit"
subnet = "{coordinator_subnet}"
acknowledge_fiduciary_cost = false

[coordinator.creation_funding]
kind = "cycles"
cycles = "270T"

[coordinator.root_funding]
minimum_reserve_cycles = "210T"
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 2
maximum_automatic_cycles = "60T"

[[fleet_subnet_roots]]
placement_subnet = "{placement}"
acknowledge_fiduciary_cost = false
component_admissions = {{ app = 1 }}

[fleet_subnet_roots.component_group_placements]
app = [0]

[fleet_subnet_roots.canister_pool]
minimum_size = 2
maximum_size = 2
canister_cycles = "5T"

[fleet_subnet_roots.root_funding]
request_threshold = "10T"
target_balance = "30T"
cooldown_secs = 2592000
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 2
maximum_automatic_cycles = "60T"

[fleet_subnet_roots.limits]
maximum_component_instances = 1
maximum_registry_bytes = 16777216
maximum_wasm_store_bytes = 40000000
maximum_group_placements = 1

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "15T"

[fleet_subnet_roots.root_creation_funding]
kind = "cycles"
cycles = "30T"

[fleet_subnet_roots.wasm_store_creation_funding]
kind = "cycles"
cycles = "10T"
"#
    )
}

fn retained_estate_seed_toml(
    fleet_id: canic_core::ids::FleetId,
    coordinator: &str,
    placement: &str,
    root: &str,
    store: &str,
    pools: [&str; 2],
) -> String {
    format!(
        r#"
schema_version = 1
fleet_id = "{fleet_id}"
coordinator = "{coordinator}"
cycles_ledger = "{}"

[[roots]]
placement_subnet = "{placement}"
root = "{root}"
store = "{store}"
pool_imports = ["{}", "{}"]
"#,
        mainnet_cycles_ledger(),
        pools[0],
        pools[1],
    )
}

fn enroll_test_network(root: &Path) -> canic_core::ids::CanonicalNetworkId {
    let mut root_key = vec![
        0x30, 0x81, 0x82, 0x30, 0x1d, 0x06, 0x0d, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c,
        0x05, 0x03, 0x01, 0x02, 0x01, 0x06, 0x0c, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c,
        0x05, 0x03, 0x02, 0x01, 0x03, 0x61, 0x00,
    ];
    root_key.extend_from_slice(&[9; 96]);
    let path = root.join("root-key.der");
    fs::write(&path, &root_key).expect("write local root key");
    enroll_network(NetworkEnrollmentOptions {
        workspace_root: root,
        environment: "local",
        root_key: &path,
        fingerprint: &sha256_hex(&root_key),
    })
    .expect("enroll local network")
    .canonical_network_id
}

fn persist_test_release_authority(
    root: &Path,
    config: &AppConfigSnapshot,
    release_build_id: ReleaseBuildId,
) {
    let directory = root
        .join(".canic/release-builds")
        .join(release_build_id.to_string());
    fs::create_dir_all(&directory).expect("create release authority directory");
    let artifacts = infrastructure_artifacts(root, release_build_id);
    let infrastructure = CanicInfrastructureArtifactManifest {
        release_build_id,
        entries: artifacts,
    };
    fs::write(
        directory.join("infrastructure-artifact-manifest.json"),
        infrastructure
            .canonical_bytes()
            .expect("canonical infrastructure manifest"),
    )
    .expect("write infrastructure manifest");

    let role = CanisterRole::from("app");
    let wasm = [b"\0asm\x01\0\0\0".as_slice(), &[74]].concat();
    let wasm_gz = gzip(&wasm);
    let application = ApplicationArtifactUnion::compile(
        config.component_topology(),
        release_build_id,
        &[ApplicationArtifactBuildTarget {
            role: role.clone(),
            package: "app".to_string(),
            wasm_relative_path: "artifacts/app.wasm".to_string(),
            wasm_gz_relative_path: "artifacts/app.wasm.gz".to_string(),
        }],
        &[ApplicationArtifactBuildOutput {
            role,
            package: "app".to_string(),
            release_build_id,
            wasm_relative_path: "artifacts/app.wasm".to_string(),
            wasm,
            wasm_gz_relative_path: "artifacts/app.wasm.gz".to_string(),
            wasm_gz,
            candid_sha256: [75; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([76; 32]),
        }],
    )
    .expect("compile application union");
    fs::write(
        directory.join("application-artifact-union.json"),
        application
            .canonical_bytes(config.component_topology())
            .expect("canonical application union"),
    )
    .expect("write application union");

    let current = CurrentReleaseSetManifest {
        application_artifact_union_sha256: application
            .digest(config.component_topology())
            .expect("application union digest"),
        build_network: canic_core::ids::BuildNetwork::Local,
        infrastructure_artifact_manifest_sha256: infrastructure
            .digest()
            .expect("infrastructure manifest digest"),
        release_build_id,
        schema_version: CurrentReleaseSetManifest::SCHEMA_VERSION,
    };
    let current_path = directory.join("current-release-set-manifest.json");
    fs::write(
        &current_path,
        current.canonical_bytes().expect("current release manifest"),
    )
    .expect("write current release manifest");
    finalize_release_build_from_manifest(root, release_build_id, &current_path)
        .expect("finalize test release authority");
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(bytes).expect("write gzip");
    encoder.finish().expect("finish gzip")
}

#[expect(
    clippy::too_many_arguments,
    reason = "the retained observation fixture binds every independent Fleet authority identity"
)]
fn retained_root_authority(
    canonical_network_id: canic_core::ids::CanonicalNetworkId,
    app: canic_core::ids::AppId,
    fleet_id: canic_core::ids::FleetId,
    source: &FleetSource,
    planned: &crate::component_topology::PlannedFleetSubnetRootTopology,
    operator: &str,
    coordinator: &str,
    coordinator_subnet: &str,
    root: &str,
    store: &str,
    release_build_id: ReleaseBuildId,
) -> FleetSubnetRootAuthority {
    let coordinator = parse_principal("Coordinator", coordinator).expect("Coordinator");
    let root = parse_principal("Root", root).expect("Root");
    let store = parse_principal("Store", store).expect("Store");
    let placement_subnet = planned.placement_subnet;
    let registry = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id,
                    fleet_id,
                },
                app,
            },
            coordinator_subnet: parse_subnet("Coordinator", coordinator_subnet)
                .expect("Coordinator Subnet"),
            coordinator,
        },
        epoch: 1,
    };
    let root_source = source.fleet_subnet_roots.first().expect("Root source");
    let mut retained_limits = root_limits(root_source);
    retained_limits.canister_pool.canister_cycles = Cycles::new(2_000_000_000_000);
    let binding = FleetSubnetRootBinding {
        authority: registry.clone(),
        placement_subnet,
        fleet_subnet_root: root,
        component_admissions: planned.component_admissions.clone(),
        component_topology_digest: planned.component_topology_digest,
        limits: retained_limits,
        funding: root_funding(source.funding_profile, &root_source.root_funding),
    };
    FleetSubnetRootAuthority {
        binding,
        initial_release_set: FleetSubnetRootReleaseSet {
            release_build_id,
            manifest_digest: ReleaseSetDigest::from_bytes([78; 32]),
        },
        expected_module_hash: [79; 32],
        wasm_store_authority: FleetSubnetWasmStoreAuthority {
            authority: registry,
            placement_subnet,
            fleet_subnet_root: root,
            wasm_store: store,
            installation_controller: parse_principal("operator", operator).expect("operator"),
            release_build_id,
            wasm_module_hash: [80; 32],
        },
    }
}

fn retained_pool_response(
    config: FleetSubnetCanisterPoolConfig,
    store: &str,
    workload: &str,
    idle: &str,
) -> CanisterPoolResponse {
    let claim = canic_core::dto::pool::CanisterPoolClaim {
        component: canic_core::ids::ComponentInstanceId::from_generated_bytes([81; 32]),
        operation_id: [82; 32],
    };
    CanisterPoolResponse {
        config,
        tracked: 3,
        store: 1,
        store_deletion_pending: 0,
        pooled: 1,
        workload: 1,
        surplus: 0,
        ready: 1,
        pending_reset: 0,
        claimed: 0,
        recycling: 0,
        recovering_ledger: 0,
        handing_off: 0,
        failed: 0,
        completed_handoffs: 0,
        pending_creation: None,
        pending_handoff: None,
        entries: vec![
            CanisterPoolAsset {
                canister_id: parse_principal("Store", store).expect("Store"),
                cycles: Cycles::new(10_000_000_000_000),
                origin: CanisterPoolAssetOrigin::InfrastructureStore,
                status: CanisterPoolAssetStatus::Store,
                added_at_ns: 1,
                updated_at_ns: 1,
            },
            CanisterPoolAsset {
                canister_id: parse_principal("workload", workload).expect("workload"),
                cycles: Cycles::new(4_900_000_000_000),
                origin: CanisterPoolAssetOrigin::Imported,
                status: CanisterPoolAssetStatus::Workload { claim },
                added_at_ns: 2,
                updated_at_ns: 3,
            },
            CanisterPoolAsset {
                canister_id: parse_principal("idle pool", idle).expect("idle pool"),
                cycles: Cycles::new(5_000_000_000_000),
                origin: CanisterPoolAssetOrigin::Imported,
                status: CanisterPoolAssetStatus::Ready,
                added_at_ns: 4,
                updated_at_ns: 5,
            },
        ],
        next_start_after: None,
    }
}

struct FakeIcpFixture<'a> {
    authority: &'a FleetSubnetRootAuthority,
    coordinator: &'a str,
    coordinator_module_hash: &'a str,
    fleet_root: &'a str,
    operator: &'a str,
    pool: &'a CanisterPoolResponse,
    public_cycle_balance: Option<(&'a str, u128)>,
    root_module_hash: &'a str,
    root_status_error: Option<canic_core::diagnostics::RegisteredDiagnosticCode>,
    store: &'a str,
    store_module_hash: &'a str,
}

#[cfg(unix)]
#[expect(
    clippy::too_many_lines,
    reason = "one process-backed fixture keeps every accepted fake ICP command visible"
)]
fn write_fake_icp(root: &Path, fixture: FakeIcpFixture<'_>) -> PathBuf {
    let FakeIcpFixture {
        authority,
        coordinator,
        coordinator_module_hash,
        fleet_root,
        operator,
        pool,
        public_cycle_balance,
        root_module_hash,
        root_status_error,
        store,
        store_module_hash,
    } = fixture;
    let executable = root.join("fake-icp");
    let counter = root.join("root-status-count");
    let coordinator_status = canister_status_json(
        coordinator,
        operator,
        coordinator_module_hash.to_string(),
        270_000_000_000_000,
    );
    let root_status = canister_status_json(
        fleet_root,
        operator,
        root_module_hash.to_string(),
        30_000_000_000_000,
    );
    let store_status = canister_status_json(
        store,
        operator,
        store_module_hash.to_string(),
        10_000_000_000_000,
    );
    let authority_response = candid_response_json(&Ok::<_, canic_core::dto::error::Error>(
        RootEstateStatusResponse::FleetAuthority(Box::new(authority.clone())),
    ));
    let pool_response = root_status_error.map_or_else(
        || {
            candid_response_json(&Ok::<_, canic_core::dto::error::Error>(
                RootEstateStatusResponse::Pool(Box::new(pool.clone())),
            ))
        },
        |code| {
            candid_response_json(&Err::<RootEstateStatusResponse, _>(
                canic_core::dto::error::Error::from_registered(code),
            ))
        },
    );
    let ledger_response = candid_response_json(&Nat::from(100_000_000_u64));
    let public_cycle_case = public_cycle_balance.map_or_else(String::new, |(canister, cycles)| {
        let response = candid_response_json(&Ok::<_, canic_core::dto::error::Error>(
            FixtureManagedStatusResponse::CycleBalance(
                canic_core::dto::role::CycleBalanceStatusResponse { cycles },
            ),
        ));
        format!(
            r#"if [ "$1" = "canister" ] && [ "$2" = "call" ] && [ "$3" = "{canister}" ] && [ "$4" = "canic_status" ]; then
  printf '%s\n' '{response}'
  exit 0
fi
"#
        )
    });
    let script = format!(
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf '%s\n' 'icp 1.2.0'
  exit 0
fi
while [ "$1" = "--project-root-override" ] || [ "$1" = "--identity-password-file" ]; do
  shift 2
done
if [ "$1" = "identity" ] && [ "$2" = "principal" ]; then
  printf '%s\n' '{operator}'
  exit 0
fi
if [ "$1" = "cycles" ] && [ "$2" = "balance" ]; then
  printf '%s\n' '{{"balance":"500000000000000 cycles"}}'
  exit 0
fi
if [ "$1" = "canister" ] && [ "$2" = "status" ]; then
  if [ "$3" = "{coordinator}" ]; then
    printf '%s\n' '{coordinator_status}'
    exit 0
  fi
  if [ "$3" = "{fleet_root}" ]; then
    printf '%s\n' '{root_status}'
    exit 0
  fi
  if [ "$3" = "{store}" ]; then
    printf '%s\n' '{store_status}'
    exit 0
  fi
fi
if [ "$1" = "canister" ] && [ "$2" = "call" ]; then
  {public_cycle_case}
  if [ "$4" = "icrc1_fee" ]; then
    printf '%s\n' '{ledger_response}'
    exit 0
  fi
  if [ "$4" = "canic_status" ]; then
    count=0
    if [ -f "{counter}" ]; then
      count=$(sed -n '1p' "{counter}")
    fi
    if [ "$count" = "0" ]; then
      printf '%s\n' '1' > "{counter}"
      printf '%s\n' '{authority_response}'
    else
      printf '%s\n' '{pool_response}'
    fi
    exit 0
  fi
fi
printf '%s\n' 'unsupported fake ICP command' >&2
exit 42
"#,
        counter = counter.display(),
    );
    fs::write(&executable, script).expect("write fake ICP executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
        .expect("make fake ICP executable runnable");
    executable
}

#[derive(candid::CandidType)]
enum FixtureManagedStatusResponse {
    CycleBalance(canic_core::dto::role::CycleBalanceStatusResponse),
}

#[cfg(not(unix))]
fn write_fake_icp(_root: &Path, _fixture: FakeIcpFixture<'_>) -> PathBuf {
    panic!("public generator fixture requires a Unix fake ICP executable")
}

fn canister_status_json(
    canister: &str,
    controller: &str,
    module_hash: String,
    cycles: u128,
) -> String {
    serde_json::json!({
        "id": canister,
        "name": null,
        "status": "running",
        "settings": { "controllers": [controller] },
        "version": 1,
        "module_hash": module_hash,
        "memory_size": null,
        "cycles": cycles.to_string(),
        "reserved_cycles": null,
        "idle_cycles_burned_per_day": null
    })
    .to_string()
}

fn candid_response_json<T: candid::CandidType>(value: &T) -> String {
    let bytes = candid::encode_one(value).expect("encode fake ICP response");
    serde_json::json!({ "response_bytes": hex_bytes(bytes) }).to_string()
}

fn infrastructure_artifacts(
    root: &Path,
    release_build_id: ReleaseBuildId,
) -> Vec<CanicInfrastructureArtifactEntry> {
    [
        CanicInfrastructureRole::FleetCoordinator,
        CanicInfrastructureRole::FleetSubnetRoot,
        CanicInfrastructureRole::PoolLedgerRecovery,
        CanicInfrastructureRole::WasmStore,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, role)| {
        let marker = u8::try_from(index).expect("three infrastructure artifacts");
        let artifact_name = role.protocol_role_name();
        let path = format!("artifacts/{artifact_name}/{artifact_name}.wasm");
        let bytes = [b"\0asm\x01\0\0\0".as_slice(), &[marker]].concat();
        let absolute = root.join(&path);
        fs::create_dir_all(absolute.parent().expect("artifact parent"))
            .expect("create artifact parent");
        fs::write(&absolute, &bytes).expect("write artifact");
        fs::write(absolute.with_extension("did"), b"service : {};").expect("write Candid");
        CanicInfrastructureArtifactEntry {
            role,
            package: role.as_str().to_string(),
            protocol_release_identity: "current".to_string(),
            protocol_role: CanisterRole::owned(role.protocol_role_name().to_string()),
            protocol_capabilities: BTreeSet::<RoleCapabilityKey>::new(),
            release_build_id,
            wasm_relative_path: path,
            wasm_size_bytes: bytes.len() as u64,
            wasm_sha256_hex: canic_core::cdk::utils::hash::sha256_hex(&bytes),
            wasm_gz_relative_path: format!("artifacts/{}.wasm.gz", role.as_str()),
            wasm_gz_size_bytes: 1,
            wasm_gz_sha256_hex: "00".repeat(32),
            candid_sha256: Sha256::digest(b"service : {};").into(),
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([marker; 32]),
        }
    })
    .collect()
}

#[test]
fn candid_sidecar_uses_the_manifest_bound_wasm_basename() {
    let root = temp_dir("canic-generate-candid-sidecar");
    let release_build_id = "91".repeat(32).parse().expect("release build ID");
    let artifacts = infrastructure_artifacts(&root, release_build_id);
    let root_artifact = artifacts
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .expect("Root artifact");

    assert_eq!(root_artifact.wasm_relative_path, "artifacts/root/root.wasm");
    assert_eq!(
        candid_sidecar(&root, root_artifact).expect("resolve Root sidecar"),
        "artifacts/root/root.did"
    );

    let sidecar = root.join("artifacts/root/root.did");
    fs::rename(&sidecar, sidecar.with_file_name("renamed.did")).expect("rename sidecar");
    assert!(matches!(
        candid_sidecar(&root, root_artifact),
        Err(FleetGenerateError::Candid(reason)) if reason.contains("is missing")
    ));
    fs::write(&sidecar, b"service : { changed : () -> (); };").expect("write changed sidecar");
    assert!(matches!(
        candid_sidecar(&root, root_artifact),
        Err(FleetGenerateError::Candid(reason)) if reason.contains("digest differs")
    ));

    fs::remove_dir_all(root).expect("remove fixture root");
}

#[cfg(unix)]
#[test]
fn candid_sidecar_rejects_a_symbolic_link() {
    let root = temp_dir("canic-generate-candid-sidecar-link");
    let release_build_id = "92".repeat(32).parse().expect("release build ID");
    let artifacts = infrastructure_artifacts(&root, release_build_id);
    let root_artifact = artifacts
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .expect("Root artifact");
    let sidecar = root.join("artifacts/root/root.did");
    let target = root.join("artifacts/root/target.did");
    fs::rename(&sidecar, &target).expect("move real sidecar");
    symlink(&target, &sidecar).expect("link sidecar");

    assert!(matches!(
        candid_sidecar(&root, root_artifact),
        Err(FleetGenerateError::Candid(reason)) if reason.contains("not a regular no-follow file")
    ));

    fs::remove_dir_all(root).expect("remove fixture root");
}

fn principal_text(byte: u8) -> String {
    Principal::from_slice(&[byte; 29]).to_text()
}

fn divergent_principal_order_pair() -> (Principal, Principal) {
    for left_byte in 0_u8..=u8::MAX {
        let left = Principal::from_slice(&[left_byte]);
        for right_byte in left_byte.saturating_add(1)..=u8::MAX {
            let right = Principal::from_slice(&[right_byte]);
            if left.cmp(&right) != left.to_text().cmp(&right.to_text()) {
                return (left, right);
            }
        }
    }
    panic!("Principal fixture set did not contain divergent text and binary ordering")
}
