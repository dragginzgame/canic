use super::*;
use crate::{
    fleet_ensure::{
        model::{
            CanisterRuntimeStatus, CurrentFleetProtocolAction, EffectRecord, EffectState,
            EnsureAction, FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsureStateRecord,
            FleetObservation, LiveCanister, RootManagementCanisterObservation,
            RootManagementObservation, RootOwnedCanisterLifecycle,
        },
        ops::{
            EffectObservation, EffectOutcome, EffectRetry, EnsurePaths, EnsurePlatform,
            IcpEnsurePlatform, IcpEnsurePlatformError, action_sha256, read_journal,
            read_root_start_authority, read_state, write_journal, write_plan, write_state,
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
    cdk::{
        types::Cycles,
        utils::hash::{hex_bytes, sha256_hex},
    },
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
fn management_runtime_and_module_observations_fail_closed() {
    assert!(matches!(
        parse_observed_runtime_status("root", "running"),
        Ok(CanisterRuntimeStatus::Running)
    ));
    assert!(matches!(
        parse_observed_runtime_status("root", "STOPPED"),
        Ok(CanisterRuntimeStatus::Stopped)
    ));
    assert!(matches!(
        parse_observed_runtime_status("root", "stopping"),
        Ok(CanisterRuntimeStatus::Stopping)
    ));
    assert!(matches!(
        parse_observed_runtime_status("root", "starting"),
        Err(FleetGenerateError::CanisterUnavailable { .. })
    ));

    let uppercase = "AB".repeat(32);
    let normalized = normalize_observed_module_sha256("root", &format!("0x{uppercase}"))
        .expect("valid observed module hash");
    assert_eq!(normalized, uppercase.to_ascii_lowercase());
    let non_hex = "gg".repeat(32);
    for invalid in ["ab", non_hex.as_str()] {
        assert!(matches!(
            normalize_observed_module_sha256("root", invalid),
            Err(FleetGenerateError::CanisterUnavailable { .. })
        ));
    }
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
fn protected_policy_rejects_unsuffixed_cycle_amounts_before_generation() {
    let principal = Principal::from_slice(&[8]).to_text();
    let source = multi_component_source_toml(&principal, &principal, &principal);
    for invalid in [
        source.replacen(
            "canister_cycles = \"5T\"",
            "canister_cycles = \"5000000000000\"",
            1,
        ),
        source.replacen(
            "canister_cycles = \"5T\"",
            "canister_cycles = 5000000000000",
            1,
        ),
    ] {
        assert!(
            toml::from_str::<FleetSource>(&invalid).is_err(),
            "accepted unsuffixed human cycle authority"
        );
    }
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
        "canister_cycles = \"4.8T\"",
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
    let coordinator_module_hash = "83".repeat(32);
    let root_module_hash = "84".repeat(32);
    let store_module_hash = "85".repeat(32);
    let write_icp = |root_runtime_status| {
        write_fake_icp(
            &root,
            FakeIcpFixture {
                authority: &retained_authority,
                coordinator: &coordinator,
                coordinator_module_hash: &coordinator_module_hash,
                fleet_root: &fleet_root,
                operator: &operator,
                pool: &retained_pool,
                public_cycle_balance: None,
                root_module_hash: &root_module_hash,
                root_runtime_status,
                root_status_error: None,
                store: &store,
                store_has_root_controller: false,
                store_module_hash: &store_module_hash,
            },
        )
    };
    let icp = write_icp("stopped");
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
    let preserved_output = root.join("fleets/retained-multi-component.toml");
    fs::create_dir_all(preserved_output.parent().expect("desired output parent"))
        .expect("create desired output parent");
    fs::write(&preserved_output, b"retained desired authority\n")
        .expect("write retained desired authority");
    let Err(stopped) = generate_desired_fleet(&request) else {
        panic!("a stopped retained Root requires a separately reviewed same-ID Start");
    };
    let stopped_message = stopped.to_string();
    assert!(matches!(
        stopped,
        FleetGenerateError::StoppedRootStartRequired(details)
        if matches!(details.as_ref(), StoppedRootStartPrerequisite {
            controller,
            fleet,
            module_sha256,
            root: stopped_root,
            subnet,
            ..
        } if controller == &operator
            && fleet == "retained-multi-component"
            && module_sha256 == &root_module_hash
            && stopped_root == &fleet_root
            && subnet == &placement)
    ));
    let Err(repeated) = generate_desired_fleet(&request) else {
        panic!("the same stopped Root prerequisite must remain deterministic");
    };
    assert_eq!(repeated.to_string(), stopped_message);
    assert!(
        !root.join("root-status-count").exists(),
        "the no-effect generator must not query a stopped Root"
    );
    assert_eq!(
        fs::read(&preserved_output).expect("read preserved desired authority"),
        b"retained desired authority\n"
    );

    write_icp("running");
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
    assert_eq!(desired.ledger_fee_cycles, "0.1B");
    assert_eq!(desired.management_creation_fee_cycles, "0B");
    assert_eq!(desired.material_cycle_threshold, "0.001B");
    assert_eq!(desired.maximum_observation_burn_cycles, "1T");
    for value in [
        &desired.ledger_fee_cycles,
        &desired.management_creation_fee_cycles,
        &desired.material_cycle_threshold,
        &desired.maximum_observation_burn_cycles,
        &desired.maximum_update_burn_cycles,
    ]
    .into_iter()
    .chain(
        desired
            .canisters
            .iter()
            .flat_map(|canister| [&canister.initial_cycles, &canister.minimum_cycles]),
    ) {
        Cycles::from_human_config_str(value).expect("generated cycle value uses compact units");
    }

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
    assert_eq!(fresh.desired.management_creation_fee_cycles, "500B");
    let fresh_seed = fs::read_to_string(&fresh_seed_path).expect("read fresh estate seed");
    assert!(fresh_seed.contains("management_creation_fee_cycles = \"500B\""));
    let invalid_fresh_seed_path = root.join("deployments/fresh-invalid-units.estate.toml");
    fs::write(
        &invalid_fresh_seed_path,
        fresh_seed.replace(
            "management_creation_fee_cycles = \"500B\"",
            "management_creation_fee_cycles = \"500000000000\"",
        ),
    )
    .expect("write invalid fresh estate seed fixture");
    assert!(matches!(
        generate_desired_fleet(&FleetGenerateRequest {
            app_config: &app_config,
            environment: "local",
            fleet: "fresh-invalid-units",
            icp_executable: icp.to_str().expect("fake ICP path"),
            release_build_id,
            root: &root,
            seed: &invalid_fresh_seed_path,
            source: &source_path,
        }),
        Err(FleetGenerateError::FreshSeedConflict(_))
    ));
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
        assert_eq!(pool.initial_cycles, "5T");
        assert_eq!(
            pool.initial_cycles
                .parse::<Cycles>()
                .map(|cycles| cycles.to_u128()),
            Ok(*funded),
        );
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
        RetainedEnsurePlatform::new(&recovery_desired, &observed, &pool_one)
            .with_post_effect_root_owned_protocol();
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
            if matches!(
                configured.kind,
                DesiredCanisterKind::Coordinator | DesiredCanisterKind::Store
            ) {
                live.status = CanisterRuntimeStatus::Stopped;
            }
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
    assert_eq!(ordered.len(), 5);
    assert_eq!(
        ordered
            .iter()
            .filter(|action| matches!(
                action,
                EnsureAction::Install {
                    mode: crate::fleet_ensure::model::InstallMode::Reinstall,
                    ..
                }
            ))
            .count(),
        3
    );
    assert_eq!(
        ordered
            .iter()
            .filter(|action| matches!(action, EnsureAction::Start { .. }))
            .count(),
        2
    );
    assert_eq!(recovery.plan.conservation.maximum_new_funding_cycles, 0);
    assert_eq!(recovery.plan.conservation.maximum_operator_debit_cycles, 0);
    let recovery_error = workflow::apply(
        &root,
        &recovery_desired,
        &source_digest,
        &recovery_desired.fleet,
        &recovery.plan.plan_sha256,
        &mut recovery_platform,
    )
    .expect_err("post-effect protocol work requires a successor review");
    assert!(matches!(
        recovery_error,
        workflow::EnsureWorkflowError::ConvergenceDrift
    ));
    assert_eq!(recovery_platform.mutations, 5);
    let recovery_paths = EnsurePaths::under(
        &root,
        &recovery_desired.environment,
        &recovery_desired.fleet,
    );
    let recovery_journal = crate::fleet_ensure::ops::read_journal(&recovery_paths)
        .expect("read recovery journal")
        .expect("retained recovery journal");
    assert_eq!(
        recovery_journal.completion,
        crate::fleet_ensure::model::FleetEnsureCompletion::ReplanRequired
    );
    let retained_recovery_files = [
        fs::read(&recovery_paths.state).expect("retain post-effect state bytes"),
        fs::read(&recovery_paths.plan).expect("retain reviewed plan bytes"),
        fs::read(&recovery_paths.journal).expect("retain applied journal bytes"),
    ];
    assert!(matches!(
        crate::fleet_ensure::read_current_fleet_inventory(
            &root,
            &recovery_desired.environment,
            &recovery_desired.fleet,
        ),
        Err(crate::fleet_ensure::CurrentFleetInventoryError::NotConverged { .. })
    ));
    let retained_recovery_state = read_state(&recovery_paths, &recovery_desired.fleet)
        .expect("read retained post-effect recovery state");
    for configured in recovery_desired.canisters.iter().filter(|configured| {
        matches!(
            configured.kind,
            DesiredCanisterKind::Root | DesiredCanisterKind::Store | DesiredCanisterKind::Pool
        )
    }) {
        assert_eq!(
            retained_recovery_state
                .principals
                .get(&configured.name)
                .map(String::as_str),
            configured.principal.as_deref()
        );
        let retained = retained_recovery_state
            .topology
            .get(&configured.name)
            .expect("retained Root-owned topology");
        assert_eq!(retained.kind, configured.kind);
        assert_eq!(retained.parent, configured.parent);
    }
    let mut resumed_recovery_platform = recovery_platform.fresh_process();
    let successor_recovery = workflow::plan(
        &root,
        &recovery_desired,
        &source_digest,
        &recovery_desired.fleet,
        1_800_000_000_000_000_075,
        &mut resumed_recovery_platform,
    )
    .expect("fresh host process replans from retained exact topology");
    assert!(
        successor_recovery
            .plan
            .canisters
            .iter()
            .all(|canister| canister.actions.is_empty()),
        "the successor must not repeat reinstall or Start effects"
    );
    assert!(matches!(
        successor_recovery.plan.protocol_actions.as_slice(),
        [EnsureAction::FleetProtocol { .. }]
    ));
    let resumed = workflow::apply(
        &root,
        &recovery_desired,
        &source_digest,
        &recovery_desired.fleet,
        &successor_recovery.plan.plan_sha256,
        &mut resumed_recovery_platform,
    )
    .expect("successor recovery converges through the remaining typed action");
    assert!(resumed.terminal);
    assert_eq!(resumed_recovery_platform.mutations, 1);
    let replay_plan = workflow::plan(
        &root,
        &recovery_desired,
        &source_digest,
        &recovery_desired.fleet,
        1_800_000_000_000_000_076,
        &mut resumed_recovery_platform,
    )
    .expect("plan terminal recovery replay");
    assert!(workflow::ordered_actions(&replay_plan.plan).is_empty());
    let replay = workflow::apply(
        &root,
        &recovery_desired,
        &source_digest,
        &recovery_desired.fleet,
        &replay_plan.plan.plan_sha256,
        &mut resumed_recovery_platform,
    )
    .expect("terminal recovery replay is effect-free");
    assert!(replay.terminal);
    assert_eq!(replay.effects_applied, 0);
    assert_eq!(resumed_recovery_platform.mutations, 1);

    fs::write(&recovery_paths.state, &retained_recovery_files[0])
        .expect("restore untouched post-effect state");
    fs::write(&recovery_paths.plan, &retained_recovery_files[1])
        .expect("restore untouched reviewed plan");
    fs::write(&recovery_paths.journal, &retained_recovery_files[2])
        .expect("restore untouched applied journal");
    let mut overwritten_successor = crate::fleet_ensure::ops::read_plan(&recovery_paths)
        .expect("read retained reviewed plan")
        .expect("retained reviewed plan");
    for canister in &mut overwritten_successor.canisters {
        canister
            .actions
            .retain(|action| matches!(action, EnsureAction::Install { .. }));
    }
    overwritten_successor.protocol_actions = terminal_observation_protocol_actions(
        &recovery_desired,
        &overwritten_successor.operation_id,
    )
    .into_iter()
    .take(1)
    .collect();
    overwritten_successor.planned_at_time += 1;
    overwritten_successor.plan_sha256 =
        crate::fleet_ensure::policy::expected_plan_sha256(&overwritten_successor);
    assert_ne!(
        overwritten_successor.plan_sha256, recovery_journal.plan_sha256,
        "the rejected successor may already have replaced plan.json"
    );
    crate::fleet_ensure::ops::write_plan(&recovery_paths, &overwritten_successor)
        .expect("retain rejected successor plan without changing the applied journal");
    let mut pending_reset_pool = retained_pool.clone();
    for asset in pending_reset_pool
        .entries
        .iter_mut()
        .filter(|asset| asset.origin == CanisterPoolAssetOrigin::Imported)
    {
        asset.cycles = Cycles::new(0);
        asset.status = CanisterPoolAssetStatus::PendingReset;
    }
    pending_reset_pool.workload = 0;
    pending_reset_pool.ready = 0;
    pending_reset_pool.pending_reset = 2;
    let versionless_icp = write_versionless_root_owned_fake_icp(
        &root,
        FakeIcpFixture {
            authority: &retained_authority,
            coordinator: &coordinator,
            coordinator_module_hash: artifacts
                .wasm_sha256_by_canister
                .get("coordinator")
                .expect("Coordinator artifact"),
            fleet_root: &fleet_root,
            operator: &operator,
            pool: &pending_reset_pool,
            public_cycle_balance: None,
            root_module_hash: artifacts
                .wasm_sha256_by_canister
                .get("root-0")
                .expect("Root artifact"),
            root_runtime_status: "running",
            root_status_error: None,
            store: &store,
            store_has_root_controller: true,
            store_module_hash: artifacts
                .wasm_sha256_by_canister
                .get("store-0")
                .expect("Store artifact"),
        },
    );
    fs::write(root.join("root-status-count"), b"1\n")
        .expect("select Root pool status after retained authority");
    let mut versionless_platform = VersionlessPlanningPlatform::new(
        IcpEnsurePlatform::new(
            recovery_desired.clone(),
            versionless_icp
                .to_str()
                .expect("version-less fake ICP path"),
            &root,
        ),
        recovery_desired.clone(),
    );
    let versionless_replan = workflow::plan(
        &root,
        &recovery_desired,
        &source_digest,
        &recovery_desired.fleet,
        1_800_000_000_000_000_077,
        &mut versionless_platform,
    )
    .expect("version-less fresh process retains journal-proved reinstalls");
    let repeated_installs = workflow::ordered_actions(&versionless_replan.plan)
        .into_iter()
        .filter_map(|action| match action {
            EnsureAction::Install { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert!(
        repeated_installs.is_disjoint(&BTreeSet::from(["coordinator", "root-0", "store-0"])),
        "version-less replan repeated proved infrastructure installs: {repeated_installs:?}"
    );
    assert_eq!(
        versionless_replan
            .plan
            .conservation
            .maximum_new_funding_cycles,
        0
    );
    assert_eq!(
        versionless_replan
            .plan
            .conservation
            .maximum_operator_debit_cycles,
        0
    );
    assert_eq!(
        crate::fleet_ensure::ops::read_journal(&recovery_paths)
            .expect("read version-less replan journal")
            .expect("retained version-less replan journal")
            .completion,
        crate::fleet_ensure::model::FleetEnsureCompletion::ReplanRequired
    );
    assert!(matches!(
        versionless_replan.plan.protocol_actions.as_slice(),
        [EnsureAction::FleetProtocol { .. }]
    ));

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
    let predecessor_root_hash = sha256_hex(b"retained predecessor Root protocol");
    let predecessor_store_hash = sha256_hex(b"retained predecessor Store protocol");
    assert_ne!(&predecessor_root_hash, root_hash);
    assert_ne!(&predecessor_store_hash, store_hash);
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
            root_module_hash: &predecessor_root_hash,
            root_runtime_status: "running",
            root_status_error: Some(canic_core::diagnostics::codes::STATE_CONFLICT),
            store: &store,
            store_has_root_controller: true,
            store_module_hash: &predecessor_store_hash,
        },
    );
    let mut state = retained_ensure_state(&production_recovery_desired, &observed, &artifacts);
    state
        .topology
        .get_mut("root-0")
        .expect("retained Root topology")
        .module_hash = Some(predecessor_root_hash.clone());
    state
        .topology
        .get_mut("store-0")
        .expect("retained Store topology")
        .module_hash = Some(predecessor_store_hash.clone());
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
    assert!(production_recovery.plan.protocol_actions.is_empty());
    assert!(matches!(
        production_actions.as_slice(),
        [
            EnsureAction::Install {
                canic_init: Some(DesiredCanisterInit::Coordinator),
                ..
            },
            EnsureAction::Install {
                canic_init: Some(DesiredCanisterInit::Store { .. }),
                ..
            },
            EnsureAction::Install {
                canic_init: Some(DesiredCanisterInit::Root { .. }),
                ..
            }
        ]
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

    let production_paths = EnsurePaths::under(
        &root,
        &production_recovery_desired.environment,
        &production_recovery_desired.fleet,
    );
    let mut retained_rejected_plan = production_recovery.plan.clone();
    let retained_store_control = retained_controller_preparation_action(
        &root,
        &production_recovery_desired,
        &state,
        &retained_rejected_plan.operation_id,
    );
    let EnsureAction::FleetProtocol {
        action: retained_store_control_request,
        ..
    } = &retained_store_control
    else {
        unreachable!("retained Store controller action is typed protocol");
    };
    let CurrentFleetProtocolAction::AdoptStore { request } =
        retained_store_control_request.as_ref()
    else {
        unreachable!("retained Store controller action is adoption");
    };
    assert!(
        crate::fleet_ensure::ops::current_protocol::retained_store_control_request_is_exact(
            &root,
            &production_recovery_desired,
            &retained_rejected_plan.operation_id,
            &state,
            "root-0",
            request,
        )
        .expect("validate retained Store controller request")
    );
    let mut wrong_operation_request = request.clone();
    wrong_operation_request.operation_id[0] ^= 1;
    assert!(
        !crate::fleet_ensure::ops::current_protocol::retained_store_control_request_is_exact(
            &root,
            &production_recovery_desired,
            &retained_rejected_plan.operation_id,
            &state,
            "root-0",
            &wrong_operation_request,
        )
        .expect("reject wrong retained operation")
    );
    let mut wrong_store_request = request.clone();
    wrong_store_request.authority.wasm_store = Principal::anonymous();
    assert!(
        !crate::fleet_ensure::ops::current_protocol::retained_store_control_request_is_exact(
            &root,
            &production_recovery_desired,
            &retained_rejected_plan.operation_id,
            &state,
            "root-0",
            &wrong_store_request,
        )
        .expect("reject wrong retained Store")
    );
    assert!(
        !crate::fleet_ensure::ops::current_protocol::retained_store_control_request_is_exact(
            &root,
            &production_recovery_desired,
            &retained_rejected_plan.operation_id,
            &state,
            "missing-root",
            request,
        )
        .expect("reject wrong retained Root")
    );
    retained_rejected_plan
        .protocol_actions
        .push(retained_store_control.clone());
    retained_rejected_plan.plan_sha256 =
        crate::fleet_ensure::policy::expected_plan_sha256(&retained_rejected_plan);
    let retained_action_hashes = workflow::ordered_actions(&retained_rejected_plan)
        .into_iter()
        .map(action_sha256)
        .collect::<Vec<_>>();
    let coordinator_install = workflow::ordered_actions(&retained_rejected_plan)
        .into_iter()
        .next()
        .expect("retained Coordinator reinstall");
    assert!(matches!(
        coordinator_install,
        EnsureAction::Install {
            canic_init: Some(DesiredCanisterInit::Coordinator),
            ..
        }
    ));
    assert_eq!(
        workflow::ordered_actions(&retained_rejected_plan)
            .get(1)
            .copied(),
        Some(&retained_store_control)
    );
    write_plan(&production_paths, &retained_rejected_plan)
        .expect("write retained rejected plan bytes");
    write_journal(
        &production_paths,
        &FleetEnsureJournalRecord {
            completion: FleetEnsureCompletion::InProgress,
            effects: vec![
                EffectRecord {
                    action_sha256: action_sha256(coordinator_install),
                    created_principal: None,
                    destination_post_cycles: None,
                    destination_pre_cycles: None,
                    post_cycles: Some(270_000_000_000_000),
                    pre_cycles: Some(270_000_000_000_000),
                    pre_canister_version: Some(0),
                    progress_identity: Some("install:coordinator:version:1".to_string()),
                    receipt: None,
                    state: EffectState::Applied,
                },
                EffectRecord {
                    action_sha256: action_sha256(&retained_store_control),
                    created_principal: None,
                    destination_post_cycles: None,
                    destination_pre_cycles: None,
                    post_cycles: None,
                    pre_cycles: Some(30_000_000_000_000),
                    pre_canister_version: Some(1),
                    progress_identity: None,
                    receipt: None,
                    state: EffectState::Intent,
                },
            ],
            fleet: retained_rejected_plan.fleet.clone(),
            initial_controlled_cycles: retained_rejected_plan
                .conservation
                .observed_controlled_cycles,
            initial_operator_cycles: 500_000_000_000_000,
            operation_id: retained_rejected_plan.operation_id.clone(),
            plan_sha256: retained_rejected_plan.plan_sha256.clone(),
            schema_version: crate::fleet_ensure::model::FLEET_ENSURE_SCHEMA_VERSION,
            stalled_observations: 1,
        },
    )
    .expect("write retained rejected journal bytes");
    fs::write(root.join("root-status-count"), b"1\n").expect("select retained typed E132 status");
    let replan_error = workflow::apply(
        &root,
        &production_recovery_desired,
        &source_digest,
        &production_recovery_desired.fleet,
        &retained_rejected_plan.plan_sha256,
        &mut production_platform,
    )
    .expect_err("predecessor Store adoption closes only at replan boundary");
    assert!(
        matches!(
            &replan_error,
            workflow::EnsureWorkflowError::ReplanRequiredAfterRejectedPrerequisite { .. }
        ),
        "unexpected retained rejection result: {replan_error:?}"
    );
    let rejected_journal = read_journal(&production_paths)
        .expect("read rejected journal")
        .expect("retained rejected journal");
    assert_eq!(
        rejected_journal.completion,
        FleetEnsureCompletion::ReplanRequired
    );
    assert_eq!(
        rejected_journal
            .effects
            .iter()
            .map(|effect| effect.action_sha256.clone())
            .collect::<Vec<_>>(),
        retained_action_hashes[..2]
    );
    assert_eq!(rejected_journal.effects[0].state, EffectState::Applied);
    assert_eq!(rejected_journal.effects[1].state, EffectState::Intent);
    assert!(rejected_journal.effects[1].receipt.is_none());
    let rejected_state = read_state(&production_paths, &production_recovery_desired.fleet)
        .expect("read exact rejected state");
    assert_eq!(
        rejected_state.completed_reinstalls,
        BTreeMap::from([("coordinator".to_string(), 0)])
    );
    assert!(!rejected_state.completed_reinstalls.contains_key("root-0"));
    assert!(!rejected_state.completed_reinstalls.contains_key("store-0"));

    fs::write(root.join("root-status-count"), b"1\n")
        .expect("retain typed predecessor Root status for fresh planning");
    let fresh_replan = workflow::plan(
        &root,
        &production_recovery_desired,
        &source_digest,
        &production_recovery_desired.fleet,
        1_800_000_000_000_000_101,
        &mut production_platform,
    )
    .expect("fresh plan preserves only the proved Coordinator reinstall");
    let fresh_actions = workflow::ordered_actions(&fresh_replan.plan);
    assert!(fresh_replan.plan.protocol_actions.is_empty());
    assert!(fresh_actions.iter().all(|action| {
        !matches!(
            action,
            EnsureAction::Install {
                canic_init: Some(DesiredCanisterInit::Coordinator),
                ..
            }
        )
    }));
    assert!(matches!(
        fresh_actions.as_slice(),
        [
            EnsureAction::Install {
                canic_init: Some(DesiredCanisterInit::Store { .. }),
                ..
            },
            EnsureAction::Install {
                canic_init: Some(DesiredCanisterInit::Root { .. }),
                ..
            }
        ]
    ));
    assert_eq!(fresh_replan.plan.conservation.maximum_new_funding_cycles, 0);
    assert_eq!(
        fresh_replan.plan.conservation.maximum_operator_debit_cycles,
        0
    );

    let mut exact_replan_platform =
        RetainedEnsurePlatform::new(&production_recovery_desired, &observed, &pool_one)
            .with_successor_store_adoption(&root);
    for configured in &production_recovery_desired.canisters {
        let principal = configured
            .principal
            .as_deref()
            .expect("retained exact Principal");
        let live = exact_replan_platform
            .live
            .get_mut(principal)
            .expect("retained exact live canister");
        match configured.kind {
            DesiredCanisterKind::Coordinator => {
                live.module_sha256 = Some(coordinator_hash.clone());
            }
            DesiredCanisterKind::Root => {
                live.module_sha256 = Some(predecessor_root_hash.clone());
                live.reinstall_required = true;
            }
            DesiredCanisterKind::Store => {
                live.controllers = vec![fleet_root.clone(), operator.clone()];
                live.controllers.sort();
                live.module_sha256 = Some(predecessor_store_hash.clone());
                live.reinstall_required = true;
            }
            DesiredCanisterKind::Auxiliary
            | DesiredCanisterKind::Pool
            | DesiredCanisterKind::Component => {}
        }
    }
    let initial_exact_cycles = exact_replan_platform.total_cycles();
    let exact_replan_error = workflow::apply(
        &root,
        &production_recovery_desired,
        &source_digest,
        &production_recovery_desired.fleet,
        &fresh_replan.plan.plan_sha256,
        &mut exact_replan_platform,
    )
    .expect_err("successor Store adoption requires one newly reviewed plan");
    assert!(matches!(
        exact_replan_error,
        workflow::EnsureWorkflowError::ConvergenceDrift
    ));
    assert_eq!(exact_replan_platform.mutations, 2);
    assert_eq!(exact_replan_platform.total_cycles(), initial_exact_cycles);
    let exact_reinstall_journal = read_journal(&production_paths)
        .expect("read exact reinstall journal")
        .expect("exact reinstall journal");
    assert_eq!(
        exact_reinstall_journal.completion,
        FleetEnsureCompletion::ReplanRequired
    );
    assert_eq!(exact_reinstall_journal.effects.len(), 2);
    assert!(
        exact_reinstall_journal
            .effects
            .iter()
            .all(|effect| effect.state == EffectState::Applied)
    );

    let mut successor_platform = exact_replan_platform.fresh_process();
    let successor_plan = workflow::plan(
        &root,
        &production_recovery_desired,
        &source_digest,
        &production_recovery_desired.fleet,
        1_800_000_000_000_000_102,
        &mut successor_platform,
    )
    .expect("plan successor-only Store adoption after exact reinstalls");
    assert!(
        successor_plan
            .plan
            .canisters
            .iter()
            .all(|canister| canister.actions.is_empty())
    );
    assert!(matches!(
        successor_plan.plan.protocol_actions.as_slice(),
        [EnsureAction::FleetProtocol { action, name, principal, .. }]
            if matches!(
                action.as_ref(),
                CurrentFleetProtocolAction::AdoptStore { .. }
            ) && name == "root-store-adoption:root-0"
                && principal == &fleet_root
    ));
    assert_eq!(
        successor_plan.plan.conservation.maximum_new_funding_cycles,
        0
    );
    assert_eq!(
        successor_plan
            .plan
            .conservation
            .maximum_operator_debit_cycles,
        0
    );
    let successor_applied = workflow::apply(
        &root,
        &production_recovery_desired,
        &source_digest,
        &production_recovery_desired.fleet,
        &successor_plan.plan.plan_sha256,
        &mut successor_platform,
    )
    .expect("successor Root accepts exact Store authority");
    assert!(successor_applied.terminal);
    assert_eq!(successor_platform.mutations, 1);
    assert_eq!(successor_platform.total_cycles(), initial_exact_cycles);
    let exact_replay_plan = workflow::plan(
        &root,
        &production_recovery_desired,
        &source_digest,
        &production_recovery_desired.fleet,
        1_800_000_000_000_000_103,
        &mut successor_platform,
    )
    .expect("plan exact terminal replay");
    assert!(workflow::ordered_actions(&exact_replay_plan.plan).is_empty());
    let exact_replay = workflow::apply(
        &root,
        &production_recovery_desired,
        &source_digest,
        &production_recovery_desired.fleet,
        &exact_replay_plan.plan.plan_sha256,
        &mut successor_platform,
    )
    .expect("exact terminal replay is effect-free");
    assert!(exact_replay.terminal);
    assert_eq!(exact_replay.effects_applied, 0);
    assert_eq!(successor_platform.mutations, 1);
    assert_eq!(successor_platform.total_cycles(), initial_exact_cycles);

    write_state(&production_paths, &state).expect("restore pre-replan retained state");
    write_plan(&production_paths, &retained_rejected_plan)
        .expect("restore pre-replan retained plan");
    let mut wrong_controller_journal = rejected_journal;
    wrong_controller_journal.completion = FleetEnsureCompletion::InProgress;
    wrong_controller_journal.effects[1].post_cycles = None;
    wrong_controller_journal.effects[1].progress_identity = None;
    wrong_controller_journal.stalled_observations = 1;
    write_journal(&production_paths, &wrong_controller_journal)
        .expect("restore pre-replan retained journal");
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
            root_module_hash: &predecessor_root_hash,
            root_runtime_status: "running",
            root_status_error: Some(canic_core::diagnostics::codes::STATE_CONFLICT),
            store: &store,
            store_has_root_controller: false,
            store_module_hash: &predecessor_store_hash,
        },
    );
    fs::write(root.join("root-status-count"), b"1\n")
        .expect("select wrong-controller typed E132 status");
    let mut wrong_controller_platform = IcpEnsurePlatform::new(
        production_recovery_desired.clone(),
        icp.to_str().expect("fake ICP path"),
        &root,
    );
    assert!(matches!(
        workflow::apply(
            &root,
            &production_recovery_desired,
            &source_digest,
            &production_recovery_desired.fleet,
            &retained_rejected_plan.plan_sha256,
            &mut wrong_controller_platform,
        ),
        Err(workflow::EnsureWorkflowError::Platform(
            IcpEnsurePlatformError::CurrentProtocol(_)
        ))
    ));
    let wrong_controller_retained = read_journal(&production_paths)
        .expect("read wrong-controller journal")
        .expect("wrong-controller journal retained");
    assert_eq!(
        wrong_controller_retained.completion,
        FleetEnsureCompletion::InProgress
    );
    assert_eq!(
        wrong_controller_retained.effects[0].state,
        EffectState::Applied
    );
    assert_eq!(
        wrong_controller_retained.effects[1].state,
        EffectState::Intent
    );
    assert!(wrong_controller_retained.effects[1].receipt.is_none());

    let successor_release =
        plan_release_build_for_profile(&root, crate::build_profile::CanisterBuildProfile::Fast)
            .expect("plan distinct requested successor release");
    let successor_release_build_id = successor_release.record.release_build_id;
    assert_ne!(successor_release_build_id, release_build_id);
    persist_test_release_authority(&root, &config, successor_release_build_id);
    let successor_manifest =
        crate::release_set::load_persisted_canic_infrastructure_artifact_manifest(
            &root,
            successor_release_build_id,
        )
        .expect("load requested successor infrastructure manifest");
    let requested_successor_root_hash = successor_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .expect("requested successor Root artifact")
        .wasm_sha256_hex
        .clone();

    let mut stopped_desired = recovery_desired.clone();
    stopped_desired.fleet = "retained-stopped-root".to_string();
    let stopped_artifacts =
        crate::fleet_ensure::ops::resolve_desired_artifacts(&root, &stopped_desired)
            .expect("resolve retained desired artifacts");
    let retained_desired_root_hash = stopped_artifacts
        .wasm_sha256_by_canister
        .get("root-0")
        .expect("retained desired Root artifact hash");
    let stopped_root_hash = sha256_hex(b"retained predecessor Root");
    assert_ne!(&stopped_root_hash, retained_desired_root_hash);
    assert_ne!(&stopped_root_hash, &requested_successor_root_hash);
    assert_ne!(retained_desired_root_hash, &requested_successor_root_hash);
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
            root_module_hash: &stopped_root_hash,
            root_runtime_status: "stopped",
            root_status_error: None,
            store: &store,
            store_has_root_controller: true,
            store_module_hash: store_hash,
        },
    );
    let root_status_counter = root.join("root-status-count");
    if root_status_counter.exists() {
        fs::remove_file(&root_status_counter).expect("reset Root query counter");
    }
    let configured_root = stopped_desired
        .canisters
        .iter()
        .find(|canister| canister.kind == DesiredCanisterKind::Root)
        .expect("configured retained Root");
    let stopped_request = FleetGenerateRequest {
        app_config: &app_config,
        environment: &stopped_desired.environment,
        fleet: &stopped_desired.fleet,
        icp_executable: icp.to_str().expect("fake ICP path"),
        release_build_id: successor_release_build_id,
        root: &root,
        seed: &seed_path,
        source: &source_path,
    };
    let Err(stopped_error) = generate_desired_fleet(&stopped_request) else {
        panic!("stopped predecessor Root requires one retained Start prerequisite");
    };
    assert!(matches!(
        stopped_error,
        FleetGenerateError::StoppedRootStartRequired(details)
            if details.module_sha256 == stopped_root_hash
                && details.successor_module_sha256 == requested_successor_root_hash
    ));
    let stopped_paths =
        EnsurePaths::under(&root, &stopped_desired.environment, &stopped_desired.fleet);
    let stopped_authority = read_root_start_authority(&stopped_paths)
        .expect("read retained Root-start authority")
        .expect("generator retained Root-start authority");
    assert_eq!(
        stopped_authority.release_build_id,
        successor_release_build_id
    );
    assert_eq!(
        stopped_authority.successor_module_sha256,
        requested_successor_root_hash
    );
    crate::fleet_ensure::ops::verify_root_start_release_authority(&root, &stopped_authority)
        .expect("sealed authority resolves its exact finalized successor");
    let mut wrong_release_authority = stopped_authority.clone();
    wrong_release_authority.release_build_id = release_build_id;
    wrong_release_authority.seal();
    assert!(matches!(
        crate::fleet_ensure::ops::verify_root_start_release_authority(
            &root,
            &wrong_release_authority,
        ),
        Err(crate::fleet_ensure::ops::EnsureStateError::InvalidRootStartReleaseAuthority { .. })
    ));
    let mut wrong_successor_authority = stopped_authority.clone();
    wrong_successor_authority.successor_module_sha256 = "00".repeat(32);
    wrong_successor_authority.seal();
    assert!(matches!(
        crate::fleet_ensure::ops::verify_root_start_release_authority(
            &root,
            &wrong_successor_authority,
        ),
        Err(crate::fleet_ensure::ops::EnsureStateError::InvalidRootStartReleaseAuthority { .. })
    ));
    let successor_root_entry = successor_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .expect("requested successor Root entry");
    let successor_root_path = root.join(&successor_root_entry.wasm_relative_path);
    let successor_root_bytes = fs::read(&successor_root_path).expect("read successor Root bytes");
    fs::write(&successor_root_path, b"changed successor Root").expect("tamper successor Root");
    assert!(matches!(
        crate::fleet_ensure::ops::verify_root_start_release_authority(&root, &stopped_authority,),
        Err(crate::fleet_ensure::ops::EnsureStateError::InvalidRootStartReleaseAuthority { .. })
    ));
    fs::write(&successor_root_path, successor_root_bytes).expect("restore successor Root bytes");
    let retained_authority_bytes = fs::read(&stopped_paths.root_start_authority)
        .expect("read exact retained Root-start authority bytes");
    let mut tampered_authority = stopped_authority.clone();
    tampered_authority.successor_module_sha256 = "00".repeat(32);
    fs::write(
        &stopped_paths.root_start_authority,
        serde_json::to_vec_pretty(&tampered_authority).expect("encode tampered authority"),
    )
    .expect("write tampered authority fixture");
    assert!(matches!(
        read_root_start_authority(&stopped_paths),
        Err(crate::fleet_ensure::ops::EnsureStateError::InvalidRootStartAuthority { .. })
    ));
    fs::write(
        &stopped_paths.root_start_authority,
        &retained_authority_bytes,
    )
    .expect("restore retained Root-start authority bytes");
    let later_release =
        plan_release_build_for_profile(&root, crate::build_profile::CanisterBuildProfile::Debug)
            .expect("plan later requested release");
    let later_release_build_id = later_release.record.release_build_id;
    assert_ne!(later_release_build_id, successor_release_build_id);
    persist_test_release_authority(&root, &config, later_release_build_id);
    let later_manifest = crate::release_set::load_persisted_canic_infrastructure_artifact_manifest(
        &root,
        later_release_build_id,
    )
    .expect("load later requested infrastructure manifest");
    let later_root_hash = later_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .expect("later requested Root artifact")
        .wasm_sha256_hex
        .clone();
    assert_ne!(later_root_hash, requested_successor_root_hash);
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
            root_module_hash: &stopped_root_hash,
            root_runtime_status: "running",
            root_status_error: None,
            store: &store,
            store_has_root_controller: true,
            store_module_hash: store_hash,
        },
    );
    let later_request = FleetGenerateRequest {
        app_config: &app_config,
        environment: &stopped_desired.environment,
        fleet: &stopped_desired.fleet,
        icp_executable: icp.to_str().expect("fake ICP path"),
        release_build_id: later_release_build_id,
        root: &root,
        seed: &seed_path,
        source: &source_path,
    };
    let Err(later_error) = generate_desired_fleet(&later_request) else {
        panic!("a later successor cannot retarget the sealed predecessor authority");
    };
    let later_message = later_error.to_string();
    assert!(
        matches!(
        &later_error,
        FleetGenerateError::SealedSuccessorConvergenceRequired(details)
            if details.fleet == stopped_desired.fleet
                && details.sealed_release_build_id == successor_release_build_id.to_string()
                && details.sealed_successor_module_sha256 == requested_successor_root_hash
                && details.requested_release_build_id == later_release_build_id.to_string()
                && details.requested_successor_module_sha256 == later_root_hash
        ),
        "unexpected later-successor result: {later_error:?}"
    );
    assert!(later_message.contains("canic fleet ensure retained-stopped-root"));
    assert!(later_message.contains("terminally converge the retained desired successor"));
    assert_eq!(
        fs::read(&stopped_paths.root_start_authority)
            .expect("read unchanged sealed Root-start authority"),
        retained_authority_bytes
    );
    assert!(
        !root_status_counter.exists(),
        "later generation must reject before a protected predecessor query"
    );
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
            root_module_hash: &stopped_root_hash,
            root_runtime_status: "stopped",
            root_status_error: None,
            store: &store,
            store_has_root_controller: true,
            store_module_hash: store_hash,
        },
    );
    let root_management = RootManagementObservation {
        operator_cycles: 500_000_000_000_000,
        roots: BTreeMap::from([(
            configured_root.name.clone(),
            RootManagementCanisterObservation {
                live: LiveCanister {
                    canister_version: Some(7),
                    controllers: configured_root.controllers.clone(),
                    cycles: 30_000_000_000_000,
                    module_sha256: Some(stopped_root_hash.clone()),
                    principal: fleet_root.clone(),
                    reinstall_required: false,
                    root_owned_lifecycle: None,
                    status: CanisterRuntimeStatus::Stopped,
                },
                name: configured_root.name.clone(),
                subnet: configured_root.subnet.clone(),
            },
        )]),
    };
    assert!(matches!(
        crate::fleet_ensure::policy::compile_root_start_prerequisite_plan(
            crate::fleet_ensure::policy::RootStartPlanInput {
                authority: None,
                created_at_time: 1_800_000_000_000_000_150,
                desired: &stopped_desired,
                desired_sha256: &source_digest,
                observation: &root_management,
                requested_fleet: &stopped_desired.fleet,
            },
        ),
        Err(
            crate::fleet_ensure::policy::EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "retained module authority",
                ..
            }
        )
    ));
    let mut wrong_authority = stopped_authority.clone();
    wrong_authority.fleet = "other-fleet".to_string();
    wrong_authority.seal();
    assert!(matches!(
        crate::fleet_ensure::policy::compile_root_start_prerequisite_plan(
            crate::fleet_ensure::policy::RootStartPlanInput {
                authority: Some(&wrong_authority),
                created_at_time: 1_800_000_000_000_000_150,
                desired: &stopped_desired,
                desired_sha256: &source_digest,
                observation: &root_management,
                requested_fleet: &stopped_desired.fleet,
            },
        ),
        Err(
            crate::fleet_ensure::policy::EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "retained module authority",
                ..
            }
        )
    ));
    let mut wrong_fleet_identity = stopped_authority.clone();
    wrong_fleet_identity.fleet_id = "ff".repeat(32).parse().expect("wrong Fleet ID");
    wrong_fleet_identity.seal();
    assert!(matches!(
        crate::fleet_ensure::policy::compile_root_start_prerequisite_plan(
            crate::fleet_ensure::policy::RootStartPlanInput {
                authority: Some(&wrong_fleet_identity),
                created_at_time: 1_800_000_000_000_000_150,
                desired: &stopped_desired,
                desired_sha256: &source_digest,
                observation: &root_management,
                requested_fleet: &stopped_desired.fleet,
            },
        ),
        Err(
            crate::fleet_ensure::policy::EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "retained module authority",
                ..
            }
        )
    ));
    crate::fleet_ensure::policy::compile_root_start_prerequisite_plan(
        crate::fleet_ensure::policy::RootStartPlanInput {
            authority: Some(&stopped_authority),
            created_at_time: 1_800_000_000_000_000_150,
            desired: &stopped_desired,
            desired_sha256: &source_digest,
            observation: &root_management,
            requested_fleet: &stopped_desired.fleet,
        },
    )
    .expect("exact Root management authority")
    .expect("stopped Root prerequisite");
    for (field, drift) in [
        ("Principal", "principal"),
        ("Subnet", "subnet"),
        ("controllers", "controller"),
        ("module SHA-256", "module"),
        ("runtime", "runtime"),
    ] {
        let mut drifted = root_management.clone();
        let observed = drifted
            .roots
            .get_mut(&configured_root.name)
            .expect("drifted Root observation");
        match drift {
            "principal" => observed.live.principal = principal_text(98),
            "subnet" => observed.subnet = principal_text(97),
            "controller" => observed.live.controllers = vec![principal_text(96)],
            "module" => observed.live.module_sha256 = Some("00".repeat(32)),
            "runtime" => observed.live.status = CanisterRuntimeStatus::Stopping,
            _ => unreachable!(),
        }
        let error = crate::fleet_ensure::policy::compile_root_start_prerequisite_plan(
            crate::fleet_ensure::policy::RootStartPlanInput {
                authority: Some(&stopped_authority),
                created_at_time: 1_800_000_000_000_000_150,
                desired: &stopped_desired,
                desired_sha256: &source_digest,
                observation: &drifted,
                requested_fleet: &stopped_desired.fleet,
            },
        )
        .expect_err("drifted Root authority must fail before planning");
        assert!(
            matches!(
                (&error, drift),
                (
                    crate::fleet_ensure::policy::EnsurePolicyError::RootManagementAuthorityMismatch { .. },
                    "principal" | "subnet" | "controller" | "module"
                ) | (
                    crate::fleet_ensure::policy::EnsurePolicyError::RootStopping { .. },
                    "runtime"
                )
            ),
            "unexpected {field} drift error: {error:?}"
        );
    }
    let retained_release_manifest_path = root
        .join(".canic/release-builds")
        .join(release_build_id.to_string())
        .join("current-release-set-manifest.json");
    let retained_release_manifest_bytes = fs::read(&retained_release_manifest_path)
        .expect("read retained desired release manifest bytes");
    fs::write(
        &retained_release_manifest_path,
        b"{\"schema_version\":1,\"historical\":true}",
    )
    .expect("replace retained desired release manifest with unsupported historical bytes");
    let retained_root_artifact_path = root.join(
        configured_root
            .wasm
            .as_deref()
            .expect("retained desired Root Wasm path"),
    );
    let retained_root_artifact_bytes =
        fs::read(&retained_root_artifact_path).expect("read retained desired Root artifact");
    fs::remove_file(&retained_root_artifact_path)
        .expect("remove disposable retained desired Root artifact");
    write_state(
        &stopped_paths,
        &FleetEnsureStateRecord {
            active_registry: None,
            completed_reinstall_action_sha256: BTreeMap::new(),
            completed_reinstall_operation_id: None,
            completed_reinstalls: BTreeMap::new(),
            fleet: stopped_desired.fleet.clone(),
            pending_principals: BTreeMap::new(),
            principals: BTreeMap::new(),
            retained_cycles_by_principal: observed
                .iter()
                .map(|(principal, canister)| (principal.clone(), canister.cycles))
                .collect(),
            schema_version: crate::fleet_ensure::model::FLEET_ENSURE_SCHEMA_VERSION,
            topology: BTreeMap::new(),
        },
    )
    .expect("retain real schema-1 state without synthesized topology");
    let mut stopped_platform = IcpEnsurePlatform::new(
        stopped_desired.clone(),
        icp.to_str().expect("fake ICP path"),
        &root,
    );
    let stopped_plan = workflow::plan(
        &root,
        &stopped_desired,
        &source_digest,
        &stopped_desired.fleet,
        1_800_000_000_000_000_150,
        &mut stopped_platform,
    )
    .expect("plan exact stopped Root before protected role observation");
    let stopped_actions = workflow::ordered_actions(&stopped_plan.plan);
    assert!(matches!(
        stopped_actions.as_slice(),
        [EnsureAction::Start { name, principal }]
            if name == "root-0" && principal == &fleet_root
    ));
    assert!(
        !root_status_counter.exists(),
        "planning must not query a stopped Root role endpoint"
    );
    assert_eq!(stopped_plan.plan.conservation.maximum_new_funding_cycles, 0);
    assert_eq!(
        stopped_plan.plan.conservation.maximum_operator_debit_cycles,
        0
    );
    assert_eq!(
        stopped_plan.plan.scope,
        crate::fleet_ensure::model::FleetEnsurePlanScope::RootStartPrerequisite
    );
    assert_eq!(
        stopped_plan.plan.root_start_authority.as_deref(),
        Some(&stopped_authority)
    );
    let reviewed_plan_sha256 = stopped_plan.plan.plan_sha256.clone();
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
            root_module_hash: &"00".repeat(32),
            root_runtime_status: "stopped",
            root_status_error: None,
            store: &store,
            store_has_root_controller: true,
            store_module_hash: store_hash,
        },
    );
    assert!(
        workflow::apply(
            &root,
            &stopped_desired,
            &source_digest,
            &stopped_desired.fleet,
            &reviewed_plan_sha256,
            &mut stopped_platform,
        )
        .is_err()
    );
    assert!(
        !root.join("root-start-count").exists(),
        "pre-effect predecessor drift must reject before Start"
    );
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
            root_module_hash: &stopped_root_hash,
            root_runtime_status: "stopped",
            root_status_error: None,
            store: &store,
            store_has_root_controller: true,
            store_module_hash: store_hash,
        },
    );
    let applied = workflow::apply(
        &root,
        &stopped_desired,
        &source_digest,
        &stopped_desired.fleet,
        &reviewed_plan_sha256,
        &mut stopped_platform,
    )
    .expect("apply only the reviewed same-ID Root Start");
    assert!(applied.terminal);
    assert_eq!(applied.effects_applied, 1);
    assert_eq!(
        applied
            .actual_conservation
            .expect("Root-start conservation")
            .operator_debit_cycles,
        0
    );
    assert_eq!(
        fs::read_to_string(root.join("root-start-count")).expect("Root start count"),
        "1\n"
    );
    assert!(
        !root_status_counter.exists(),
        "applying the prerequisite must not query a protected Root endpoint"
    );
    let replay = workflow::apply(
        &root,
        &stopped_desired,
        &source_digest,
        &stopped_desired.fleet,
        &reviewed_plan_sha256,
        &mut stopped_platform,
    )
    .expect("terminal Root-start replay is effect-free");
    assert!(replay.terminal);
    assert_eq!(replay.effects_applied, 1);
    assert_eq!(
        fs::read_to_string(root.join("root-start-count")).expect("replayed Root start count"),
        "1\n"
    );
    let retained_after_start = read_state(&stopped_paths, &stopped_desired.fleet)
        .expect("read state after Root-start prerequisite");
    assert!(retained_after_start.principals.is_empty());
    assert!(retained_after_start.topology.is_empty());

    fs::remove_file(&stopped_paths.root_start_authority)
        .expect("remove disposable bridge authority");
    fs::write(
        root.join("predecessor-pool-status"),
        b"exact predecessor shape\n",
    )
    .expect("select predecessor response without authority");
    assert!(matches!(
        generate_desired_fleet(&stopped_request),
        Err(FleetGenerateError::CanisterUnavailable { .. })
    ));
    fs::write(
        &stopped_paths.root_start_authority,
        &retained_authority_bytes,
    )
    .expect("restore exact bridge authority");
    fs::remove_file(&root_status_counter).expect("reset Root status fixture");

    let mut wrong_bridge_successor = stopped_authority;
    wrong_bridge_successor.successor_module_sha256 = "00".repeat(32);
    wrong_bridge_successor.seal();
    fs::write(
        &stopped_paths.root_start_authority,
        serde_json::to_vec_pretty(&wrong_bridge_successor).expect("encode wrong bridge authority"),
    )
    .expect("write wrong bridge authority");
    assert!(matches!(
        generate_desired_fleet(&stopped_request),
        Err(FleetGenerateError::Authority(_))
    ));
    fs::write(
        &stopped_paths.root_start_authority,
        &retained_authority_bytes,
    )
    .expect("restore exact bridge authority");

    let unrelated_running_module = "11".repeat(32);
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
            root_module_hash: &unrelated_running_module,
            root_runtime_status: "running",
            root_status_error: None,
            store: &store,
            store_has_root_controller: true,
            store_module_hash: store_hash,
        },
    );
    if root_status_counter.exists() {
        fs::remove_file(&root_status_counter).expect("reset exact bridge status fixture");
    }
    fs::write(
        root.join("predecessor-pool-status"),
        b"exact predecessor shape\n",
    )
    .expect("select predecessor response for unrelated module");
    assert!(matches!(
        generate_desired_fleet(&stopped_request),
        Err(FleetGenerateError::CanisterUnavailable { .. })
    ));
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
            root_module_hash: &stopped_root_hash,
            root_runtime_status: "running",
            root_status_error: None,
            store: &store,
            store_has_root_controller: true,
            store_module_hash: store_hash,
        },
    );
    if root_status_counter.exists() {
        fs::remove_file(&root_status_counter).expect("reset successful bridge status fixture");
    }
    fs::write(root.join("root-started"), b"running\n")
        .expect("restore terminal Root-start observation");
    fs::write(
        root.join("predecessor-pool-status"),
        b"exact predecessor shape\n",
    )
    .expect("select exact predecessor Root pool response");
    let regenerated = generate_desired_fleet(&stopped_request)
        .expect("bridge exact predecessor status after reviewed Root Start");
    assert_eq!(
        regenerated
            .desired
            .bootstrap
            .as_ref()
            .expect("regenerated bootstrap")
            .release_build_id,
        successor_release_build_id
    );
    assert_eq!(
        fs::read(&preserved_output).expect("read unchanged retained desired authority"),
        b"retained desired authority\n"
    );
    fs::write(
        &retained_release_manifest_path,
        retained_release_manifest_bytes,
    )
    .expect("restore retained desired release manifest for later independent cases");
    fs::write(&retained_root_artifact_path, retained_root_artifact_bytes)
        .expect("restore retained desired Root artifact for later independent cases");
    assert_eq!(regenerated.observed_canisters, 5);
    assert_eq!(regenerated.observed_controlled_cycles, 319_899_950_000_000);
    assert!(root_status_counter.exists());

    let successor_desired_sha256 = sha256_hex(
        &serde_json::to_vec(&regenerated.desired).expect("encode regenerated desired Fleet"),
    );
    let mut successor_platform = IcpEnsurePlatform::new(
        regenerated.desired.clone(),
        icp.to_str().expect("fake ICP path"),
        &root,
    );
    let successor_plan = workflow::plan(
        &root,
        &regenerated.desired,
        &successor_desired_sha256,
        &regenerated.desired.fleet,
        1_800_000_000_000_000_175,
        &mut successor_platform,
    )
    .expect("review ordinary successor plan through predecessor status bridge");
    let successor_actions = workflow::ordered_actions(&successor_plan.plan);
    assert!(successor_actions.iter().any(|action| {
        matches!(
            action,
            EnsureAction::Install {
                mode: crate::fleet_ensure::model::InstallMode::Reinstall,
                name,
                wasm_sha256,
                ..
            } if name == "root-0" && wasm_sha256 == &requested_successor_root_hash
        )
    }));
    assert!(successor_actions.iter().all(|action| {
        !matches!(
            action,
            EnsureAction::Create { .. }
                | EnsureAction::Delete { .. }
                | EnsureAction::Fund { .. }
                | EnsureAction::Transfer { .. }
        )
    }));
    assert_eq!(
        successor_plan
            .plan
            .conservation
            .maximum_operator_debit_cycles,
        0
    );
    assert_eq!(
        successor_plan.plan.conservation.maximum_new_funding_cycles,
        0
    );
    assert_eq!(
        successor_plan
            .plan
            .conservation
            .maximum_unavoidable_fee_cycles,
        0
    );
    assert_eq!(
        successor_plan.plan.conservation.scheduled_transfer_cycles,
        0
    );

    fs::write(&root_status_counter, b"1\n").expect("prime later pool-only fake query");

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
            root_runtime_status: "running",
            root_status_error: None,
            store: &store,
            store_has_root_controller: false,
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
    post_effect_protocol: bool,
    post_effect_protocol_applied: bool,
    successor_store_adoption_root: Option<PathBuf>,
    terminal_observation_protocol: bool,
}

struct VersionlessPlanningPlatform {
    desired: DesiredFleet,
    inner: IcpEnsurePlatform,
}

impl VersionlessPlanningPlatform {
    fn new(inner: IcpEnsurePlatform, desired: DesiredFleet) -> Self {
        Self { desired, inner }
    }
}

impl EnsurePlatform for VersionlessPlanningPlatform {
    type Error = io::Error;

    fn bind_reviewed_desired(&mut self, desired: &DesiredFleet) -> Result<(), Self::Error> {
        self.desired = desired.clone();
        self.inner
            .bind_reviewed_desired(desired)
            .map_err(io::Error::other)
    }

    fn observe_root_management(
        &mut self,
        state: &FleetEnsureStateRecord,
        reviewed_targets: &BTreeSet<String>,
    ) -> Result<Option<RootManagementObservation>, Self::Error> {
        self.inner
            .observe_root_management(state, reviewed_targets)
            .map_err(io::Error::other)
    }

    fn observe(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        self.inner
            .observe(operation_id, state)
            .map_err(io::Error::other)
    }

    fn protocol_actions(
        &mut self,
        operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Vec<EnsureAction>, Self::Error> {
        Ok(
            terminal_observation_protocol_actions(&self.desired, operation_id)
                .into_iter()
                .take(1)
                .collect(),
        )
    }

    fn observe_effect(
        &mut self,
        _operation_id: &str,
        _action: &EnsureAction,
        _record: &EffectRecord,
        _state: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error> {
        Err(io::Error::other("planning adapter cannot observe effects"))
    }

    fn action_cycles(
        &mut self,
        _action: &EnsureAction,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        Err(io::Error::other("planning adapter cannot observe effects"))
    }

    fn action_destination_cycles(
        &mut self,
        _action: &EnsureAction,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error> {
        Err(io::Error::other("planning adapter cannot observe effects"))
    }

    fn apply(
        &mut self,
        _operation_id: &str,
        _action: &EnsureAction,
        _record: &EffectRecord,
        _state: &FleetEnsureStateRecord,
    ) -> Result<EffectOutcome, Self::Error> {
        Err(io::Error::other("planning adapter cannot apply effects"))
    }
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
            ledger_fee_cycles: desired
                .ledger_fee_cycles
                .parse::<Cycles>()
                .map(|cycles| cycles.to_u128())
                .expect("ledger fee"),
            live,
            mutations: 0,
            post_effect_protocol: false,
            post_effect_protocol_applied: false,
            successor_store_adoption_root: None,
            terminal_observation_protocol: false,
        }
    }

    fn fresh_process(&self) -> Self {
        Self {
            desired: self.desired.clone(),
            ledger_fee_cycles: self.ledger_fee_cycles,
            live: self.live.clone(),
            mutations: 0,
            post_effect_protocol: self.post_effect_protocol,
            post_effect_protocol_applied: self.post_effect_protocol_applied,
            successor_store_adoption_root: self.successor_store_adoption_root.clone(),
            terminal_observation_protocol: self.terminal_observation_protocol,
        }
    }

    fn with_post_effect_root_owned_protocol(mut self) -> Self {
        self.post_effect_protocol = true;
        self
    }

    fn with_successor_store_adoption(mut self, root: &Path) -> Self {
        self.post_effect_protocol = true;
        self.successor_store_adoption_root = Some(root.to_path_buf());
        self
    }

    fn with_terminal_observation_protocol(mut self) -> Self {
        self.terminal_observation_protocol = true;
        self
    }

    fn total_cycles(&self) -> u128 {
        self.live.values().map(|canister| canister.cycles).sum()
    }

    fn infrastructure_ready(&self) -> bool {
        self.desired
            .canisters
            .iter()
            .filter(|configured| {
                matches!(
                    configured.kind,
                    DesiredCanisterKind::Coordinator
                        | DesiredCanisterKind::Root
                        | DesiredCanisterKind::Store
                )
            })
            .all(|configured| {
                configured
                    .principal
                    .as_ref()
                    .and_then(|principal| self.live.get(principal))
                    .is_some_and(|live| {
                        !live.reinstall_required && live.status == CanisterRuntimeStatus::Running
                    })
            })
    }

    fn require_exact_root_owned_topology(
        &self,
        state: &FleetEnsureStateRecord,
    ) -> Result<(), io::Error> {
        for configured in self.desired.canisters.iter().filter(|configured| {
            matches!(
                configured.kind,
                DesiredCanisterKind::Store | DesiredCanisterKind::Pool
            )
        }) {
            let principal = configured
                .principal
                .as_ref()
                .expect("retained Root-owned Principal");
            let parent = configured
                .parent
                .as_ref()
                .expect("retained Root-owned parent");
            let parent_principal = self
                .desired
                .canisters
                .iter()
                .find(|candidate| candidate.name == *parent)
                .and_then(|candidate| candidate.principal.as_ref())
                .expect("retained Root Principal");
            let exact = state.principals.get(&configured.name) == Some(principal)
                && state.principals.get(parent) == Some(parent_principal)
                && state
                    .topology
                    .get(&configured.name)
                    .is_some_and(|topology| {
                        topology.kind == configured.kind && topology.parent.as_ref() == Some(parent)
                    });
            if !exact {
                return Err(io::Error::other(format!(
                    "{} has no exact retained topology authority",
                    configured.name
                )));
            }
        }
        Ok(())
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
        state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error> {
        if self.post_effect_protocol
            && !self.post_effect_protocol_applied
            && self.infrastructure_ready()
        {
            self.require_exact_root_owned_topology(state)?;
        }
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
        state: &FleetEnsureStateRecord,
    ) -> Result<Vec<EnsureAction>, Self::Error> {
        if self.terminal_observation_protocol {
            return Ok(terminal_observation_protocol_actions(
                &self.desired,
                operation_id,
            ));
        }
        if self.post_effect_protocol
            && self.infrastructure_ready()
            && !self.post_effect_protocol_applied
        {
            if let Some(root) = &self.successor_store_adoption_root {
                return Ok(vec![successor_store_adoption_action(
                    root,
                    &self.desired,
                    state,
                    operation_id,
                )]);
            }
            return Ok(
                terminal_observation_protocol_actions(&self.desired, operation_id)
                    .into_iter()
                    .take(1)
                    .collect(),
            );
        }
        Ok(Vec::new())
    }

    fn observe_effect(
        &mut self,
        _operation_id: &str,
        action: &EnsureAction,
        _record: &EffectRecord,
        _state: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error> {
        let applied = match action {
            EnsureAction::Install {
                principal,
                wasm_sha256,
                ..
            } => self.live.get(principal).is_some_and(|canister| {
                !canister.reinstall_required
                    && canister.module_sha256.as_deref() == Some(wasm_sha256)
            }),
            EnsureAction::Start { principal, .. } => self
                .live
                .get(principal)
                .is_some_and(|canister| canister.status == CanisterRuntimeStatus::Running),
            EnsureAction::FleetProtocol { .. } => self.post_effect_protocol_applied,
            _ => return Err(io::Error::other("unexpected retained journey effect")),
        };
        Ok(EffectObservation {
            applied,
            progress_identity: format!("retained:{}:{applied}", action.name()),
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
        let (EnsureAction::Install { principal, .. } | EnsureAction::Start { principal, .. }) =
            action
        else {
            return Ok(None);
        };
        Ok(self.live.get(principal).map(|canister| canister.cycles))
    }

    fn action_canister_version(
        &mut self,
        action: &EnsureAction,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<u64>, Self::Error> {
        let EnsureAction::Install { principal, .. } = action else {
            return Ok(None);
        };
        Ok(self
            .live
            .get(principal)
            .and_then(|canister| canister.canister_version))
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
        let (post_cycles, receipt) = match action {
            EnsureAction::Install {
                principal,
                wasm_sha256,
                ..
            } => {
                let canister = self
                    .live
                    .get_mut(principal)
                    .ok_or_else(|| io::Error::other("missing retained canister"))?;
                canister.module_sha256 = Some(wasm_sha256.clone());
                canister.reinstall_required = false;
                canister.canister_version = canister.canister_version.map(|version| version + 1);
                (Some(canister.cycles), format!("installed:{principal}"))
            }
            EnsureAction::Start { principal, .. } => {
                let canister = self
                    .live
                    .get_mut(principal)
                    .ok_or_else(|| io::Error::other("missing retained canister"))?;
                canister.status = CanisterRuntimeStatus::Running;
                (Some(canister.cycles), format!("started:{principal}"))
            }
            EnsureAction::FleetProtocol { name, .. } => {
                self.post_effect_protocol_applied = true;
                (None, format!("protocol:{name}"))
            }
            _ => return Err(io::Error::other("unexpected retained journey effect")),
        };
        self.mutations += 1;
        Ok(EffectOutcome {
            created_principal: None,
            post_cycles,
            receipt: Some(receipt),
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

fn retained_controller_preparation_action(
    root: &Path,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    operation_id: &str,
) -> EnsureAction {
    let protocol = desired.protocol.as_ref().expect("generated protocol");
    let configured_root = desired
        .canisters
        .iter()
        .find(|canister| canister.kind == DesiredCanisterKind::Root)
        .expect("retained Root");
    let request = crate::fleet_ensure::ops::current_protocol::expected_retained_store_control_request_for_test(
        root,
        desired,
        operation_id,
        state,
        &configured_root.name,
    )
    .expect("compile retained Store controller request")
    .expect("retained Store controller request");
    let subject_text = configured_root
        .principal
        .clone()
        .expect("retained Root Principal");
    let candid_bytes = fs::read(root.join(&protocol.root_candid)).expect("read Root Candid");
    EnsureAction::FleetProtocol {
        action: Box::new(CurrentFleetProtocolAction::AdoptStore { request }),
        candid: protocol.root_candid.clone(),
        candid_sha256: sha256_hex(&candid_bytes),
        maximum_execution_burn_cycles: desired
            .maximum_update_burn_cycles
            .parse::<Cycles>()
            .expect("maximum update burn")
            .to_u128(),
        name: format!("root-store-control:{}", configured_root.name),
        principal: subject_text,
    }
}

fn successor_store_adoption_action(
    root: &Path,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    operation_id: &str,
) -> EnsureAction {
    let mut action = retained_controller_preparation_action(root, desired, state, operation_id);
    let EnsureAction::FleetProtocol { name, .. } = &mut action else {
        unreachable!("Store adoption helper always returns one typed protocol action");
    };
    *name = "root-store-adoption:root-0".to_string();
    action
}

fn retained_ensure_state(
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
    artifacts: &crate::fleet_ensure::model::DesiredFleetArtifacts,
) -> FleetEnsureStateRecord {
    FleetEnsureStateRecord {
        active_registry: None,
        completed_reinstall_action_sha256: BTreeMap::new(),
        completed_reinstall_operation_id: None,
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
    root_runtime_status: &'a str,
    root_status_error: Option<canic_core::diagnostics::RegisteredDiagnosticCode>,
    store: &'a str,
    store_has_root_controller: bool,
    store_module_hash: &'a str,
}

#[cfg(unix)]
fn write_fake_icp(root: &Path, fixture: FakeIcpFixture<'_>) -> PathBuf {
    write_fake_icp_with_status_projection(root, fixture, Some(1), false)
}

#[cfg(unix)]
fn write_versionless_root_owned_fake_icp(root: &Path, fixture: FakeIcpFixture<'_>) -> PathBuf {
    write_fake_icp_with_status_projection(root, fixture, None, true)
}

#[cfg(unix)]
#[expect(
    clippy::too_many_lines,
    reason = "one process-backed fixture keeps every accepted fake ICP command visible"
)]
fn write_fake_icp_with_status_projection(
    root: &Path,
    fixture: FakeIcpFixture<'_>,
    status_canister_version: Option<u64>,
    store_status_unavailable: bool,
) -> PathBuf {
    let FakeIcpFixture {
        authority,
        coordinator,
        coordinator_module_hash,
        fleet_root,
        operator,
        pool,
        public_cycle_balance,
        root_module_hash,
        root_runtime_status,
        root_status_error,
        store,
        store_has_root_controller,
        store_module_hash,
    } = fixture;
    let executable = root.join("fake-icp");
    let counter = root.join("root-status-count");
    let predecessor_pool_status = root.join("predecessor-pool-status");
    let root_started = root.join("root-started");
    let root_start_count = root.join("root-start-count");
    let store_status_count = root.join("store-status-count");
    if root_started.exists() {
        fs::remove_file(&root_started).expect("reset fake Root runtime state");
    }
    if root_start_count.exists() {
        fs::remove_file(&root_start_count).expect("reset fake Root start count");
    }
    if predecessor_pool_status.exists() {
        fs::remove_file(&predecessor_pool_status).expect("reset predecessor pool response");
    }
    if store_status_count.exists() {
        fs::remove_file(&store_status_count).expect("reset Store status counter");
    }
    let coordinator_status = canister_status_json(
        coordinator,
        operator,
        coordinator_module_hash.to_string(),
        270_000_000_000_000,
        "running",
        None,
        status_canister_version,
    );
    let root_status = canister_status_json(
        fleet_root,
        operator,
        root_module_hash.to_string(),
        30_000_000_000_000,
        root_runtime_status,
        None,
        status_canister_version,
    );
    let running_root_status = canister_status_json(
        fleet_root,
        operator,
        root_module_hash.to_string(),
        29_999_950_000_000,
        "running",
        None,
        status_canister_version,
    );
    let store_status = canister_status_json(
        store,
        operator,
        store_module_hash.to_string(),
        10_000_000_000_000,
        "running",
        store_has_root_controller.then_some(fleet_root),
        status_canister_version,
    );
    let store_status_command = if store_status_unavailable {
        format!(
            r#"if [ ! -f "{}" ]; then
      printf '%s\n' '1' > "{}"
      printf '%s\n' 'Store status unavailable' >&2
      exit 1
    fi
    printf '%s\n' '{store_status}'
    exit 0"#,
            store_status_count.display(),
            store_status_count.display(),
        )
    } else {
        format!("printf '%s\\n' '{store_status}'\n    exit 0")
    };
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
    let predecessor_pool_response = serde_json::json!({
        "response_bytes": hex_bytes(
            crate::fleet_ensure::ops::predecessor_root_status::encode_pool_response_fixture(pool)
        )
    })
    .to_string();
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
  printf '%s\n' 'icp 1.3.0'
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
    if [ -f "{root_started}" ]; then
      printf '%s\n' '{running_root_status}'
    else
      printf '%s\n' '{root_status}'
    fi
    exit 0
  fi
  if [ "$3" = "{store}" ]; then
    {store_status_command}
  fi
fi
if [ "$1" = "canister" ] && [ "$2" = "start" ] && [ "$3" = "{fleet_root}" ]; then
  count=0
  if [ -f "{root_start_count}" ]; then
    count=$(sed -n '1p' "{root_start_count}")
  fi
  count=$((count + 1))
  printf '%s\n' "$count" > "{root_start_count}"
  printf '%s\n' 'running' > "{root_started}"
  exit 0
fi
if [ "$1" = "canister" ] && [ "$2" = "call" ]; then
  {public_cycle_case}
  if [ "$4" = "icrc1_fee" ]; then
    printf '%s\n' '{ledger_response}'
    exit 0
  fi
  if [ "$3" = "{fleet_root}" ] && [ "$4" = "canic_status" ]; then
    count=0
    if [ -f "{counter}" ]; then
      count=$(sed -n '1p' "{counter}")
    fi
    if [ "$count" = "0" ]; then
      printf '%s\n' '1' > "{counter}"
      printf '%s\n' '{authority_response}'
    elif [ -f "{predecessor_pool_status}" ]; then
      printf '%s\n' '{predecessor_pool_response}'
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
        predecessor_pool_status = predecessor_pool_status.display(),
        root_start_count = root_start_count.display(),
        root_started = root_started.display(),
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

#[cfg(not(unix))]
fn write_versionless_root_owned_fake_icp(_root: &Path, _fixture: FakeIcpFixture<'_>) -> PathBuf {
    panic!("public generator fixture requires a Unix fake ICP executable")
}

fn canister_status_json(
    canister: &str,
    controller: &str,
    module_hash: String,
    cycles: u128,
    status: &str,
    second_controller: Option<&str>,
    canister_version: Option<u64>,
) -> String {
    let mut controllers = vec![controller];
    controllers.extend(second_controller);
    let mut status = serde_json::json!({
        "id": canister,
        "name": null,
        "status": status,
        "settings": { "controllers": controllers },
        "module_hash": module_hash,
        "memory_size": null,
        "cycles": cycles.to_string(),
        "reserved_cycles": null,
        "idle_cycles_burned_per_day": null
    });
    if let Some(canister_version) = canister_version {
        status["version"] = serde_json::json!(canister_version);
    }
    status.to_string()
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
        let release_identity = release_build_id.to_string();
        let path = format!("artifacts/{release_build_id}/{artifact_name}/{artifact_name}.wasm");
        let bytes = [
            b"\0asm\x01\0\0\0".as_slice(),
            &[marker],
            release_identity.as_bytes(),
        ]
        .concat();
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
            wasm_gz_relative_path: format!(
                "artifacts/{release_build_id}/{}.wasm.gz",
                role.as_str()
            ),
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

    assert_eq!(
        root_artifact.wasm_relative_path,
        format!("artifacts/{release_build_id}/root/root.wasm")
    );
    assert_eq!(
        candid_sidecar(&root, root_artifact).expect("resolve Root sidecar"),
        format!("artifacts/{release_build_id}/root/root.did")
    );

    let sidecar = root
        .join(&root_artifact.wasm_relative_path)
        .with_extension("did");
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
    let sidecar = root
        .join(&root_artifact.wasm_relative_path)
        .with_extension("did");
    let target = sidecar.with_file_name("target.did");
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
