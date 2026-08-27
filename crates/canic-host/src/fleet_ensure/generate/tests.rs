use super::*;
use crate::{
    fleet_ensure::{
        model::{
            CanisterRuntimeStatus, EffectRecord, EnsureAction, FleetEnsureStateRecord,
            FleetObservation, LiveCanister, RootOwnedCanisterLifecycle,
        },
        ops::{EffectObservation, EffectOutcome, EnsurePlatform},
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
use canic_core::{
    cdk::utils::hash::{hex_bytes, sha256_hex},
    dto::pool::{CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus},
    ids::{CanisterRole, FleetRegistryAuthority, FleetSubnetRootReleaseSet, ReleaseSetDigest},
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey},
};
use flate2::{Compression, GzBuilder};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
        coordinator: Principal::from_slice(&[3]).to_text(),
        treasury: None,
        cycles_ledger: mainnet_cycles_ledger(),
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
        coordinator: coordinator.clone(),
        treasury: None,
        cycles_ledger: mainnet_cycles_ledger(),
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
        &operator,
        &coordinator,
        &fleet_root,
        &retained_authority,
        &retained_pool,
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

    let mut platform = RetainedEnsurePlatform::new(&desired, &observed, &pool_one);
    let source_digest = "42".repeat(32);
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
                        controllers: canister.controllers.clone(),
                        cycles: observed.cycles,
                        module_sha256: canister.wasm.as_ref().map(|_| "00".repeat(32)),
                        principal,
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
        }
    }

    fn total_cycles(&self) -> u128 {
        self.live.values().map(|canister| canister.cycles).sum()
    }
}

impl EnsurePlatform for RetainedEnsurePlatform {
    type Error = io::Error;

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

#[cfg(unix)]
fn write_fake_icp(
    root: &Path,
    operator: &str,
    coordinator: &str,
    fleet_root: &str,
    authority: &FleetSubnetRootAuthority,
    pool: &CanisterPoolResponse,
) -> PathBuf {
    let executable = root.join("fake-icp");
    let counter = root.join("root-status-count");
    let coordinator_status =
        canister_status_json(coordinator, operator, "83".repeat(32), 270_000_000_000_000);
    let root_status =
        canister_status_json(fleet_root, operator, "84".repeat(32), 30_000_000_000_000);
    let authority_response = candid_response_json(&Ok::<_, canic_core::dto::error::Error>(
        RootEstateStatusResponse::FleetAuthority(Box::new(authority.clone())),
    ));
    let pool_response = candid_response_json(&Ok::<_, canic_core::dto::error::Error>(
        RootEstateStatusResponse::Pool(Box::new(pool.clone())),
    ));
    let ledger_response = candid_response_json(&Nat::from(100_000_000_u64));
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
if [ "$1" = "canister" ] && [ "$2" = "status" ]; then
  if [ "$3" = "{coordinator}" ]; then
    printf '%s\n' '{coordinator_status}'
    exit 0
  fi
  if [ "$3" = "{fleet_root}" ]; then
    printf '%s\n' '{root_status}'
    exit 0
  fi
fi
if [ "$1" = "canister" ] && [ "$2" = "call" ]; then
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

#[cfg(not(unix))]
fn write_fake_icp(
    _root: &Path,
    _operator: &str,
    _coordinator: &str,
    _fleet_root: &str,
    _authority: &FleetSubnetRootAuthority,
    _pool: &CanisterPoolResponse,
) -> PathBuf {
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
        CanicInfrastructureRole::WasmStore,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, role)| {
        let marker = u8::try_from(index).expect("three infrastructure artifacts");
        let path = format!("artifacts/{}.wasm", role.as_str());
        let bytes = [b"\0asm\x01\0\0\0".as_slice(), &[marker]].concat();
        let absolute = root.join(&path);
        fs::create_dir_all(absolute.parent().expect("artifact parent"))
            .expect("create artifact parent");
        fs::write(&absolute, &bytes).expect("write artifact");
        fs::write(
            absolute
                .parent()
                .expect("artifact parent")
                .join(format!("{}.did", role.as_str())),
            b"service : {};",
        )
        .expect("write Candid");
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

fn principal_text(byte: u8) -> String {
    Principal::from_slice(&[byte; 29]).to_text()
}
