//! Module: release_set::application::tests
//!
//! Responsibility: verify qualified application artifact unions and exact root projections.
//! Does not own: Cargo execution, artifact persistence, Store publication, or installation.
//! Boundary: exercises topology completeness, build qualification, and admission-scoped output.

use std::io::Write;

use candid::Principal;
use canic_core::{
    bootstrap::{compiled::ConfigModel, parse_config_model},
    cdk::{types::Cycles, utils::hash::hex_bytes},
    ids::{
        AppId, CanonicalNetworkId, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits, ReleaseBuildId,
        ReleaseBuildNonce, SubnetId,
    },
};
use flate2::{Compression, GzBuilder};

use crate::component_topology::{
    FleetSubnetRootTopologyInput, RootComponentAdmissionInput, plan_fleet_topology,
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

[component_specs.alpha.provisions.beta]
maximum_instances_per_requester_per_root = 2

[component_specs.alpha.children.shared]
kind = "replica"

[component_specs.alpha.spawn_grants.alpha.shared]
maximum_instances_per_parent = 2

[component_specs.beta]
component_role = "beta"
maximum_instances = 2

[component_specs.beta.children.shared]
kind = "replica"

[component_specs.beta.spawn_grants.beta.shared]
maximum_instances_per_parent = 2
"#;

fn config() -> ConfigModel {
    parse_config_model(CONFIG).expect("valid application topology config")
}

fn config_with_component_role_descendant() -> ConfigModel {
    parse_config_model(&format!(
        "{CONFIG}\n\
         [component_specs.alpha.children.beta]\n\
         kind = \"instance\"\n\
         [component_specs.alpha.spawn_grants.alpha.beta]\n\
         maximum_instances_per_parent = 2\n"
    ))
    .expect("Component role reused as another Spec's potential descendant")
}

fn release_build(byte: u8) -> ReleaseBuildId {
    ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([byte; 32]))
}

fn authority() -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::public_ic(),
                    fleet_id: FleetId::from_generated_bytes([3; 32]),
                },
                app: AppId::from("demo"),
            },
            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[4; 29])),
            coordinator: Principal::from_slice(&[5; 29]),
        },
        epoch: 1,
    }
}

fn limits(maximum_wasm_store_bytes: u64) -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 10,
        maximum_managed_canisters: 1_000,
        maximum_registry_bytes: 4_194_304,
        maximum_wasm_store_bytes,
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(1_000_000_000_000_000),
        },
    }
}

fn admission(component_spec: &str) -> RootComponentAdmissionInput {
    RootComponentAdmissionInput {
        component_spec: component_spec.parse().expect("Component Spec ID"),
        maximum_root_instances: 1,
    }
}

fn root(
    root_byte: u8,
    subnet_byte: u8,
    component_specs: &[&str],
    maximum_wasm_store_bytes: u64,
) -> FleetSubnetRootTopologyInput {
    FleetSubnetRootTopologyInput {
        placement_subnet: SubnetId::from_principal(Principal::from_slice(&[subnet_byte; 29])),
        fleet_subnet_root: Principal::from_slice(&[root_byte; 29]),
        component_admissions: component_specs
            .iter()
            .map(|component_spec| admission(component_spec))
            .collect(),
        limits: limits(maximum_wasm_store_bytes),
    }
}

fn wasm(role: &str) -> Vec<u8> {
    let mut bytes = WASM_MAGIC.to_vec();
    bytes.extend_from_slice(role.as_bytes());
    bytes
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(bytes).expect("write gzip fixture");
    encoder.finish().expect("finish gzip fixture")
}

fn target(role: &str) -> ApplicationArtifactBuildTarget {
    ApplicationArtifactBuildTarget {
        role: CanisterRole::owned(role.to_string()),
        package: format!("{role}-package"),
        wasm_relative_path: format!(".icp/local/canisters/{role}/{role}.wasm"),
        wasm_gz_relative_path: format!(".icp/local/canisters/{role}/{role}.wasm.gz"),
    }
}

fn output(role: &str, release_build_id: ReleaseBuildId) -> ApplicationArtifactBuildOutput {
    let target = target(role);
    let wasm = wasm(role);
    ApplicationArtifactBuildOutput {
        role: target.role,
        package: target.package,
        release_build_id,
        wasm_relative_path: target.wasm_relative_path,
        wasm_gz_relative_path: target.wasm_gz_relative_path,
        wasm_gz: gzip(&wasm),
        wasm,
    }
}

fn build_inputs(
    release_build_id: ReleaseBuildId,
) -> (
    Vec<ApplicationArtifactBuildTarget>,
    Vec<ApplicationArtifactBuildOutput>,
) {
    (
        ["shared", "beta", "alpha"].map(target).to_vec(),
        ["shared", "beta", "alpha"]
            .map(|role| output(role, release_build_id))
            .to_vec(),
    )
}

fn compile_union(
    topology: &ComponentTopology,
    release_build_id: ReleaseBuildId,
) -> ApplicationArtifactUnion {
    let (targets, outputs) = build_inputs(release_build_id);
    ApplicationArtifactUnion::compile(topology, release_build_id, &targets, &outputs)
        .expect("qualified application artifact union")
}

fn complete_plan(maximum_wasm_store_bytes: u64) -> crate::component_topology::FleetTopologyPlan {
    plan_fleet_topology(
        &config(),
        authority(),
        vec![root(6, 7, &["beta", "alpha"], maximum_wasm_store_bytes)],
    )
    .expect("complete root topology plan")
}

#[test]
fn compiler_freezes_one_canonical_qualified_topology_union() {
    let topology = config()
        .compile_component_topology()
        .expect("Component Topology");
    let release_build_id = release_build(8);
    let (targets, outputs) = build_inputs(release_build_id);
    let union = ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs)
        .expect("artifact union");

    assert_eq!(union.release_build_id, release_build_id);
    assert_eq!(
        union.fleet_component_topology_digest,
        topology.digest().expect("topology digest")
    );
    assert_eq!(
        union
            .entries
            .iter()
            .map(|entry| entry.role.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "beta", "shared"]
    );
    for entry in &union.entries {
        assert_eq!(entry.package, format!("{}-package", entry.role));
        assert!(entry.wasm_size_bytes > 0);
        assert!(entry.wasm_gz_size_bytes > 0);
        assert_eq!(entry.wasm_sha256_hex.len(), 64);
        assert_eq!(entry.wasm_gz_sha256_hex.len(), 64);
    }

    let mut reversed_targets = targets;
    let mut reversed_outputs = outputs;
    reversed_targets.reverse();
    reversed_outputs.reverse();
    let equivalent = ApplicationArtifactUnion::compile(
        &topology,
        release_build_id,
        &reversed_targets,
        &reversed_outputs,
    )
    .expect("order-independent compilation");
    assert_eq!(
        union
            .canonical_bytes(&topology)
            .expect("canonical union bytes"),
        equivalent
            .canonical_bytes(&topology)
            .expect("equivalent canonical union bytes")
    );
    assert_eq!(
        union.digest(&topology).expect("union digest"),
        equivalent
            .digest(&topology)
            .expect("equivalent union digest")
    );
    assert_eq!(
        hex_bytes(union.digest(&topology).expect("frozen union digest")),
        "5526576a9a1475d1d842ff84946399abba5d0bb17395b6bcbdd6be1693b5e7d5"
    );
}

#[test]
fn compiler_rejects_incomplete_duplicate_or_orphan_role_sets() {
    let topology = config()
        .compile_component_topology()
        .expect("Component Topology");
    let release_build_id = release_build(9);
    let (mut targets, mut outputs) = build_inputs(release_build_id);

    targets.pop();
    std::assert_matches!(
        ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs),
        Err(ApplicationReleaseSetError::BuildTargetRoleSet { .. })
    );

    let (mut targets, _) = build_inputs(release_build_id);
    targets.push(targets[0].clone());
    std::assert_matches!(
        ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs),
        Err(ApplicationReleaseSetError::BuildTargetRoleSet { .. })
    );

    let (targets, _) = build_inputs(release_build_id);
    outputs.pop();
    std::assert_matches!(
        ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs),
        Err(ApplicationReleaseSetError::BuildOutputRoleSet { .. })
    );

    let (_, mut outputs) = build_inputs(release_build_id);
    outputs[0] = output("orphan", release_build_id);
    std::assert_matches!(
        ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs),
        Err(ApplicationReleaseSetError::BuildOutputRoleSet { .. })
    );
}

#[test]
fn compiler_rejects_outputs_outside_their_qualified_build_targets() {
    let topology = config()
        .compile_component_topology()
        .expect("Component Topology");
    let release_build_id = release_build(10);
    let (targets, mut outputs) = build_inputs(release_build_id);

    outputs[0].package = "wrong-package".to_string();
    std::assert_matches!(
        ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs),
        Err(ApplicationReleaseSetError::BuildOutputPackageMismatch { .. })
    );

    let (_, mut outputs) = build_inputs(release_build_id);
    outputs[0].wasm_gz_relative_path = "different/output.wasm.gz".to_string();
    std::assert_matches!(
        ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs),
        Err(ApplicationReleaseSetError::BuildOutputPathMismatch { .. })
    );

    let (_, mut outputs) = build_inputs(release_build_id);
    outputs[0].release_build_id = release_build(11);
    std::assert_matches!(
        ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs),
        Err(ApplicationReleaseSetError::ReleaseBuildMismatch { .. })
    );

    let (_, mut outputs) = build_inputs(release_build_id);
    outputs[0].wasm_gz = gzip(&wasm("different"));
    std::assert_matches!(
        ApplicationArtifactUnion::compile(&topology, release_build_id, &targets, &outputs),
        Err(ApplicationReleaseSetError::RepresentationMismatch { .. })
    );
}

#[test]
fn projection_preserves_every_spec_role_while_reusing_shared_artifact_evidence() {
    let plan = complete_plan(u64::MAX);
    let release_build_id = release_build(12);
    let union = compile_union(&plan.component_topology, release_build_id);
    let binding = &plan.fleet_subnet_roots[0];
    let manifest =
        FleetSubnetRootReleaseSetManifest::project(&plan.component_topology, binding, &union)
            .expect("root release-set projection");

    assert_eq!(
        manifest
            .entries
            .iter()
            .map(|entry| (
                entry.component_spec.as_str(),
                entry.kind,
                entry.artifact.role.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("alpha", ApplicationReleaseSetEntryKind::Component, "alpha"),
            (
                "alpha",
                ApplicationReleaseSetEntryKind::ComponentChild,
                "shared"
            ),
            ("beta", ApplicationReleaseSetEntryKind::Component, "beta"),
            (
                "beta",
                ApplicationReleaseSetEntryKind::ComponentChild,
                "shared"
            ),
        ]
    );
    assert_eq!(
        manifest.entries[1].artifact, manifest.entries[3].artifact,
        "one child artifact remains authorized under both exact Spec catalogs"
    );
    manifest
        .validate_against(&plan.component_topology, binding, &union)
        .expect("valid exact projection");
    assert_eq!(
        manifest
            .canonical_bytes(&plan.component_topology, binding, &union)
            .expect("manifest bytes"),
        manifest
            .canonical_bytes(&plan.component_topology, binding, &union)
            .expect("stable manifest bytes")
    );
    assert_eq!(
        hex_bytes(
            manifest
                .digest(&plan.component_topology, binding, &union)
                .expect("frozen manifest digest")
                .into_bytes()
        ),
        "5e2ed7334a65313a095bea33b4972dead5cc18c65ddf93cf59bb37d89875111c"
    );
}

#[test]
fn projection_stores_a_component_role_once_when_another_spec_admits_it_as_a_descendant() {
    let config = config_with_component_role_descendant();
    let plan = plan_fleet_topology(
        &config,
        authority(),
        vec![root(6, 7, &["beta", "alpha"], u64::MAX)],
    )
    .expect("topology with a Component role reused as a descendant");
    let union = compile_union(&plan.component_topology, release_build(16));
    let manifest = FleetSubnetRootReleaseSetManifest::project(
        &plan.component_topology,
        &plan.fleet_subnet_roots[0],
        &union,
    )
    .expect("root release-set projection");

    assert_eq!(
        union
            .entries
            .iter()
            .filter(|entry| entry.role.as_str() == "beta")
            .count(),
        1,
        "the artifact union carries one qualified beta Wasm"
    );
    let beta_uses = manifest
        .entries
        .iter()
        .filter(|entry| entry.artifact.role.as_str() == "beta")
        .collect::<Vec<_>>();
    assert_eq!(beta_uses.len(), 2);
    assert_eq!(
        beta_uses
            .iter()
            .map(|entry| (entry.component_spec.as_str(), entry.kind))
            .collect::<Vec<_>>(),
        vec![
            ("alpha", ApplicationReleaseSetEntryKind::ComponentChild),
            ("beta", ApplicationReleaseSetEntryKind::Component),
        ]
    );
    assert_eq!(beta_uses[0].artifact, beta_uses[1].artifact);
}

#[test]
fn separate_roots_receive_only_their_admitted_spec_closure() {
    let plan = plan_fleet_topology(
        &config(),
        authority(),
        vec![
            root(6, 7, &["alpha"], u64::MAX),
            root(8, 9, &["beta"], u64::MAX),
        ],
    )
    .expect("split topology plan");
    let union = compile_union(&plan.component_topology, release_build(13));

    for (binding, expected_spec) in plan.fleet_subnet_roots.iter().zip(["alpha", "beta"]) {
        let projected = plan
            .component_topology
            .project_for_admissions(&binding.component_admissions)
            .expect("root-local topology projection");
        let manifest =
            FleetSubnetRootReleaseSetManifest::project(&plan.component_topology, binding, &union)
                .expect("root-local projection");
        assert_eq!(manifest.entries.len(), 2);
        assert!(
            manifest
                .entries
                .iter()
                .all(|entry| entry.component_spec.as_str() == expected_spec)
        );
        assert_eq!(manifest.entries[1].artifact.role.as_str(), "shared");
        if expected_spec == "beta" {
            assert_eq!(
                projected.provisioning_grants.len(),
                1,
                "target root retains the incoming grant"
            );
            assert!(
                manifest
                    .entries
                    .iter()
                    .all(|entry| entry.artifact.role.as_str() != "alpha"),
                "an incoming grant must not pull requester artifacts into the root release set"
            );
        } else {
            assert!(projected.provisioning_grants.is_empty());
        }
    }
}

#[test]
fn store_limit_counts_exact_artifact_bytes_once_without_erasing_authorization() {
    let mut plan = complete_plan(u64::MAX);
    let release_build_id = release_build(14);
    let (targets, mut outputs) = build_inputs(release_build_id);
    let alpha_wasm = outputs
        .iter()
        .find(|output| output.role.as_str() == "alpha")
        .expect("alpha output")
        .wasm
        .clone();
    let beta = outputs
        .iter_mut()
        .find(|output| output.role.as_str() == "beta")
        .expect("beta output");
    beta.wasm = alpha_wasm;
    beta.wasm_gz = gzip(&beta.wasm);
    let union = ApplicationArtifactUnion::compile(
        &plan.component_topology,
        release_build_id,
        &targets,
        &outputs,
    )
    .expect("union with byte-identical distinct roles");

    let required_bytes = union
        .entries
        .iter()
        .map(|entry| {
            (
                entry.wasm_sha256_hex.as_str(),
                entry.wasm_gz_sha256_hex.as_str(),
                entry.wasm_gz_size_bytes,
            )
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|(_, _, size)| size)
        .sum();
    plan.fleet_subnet_roots[0].limits.maximum_wasm_store_bytes = required_bytes;
    let manifest = FleetSubnetRootReleaseSetManifest::project(
        &plan.component_topology,
        &plan.fleet_subnet_roots[0],
        &union,
    )
    .expect("exact Store byte ceiling");
    assert_eq!(manifest.entries.len(), 4);

    plan.fleet_subnet_roots[0].limits.maximum_wasm_store_bytes = required_bytes - 1;
    std::assert_matches!(
        FleetSubnetRootReleaseSetManifest::project(
            &plan.component_topology,
            &plan.fleet_subnet_roots[0],
            &union,
        ),
        Err(ApplicationReleaseSetError::WasmStoreLimitExceeded {
            maximum_bytes,
            required_bytes: actual,
        }) if maximum_bytes + 1 == actual
    );
}

#[test]
fn projection_rejects_cross_build_topology_and_manifest_tampering() {
    let plan = complete_plan(u64::MAX);
    let release_build_id = release_build(15);
    let union = compile_union(&plan.component_topology, release_build_id);
    let binding = &plan.fleet_subnet_roots[0];
    let manifest =
        FleetSubnetRootReleaseSetManifest::project(&plan.component_topology, binding, &union)
            .expect("root projection");

    let mut wrong_topology_union = union.clone();
    wrong_topology_union.fleet_component_topology_digest =
        canic_core::ids::ComponentTopologyDigest::from_bytes([16; 32]);
    std::assert_matches!(
        FleetSubnetRootReleaseSetManifest::project(
            &plan.component_topology,
            binding,
            &wrong_topology_union,
        ),
        Err(ApplicationReleaseSetError::UnionTopologyDigestMismatch { .. })
    );

    let mut incomplete_union = union.clone();
    incomplete_union.entries.pop();
    std::assert_matches!(
        incomplete_union.validate_against(&plan.component_topology),
        Err(ApplicationReleaseSetError::ArtifactUnionRoleSet { .. })
    );

    let mut wrong_build = manifest.clone();
    wrong_build.release_build_id = release_build(17);
    std::assert_matches!(
        wrong_build.validate_against(&plan.component_topology, binding, &union),
        Err(ApplicationReleaseSetError::ManifestBuildMismatch { .. })
    );

    let mut wrong_digest = manifest.clone();
    wrong_digest.component_topology_digest =
        canic_core::ids::ComponentTopologyDigest::from_bytes([18; 32]);
    std::assert_matches!(
        wrong_digest.validate_against(&plan.component_topology, binding, &union),
        Err(ApplicationReleaseSetError::ManifestTopologyDigestMismatch { .. })
    );

    let mut wrong_entries = manifest;
    wrong_entries.entries.pop();
    std::assert_matches!(
        wrong_entries.validate_against(&plan.component_topology, binding, &union),
        Err(ApplicationReleaseSetError::ProjectionMismatch)
    );
}
