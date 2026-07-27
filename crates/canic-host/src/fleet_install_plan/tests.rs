//! Module: fleet_install_plan::tests
//!
//! Responsibility: verify immutable multi-root planning and release-set publication.
//! Does not own: network metadata, Canister creation, installation, or Registry mutation.
//! Boundary: exercises finalized-build, Fleet, topology, funding, path, and retry identity.

use std::{fs, io::Write, path::Path};

use candid::Principal;
use canic_core::{
    bootstrap::{compiled::ConfigModel, parse_config_model},
    cdk::types::Cycles,
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, CyclesFundingBudget, FleetBinding, FleetId,
        FleetKey, FleetSubnetRootLimits, ReleaseBuildId, SubnetId,
    },
};
use flate2::{Compression, GzBuilder};

use crate::{
    component_topology::RootComponentAdmissionInput,
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    release_set::{
        ApplicationArtifactBuildTarget, ApplicationArtifactFileBuildOutput,
        compile_and_persist_application_artifact_union,
    },
    test_support::temp_dir,
};

use super::*;

const CONFIG: &str = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[roles.beta]
kind = "canister"
package = "beta"

[roles.shared]
kind = "canister"
package = "shared"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 2

[component_specs.alpha.children.shared]
kind = "replica"

[component_specs.alpha.spawn_grants.alpha.shared]
maximum_instances_per_parent = 2

[component_specs.beta]
component_role = "beta"
maximum_instances = 1
"#;

fn config() -> ConfigModel {
    parse_config_model(CONFIG).expect("valid Fleet config")
}

fn fleet_binding(byte: u8) -> FleetBinding {
    FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::public_ic(),
            fleet_id: FleetId::from_generated_bytes([byte; 32]),
        },
        app: AppId::from("demo"),
    }
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(Principal::from_slice(&[byte; 29]))
}

fn limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 8,
        maximum_managed_canisters: 64,
        maximum_registry_bytes: 4_194_304,
        maximum_wasm_store_bytes: 40_000_000,
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(1_000_000_000_000),
        },
    }
}

fn admission(component_spec: &str, maximum_root_instances: u32) -> RootComponentAdmissionInput {
    RootComponentAdmissionInput {
        component_spec: component_spec.parse().expect("Component Spec ID"),
        maximum_root_instances,
    }
}

fn root_input(
    subnet_byte: u8,
    admissions: Vec<RootComponentAdmissionInput>,
) -> PlannedFleetSubnetRootInput {
    PlannedFleetSubnetRootInput {
        placement_subnet: subnet(subnet_byte),
        component_admissions: admissions,
        limits: limits(),
        creation_funding: PlannedCanisterCreationFunding::Cycles {
            cycles: 2_000_000_000_000,
        },
    }
}

fn coordinator() -> PlannedFleetCoordinator {
    PlannedFleetCoordinator {
        coordinator_subnet: subnet(4),
        creation_funding: PlannedCanisterCreationFunding::Icp { e8s: 25_000_000 },
    }
}

fn request<'a>(
    root: &'a Path,
    config: &'a ConfigModel,
    fleet: FleetBinding,
    release_build_id: ReleaseBuildId,
) -> FleetInstallPlanRequest<'a> {
    FleetInstallPlanRequest {
        root,
        config,
        fleet,
        release_build_id,
        coordinator: coordinator(),
        fleet_subnet_roots: vec![
            root_input(7, vec![admission("beta", 1), admission("alpha", 1)]),
            root_input(6, vec![admission("alpha", 1)]),
        ],
    }
}

#[test]
fn exact_multi_root_plan_and_manifests_are_immutable_and_idempotent() {
    let root = temp_dir("fleet-install-plan");
    let config = config();
    let release_build_id = prepare_finalized_release(&root, &config);
    let fleet = fleet_binding(3);

    let persisted = compile_and_persist_fleet_install_plan(request(
        &root,
        &config,
        fleet.clone(),
        release_build_id,
    ))
    .expect("persist Fleet install plan");
    let repeated = compile_and_persist_fleet_install_plan(request(
        &root,
        &config,
        fleet.clone(),
        release_build_id,
    ))
    .expect("repeat exact plan");

    assert_eq!(repeated, persisted);
    fs::remove_file(&persisted.path).expect("simulate interruption before plan publication");
    assert_eq!(
        compile_and_persist_fleet_install_plan(request(
            &root,
            &config,
            fleet.clone(),
            release_build_id,
        ))
        .expect("recover from exact root manifests"),
        persisted
    );
    assert_eq!(
        persisted.path,
        root.join(".canic")
            .join("recovery")
            .join("fleet-install-plans")
            .join(fleet.fleet.canonical_network_id.to_string())
            .join(fleet.fleet.fleet_id.to_string())
            .join(release_build_id.to_string())
            .join(FLEET_INSTALL_PLAN_FILE)
    );
    assert_eq!(
        persisted
            .plan
            .fleet_subnet_roots
            .iter()
            .map(|root| root.placement_subnet)
            .collect::<Vec<_>>(),
        vec![subnet(6), subnet(7)]
    );
    assert_eq!(
        persisted.plan.fleet_subnet_roots[1]
            .component_admissions
            .iter()
            .map(|admission| admission.component_spec.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "beta"]
    );
    assert_eq!(persisted.root_release_sets.len(), 2);
    let plan_json = fs::read_to_string(&persisted.path).expect("read canonical plan JSON");
    assert!(plan_json.contains(r#""cycles":"2000000000000""#));
    assert!(plan_json.contains(r#""maximum_cycles":"1000000000000""#));
    for release_set in &persisted.root_release_sets {
        let planned_root = persisted
            .plan
            .fleet_subnet_roots
            .iter()
            .find(|root| root.placement_subnet == release_set.placement_subnet)
            .expect("planned root");
        assert_eq!(
            planned_root.initial_release_set.manifest_digest,
            release_set.digest
        );
        assert_eq!(
            release_set
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("UTF-8 manifest filename"),
            format!("{}.json", release_set.placement_subnet)
        );
    }
    assert_eq!(
        load_persisted_fleet_install_plan(&root, &config, &fleet, release_build_id)
            .expect("load Fleet install plan"),
        persisted
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn plan_requires_finalized_release_build_authority() {
    let root = temp_dir("fleet-install-plan-finalized-build");
    let config = config();
    let release = plan_release_build(&root).expect("plan release build");
    let fleet = fleet_binding(7);

    std::assert_matches!(
        compile_and_persist_fleet_install_plan(request(
            &root,
            &config,
            fleet.clone(),
            release.record.release_build_id,
        )),
        Err(FleetInstallPlanError::ReleaseBuild(_))
    );
    assert!(
        !fleet_install_plan_path(&root, &fleet, release.record.release_build_id).exists(),
        "unfinalized release must not publish Fleet install authority"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn plan_rejects_nonpositive_funding_and_conflicting_identity() {
    let root = temp_dir("fleet-install-plan-conflict");
    let config = config();
    let release_build_id = prepare_finalized_release(&root, &config);
    let fleet = fleet_binding(8);
    let persisted = compile_and_persist_fleet_install_plan(request(
        &root,
        &config,
        fleet.clone(),
        release_build_id,
    ))
    .expect("persist Fleet install plan");

    let mut conflicting = request(&root, &config, fleet, release_build_id);
    conflicting.coordinator.creation_funding = PlannedCanisterCreationFunding::Cycles { cycles: 1 };
    std::assert_matches!(
        compile_and_persist_fleet_install_plan(conflicting),
        Err(FleetInstallPlanError::ConflictingPlan { .. })
    );

    let conflicting_manifest_fleet = fleet_binding(9);
    let conflicting_plan_path =
        fleet_install_plan_path(&root, &conflicting_manifest_fleet, release_build_id);
    let conflicting_manifest_path = root_release_set_path(&conflicting_plan_path, subnet(6));
    fs::create_dir_all(conflicting_manifest_path.parent().expect("manifest parent"))
        .expect("create manifest parent");
    fs::write(&conflicting_manifest_path, b"conflicting manifest")
        .expect("write conflicting manifest");
    std::assert_matches!(
        compile_and_persist_fleet_install_plan(request(
            &root,
            &config,
            conflicting_manifest_fleet,
            release_build_id,
        )),
        Err(FleetInstallPlanError::ConflictingRootReleaseSet { .. })
    );
    assert!(
        !conflicting_plan_path.exists(),
        "manifest conflict must not publish a plan"
    );

    let invalid_fleet = fleet_binding(15);
    let mut invalid = request(&root, &config, invalid_fleet.clone(), release_build_id);
    invalid.fleet_subnet_roots[0].creation_funding =
        PlannedCanisterCreationFunding::Cycles { cycles: 0 };
    std::assert_matches!(
        compile_and_persist_fleet_install_plan(invalid),
        Err(FleetInstallPlanError::NonPositiveCreationFunding { .. })
    );
    assert!(
        !fleet_install_plan_path(&root, &invalid_fleet, release_build_id).exists(),
        "invalid funding must not publish a plan"
    );
    assert!(persisted.path.exists());

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn different_fleets_may_plan_independent_roots_on_the_same_subnets() {
    let root = temp_dir("fleet-install-plan-shared-subnets");
    let config = config();
    let release_build_id = prepare_finalized_release(&root, &config);
    let first_fleet = fleet_binding(10);
    let second_fleet = fleet_binding(11);

    let first = compile_and_persist_fleet_install_plan(request(
        &root,
        &config,
        first_fleet,
        release_build_id,
    ))
    .expect("persist first Fleet");
    let second = compile_and_persist_fleet_install_plan(request(
        &root,
        &config,
        second_fleet,
        release_build_id,
    ))
    .expect("persist second Fleet");

    assert_ne!(first.path, second.path);
    assert_eq!(
        first
            .plan
            .fleet_subnet_roots
            .iter()
            .map(|root| root.placement_subnet)
            .collect::<Vec<_>>(),
        second
            .plan
            .fleet_subnet_roots
            .iter()
            .map(|root| root.placement_subnet)
            .collect::<Vec<_>>()
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn loader_rejects_noncanonical_plan_identity_and_manifest_bytes() {
    let root = temp_dir("fleet-install-plan-load-rejection");
    let config = config();
    let release_build_id = prepare_finalized_release(&root, &config);
    let fleet = fleet_binding(12);
    let persisted = compile_and_persist_fleet_install_plan(request(
        &root,
        &config,
        fleet.clone(),
        release_build_id,
    ))
    .expect("persist Fleet install plan");
    let canonical_plan = fs::read(&persisted.path).expect("read canonical plan");

    fs::write(
        &persisted.path,
        serde_json::to_vec_pretty(&persisted.plan).expect("pretty plan"),
    )
    .expect("replace plan with noncanonical bytes");
    std::assert_matches!(
        load_persisted_fleet_install_plan(&root, &config, &fleet, release_build_id),
        Err(FleetInstallPlanError::InvalidPlanDocument { .. })
    );

    let other_fleet = fleet_binding(13);
    let other = compile_and_persist_fleet_install_plan(request(
        &root,
        &config,
        other_fleet,
        release_build_id,
    ))
    .expect("persist other Fleet plan");
    fs::write(
        &persisted.path,
        fs::read(other.path).expect("read other plan"),
    )
    .expect("replace with identity-mismatched plan");
    std::assert_matches!(
        load_persisted_fleet_install_plan(&root, &config, &fleet, release_build_id),
        Err(FleetInstallPlanError::InvalidPlanDocument { .. })
    );

    fs::write(&persisted.path, canonical_plan).expect("restore canonical plan");
    let manifest = &persisted.root_release_sets[0];
    fs::write(
        &manifest.path,
        serde_json::to_vec_pretty(&manifest.manifest).expect("pretty manifest"),
    )
    .expect("replace manifest with noncanonical bytes");
    std::assert_matches!(
        load_persisted_fleet_install_plan(&root, &config, &fleet, release_build_id),
        Err(FleetInstallPlanError::InvalidRootReleaseSetDocument { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn loader_rejects_symlinked_plan_and_root_manifest() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("fleet-install-plan-symlink");
    let config = config();
    let release_build_id = prepare_finalized_release(&root, &config);
    let fleet = fleet_binding(14);
    let persisted = compile_and_persist_fleet_install_plan(request(
        &root,
        &config,
        fleet.clone(),
        release_build_id,
    ))
    .expect("persist Fleet install plan");

    let real_plan = persisted.path.with_file_name("real-plan.json");
    fs::rename(&persisted.path, &real_plan).expect("move plan");
    symlink(&real_plan, &persisted.path).expect("link plan");
    std::assert_matches!(
        load_persisted_fleet_install_plan(&root, &config, &fleet, release_build_id),
        Err(FleetInstallPlanError::UnsafePlan { .. })
    );
    fs::remove_file(&persisted.path).expect("remove plan link");
    fs::rename(real_plan, &persisted.path).expect("restore plan");

    let manifest = &persisted.root_release_sets[0];
    let real_manifest = manifest.path.with_file_name("real-manifest.json");
    fs::rename(&manifest.path, &real_manifest).expect("move manifest");
    symlink(&real_manifest, &manifest.path).expect("link manifest");
    std::assert_matches!(
        load_persisted_fleet_install_plan(&root, &config, &fleet, release_build_id),
        Err(FleetInstallPlanError::UnsafeRootReleaseSet { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

fn prepare_finalized_release(root: &Path, config: &ConfigModel) -> ReleaseBuildId {
    let release = plan_release_build(root).expect("plan release build");
    let release_build_id = release.record.release_build_id;
    let topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let targets = ["shared", "beta", "alpha"].map(target).to_vec();
    let outputs = ["shared", "beta", "alpha"]
        .map(|role| build_output(root, release_build_id, role))
        .to_vec();
    compile_and_persist_application_artifact_union(
        root,
        &topology,
        release_build_id,
        &targets,
        &outputs,
    )
    .expect("persist application union");
    let release_set = root.join("release-set.json");
    fs::write(&release_set, b"exact finalized release set").expect("write release set");
    finalize_release_build_from_manifest(root, release_build_id, &release_set)
        .expect("finalize release build");
    release_build_id
}

fn target(role: &str) -> ApplicationArtifactBuildTarget {
    ApplicationArtifactBuildTarget {
        role: CanisterRole::owned(role.to_string()),
        package: format!("{role}-package"),
        wasm_relative_path: format!(".icp/local/canisters/{role}/{role}.wasm"),
        wasm_gz_relative_path: format!(".icp/local/canisters/{role}/{role}.wasm.gz"),
    }
}

fn build_output(
    root: &Path,
    release_build_id: ReleaseBuildId,
    role: &str,
) -> ApplicationArtifactFileBuildOutput {
    let artifact_root = root.join(".icp/local/canisters").join(role);
    fs::create_dir_all(&artifact_root).expect("create artifact root");
    let wasm_path = artifact_root.join(format!("{role}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{role}.wasm.gz"));
    let mut wasm = crate::release_set::WASM_MAGIC.to_vec();
    wasm.extend_from_slice(role.as_bytes());
    fs::write(&wasm_path, &wasm).expect("write Wasm");
    fs::write(&wasm_gz_path, gzip(&wasm)).expect("write gzip Wasm");
    ApplicationArtifactFileBuildOutput {
        role: CanisterRole::owned(role.to_string()),
        release_build_id,
        wasm_path,
        wasm_gz_path,
    }
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(bytes).expect("write gzip");
    encoder.finish().expect("finish gzip")
}
