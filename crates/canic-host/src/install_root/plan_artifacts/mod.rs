//! Module: install_root::plan_artifacts
//!
//! Responsibility: admit supplied-plan artifact bytes into one canonical install snapshot.
//! Does not own: deployment truth policy, network mutation, or activation sequencing.
//! Boundary: truth, manifest, and activation consumers receive the same prepared authority.

mod error;
mod prepared;

use crate::{
    canister_build::CurrentCanisterArtifactBuildOutput,
    deployment_truth::DeploymentPlanV1,
    install_root::{
        build_snapshot::{CompleteInstallBuildSnapshot, ValidatedInstallSnapshot},
        clock::current_unix_timestamp_label,
        operations::{EmitRootManifestOperation, InstallPhaseLabel},
        phase_receipts::CompletedInstallPhase,
    },
    release_build::{FinalizedReleaseBuild, finalize_release_build_from_manifest},
    release_set::{
        ApplicationArtifactFileBuildOutput, CanicInfrastructureArtifactBuildOutput,
        compile_and_persist_application_artifact_union,
        compile_and_persist_canic_infrastructure_artifact_manifest,
    },
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

use canic_core::ids::{CanisterRole, ReleaseBuildId};

#[cfg(test)]
use crate::release_set::artifact_root_path;

pub(super) use prepared::PreparedPlanArtifacts;

#[cfg(test)]
pub(super) use error::PlanArtifactError;

pub(super) struct EmittedInstallManifest {
    pub(super) phase: CompletedInstallPhase,
    pub(super) duration: Duration,
    pub(super) finalized_release_build: Option<FinalizedReleaseBuild>,
}

pub(super) fn prepare_plan_artifacts_with_phase(
    plan: &DeploymentPlanV1,
    icp_root: &Path,
    environment: &str,
) -> Result<(PreparedPlanArtifacts, CompletedInstallPhase, Duration), Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let started = Instant::now();
    let prepared = PreparedPlanArtifacts::materialize(plan, icp_root, environment)?;
    let duration = started.elapsed();
    let role_names = prepared
        .plan()
        .role_artifacts
        .iter()
        .map(|artifact| artifact.role.clone())
        .collect::<Vec<_>>();
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::MATERIALIZE_ARTIFACTS,
        attempted_action: "verify and materialize supplied deployment plan artifacts",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: vec![format!("deployment_plan:{}", prepared.plan().plan_id)],
        role_names,
    };
    Ok((prepared, phase, duration))
}

pub(super) fn emit_manifest_with_phase(
    icp_root: &Path,
    install_snapshot: &ValidatedInstallSnapshot,
    build_outputs: &[CurrentCanisterArtifactBuildOutput],
    infrastructure_build_outputs: &[CanicInfrastructureArtifactBuildOutput],
    plan_artifacts: Option<&PreparedPlanArtifacts>,
) -> Result<EmittedInstallManifest, Box<dyn std::error::Error>> {
    let emit_manifest_started_at_label = current_unix_timestamp_label()?;
    let emit_manifest_started_at = Instant::now();
    let (manifest_path, application_union_path, infrastructure_manifest_path) =
        if let Some(plan_artifacts) = plan_artifacts {
            (plan_artifacts.emit_release_set_manifest()?, None, None)
        } else {
            let complete_build = install_snapshot.complete_build.as_ref().ok_or_else(|| {
                "normal install is missing its complete-build snapshot".to_string()
            })?;
            let operation = EmitRootManifestOperation::new(&complete_build.manifest, build_outputs);
            let manifest_path = operation.execute()?;
            let release_build = install_snapshot
                .release_build
                .as_ref()
                .ok_or_else(|| "normal install is missing its planned release build".to_string())?;
            let application_outputs = application_file_build_outputs(
                complete_build,
                release_build.record.release_build_id,
                build_outputs,
            );
            let persisted = compile_and_persist_application_artifact_union(
                icp_root,
                &complete_build.component_topology,
                release_build.record.release_build_id,
                &complete_build.application_artifact_targets,
                &application_outputs,
            )?;
            let infrastructure = compile_and_persist_canic_infrastructure_artifact_manifest(
                icp_root,
                release_build.record.release_build_id,
                infrastructure_build_outputs,
            )?;
            (
                manifest_path,
                Some(persisted.path),
                Some(infrastructure.path),
            )
        };
    let emit_manifest_duration = emit_manifest_started_at.elapsed();
    let finalized_release_build = install_snapshot
        .release_build
        .as_ref()
        .map(|planned| {
            finalize_release_build_from_manifest(
                icp_root,
                planned.record.release_build_id,
                &manifest_path,
            )
        })
        .transpose()?;
    let mut evidence = EmitRootManifestOperation::evidence(&manifest_path);
    if let Some(path) = application_union_path {
        evidence.push(format!(
            "application_artifact_union_path:{}",
            path.display()
        ));
    }
    if let Some(path) = infrastructure_manifest_path {
        evidence.push(format!(
            "infrastructure_artifact_manifest_path:{}",
            path.display()
        ));
    }
    let phase = CompletedInstallPhase {
        phase: InstallPhaseLabel::EMIT_MANIFEST,
        attempted_action: "emit root release-set manifest",
        started_at: emit_manifest_started_at_label,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence,
        role_names: Vec::new(),
    };
    Ok(EmittedInstallManifest {
        phase,
        duration: emit_manifest_duration,
        finalized_release_build,
    })
}

fn application_file_build_outputs(
    complete_build: &CompleteInstallBuildSnapshot,
    release_build_id: ReleaseBuildId,
    build_outputs: &[CurrentCanisterArtifactBuildOutput],
) -> Vec<ApplicationArtifactFileBuildOutput> {
    build_outputs
        .iter()
        .filter(|output| {
            complete_build
                .application_artifact_targets
                .iter()
                .any(|target| target.role.as_str() == output.role)
        })
        .map(|output| ApplicationArtifactFileBuildOutput {
            role: CanisterRole::owned(output.role.clone()),
            package: output.output.package_name.clone(),
            release_build_id,
            wasm_path: output.output.wasm_path.clone(),
            wasm_gz_path: output.output.wasm_gz_path.clone(),
        })
        .collect()
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        canister_build::{
            CanisterArtifactBuildOutput, CanisterArtifactBuildSpec,
            CurrentCanisterArtifactBuildOutput,
        },
        install_root::{
            build_snapshot::{CompleteInstallBuildSnapshot, InstallBuildTarget},
            reused_build::load_reused_install_build,
        },
        release_build::{ReleaseBuildPlanState, load_release_build_plan, plan_release_build},
        release_set::{
            ApplicationArtifactBuildTarget, RootReleaseSetBuildSnapshot, RootReleaseSetBuildTarget,
            load_persisted_application_artifact_union,
            load_persisted_canic_infrastructure_artifact_manifest,
        },
        test_support::temp_dir,
    };
    use std::{fs, io::Write};

    use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};
    use flate2::{Compression, GzBuilder};

    const MINIMAL_WASM: &[u8] = b"\0asm\x01\0\0\0";

    #[test]
    fn normal_install_uses_current_local_root_wasm() {
        let icp_root = temp_dir("canic-install-root-artifact-authority");

        assert_eq!(
            artifact_root_path(&icp_root, "local")
                .join("root")
                .join("root.wasm"),
            icp_root.join(".icp/local/canisters/root/root.wasm")
        );
        let _ = fs::remove_dir_all(icp_root);
    }

    #[test]
    fn normal_manifest_phase_persists_application_union_before_finalization() {
        let root = temp_dir("normal-install-application-union");
        let plan = plan_release_build(&root).expect("plan release build");
        let release_build_id = plan.record.release_build_id;
        let topology = topology();
        let root_output = build_output(&root, "root", release_build_id);
        let app_output = build_output(&root, "app", release_build_id);
        let infrastructure_outputs = infrastructure_outputs(&root, release_build_id, &root_output);
        let complete_build = complete_build_snapshot(&root, &topology, &root_output, &app_output);
        let snapshot = ValidatedInstallSnapshot {
            app_id: "demo".to_string(),
            complete_build: Some(complete_build),
            release_build: Some(plan),
        };

        let emitted = emit_manifest_with_phase(
            &root,
            &snapshot,
            &[root_output, app_output],
            &infrastructure_outputs,
            None,
        )
        .expect("emit complete manifest authority");
        let finalized = emitted
            .finalized_release_build
            .expect("finalized release build");
        assert_eq!(finalized.record.release_build_id, release_build_id);
        let persisted =
            load_persisted_application_artifact_union(&root, &topology, release_build_id)
                .expect("durable application union");
        assert!(emitted.phase.evidence.contains(&format!(
            "application_artifact_union_path:{}",
            persisted.path.display()
        )));
        let infrastructure =
            load_persisted_canic_infrastructure_artifact_manifest(&root, release_build_id)
                .expect("durable infrastructure manifest");
        assert_eq!(infrastructure.manifest.release_build_id, release_build_id);
        assert_eq!(
            infrastructure
                .manifest
                .entries
                .iter()
                .map(|entry| (entry.role, entry.package.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    crate::release_set::CanicInfrastructureRole::FleetCoordinator,
                    "canic-fleet-coordinator",
                ),
                (
                    crate::release_set::CanicInfrastructureRole::FleetSubnetRoot,
                    "root-package",
                ),
                (
                    crate::release_set::CanicInfrastructureRole::WasmStore,
                    "canic-wasm-store",
                ),
            ]
        );
        assert!(emitted.phase.evidence.contains(&format!(
            "infrastructure_artifact_manifest_path:{}",
            infrastructure.path.display()
        )));
        assert!(matches!(
            load_release_build_plan(&root, release_build_id)
                .expect("load finalized release build")
                .state,
            ReleaseBuildPlanState::Finalized { .. }
        ));
        assert_finalized_release_is_reusable(&root, &snapshot, &finalized);

        fs::remove_dir_all(root).expect("remove temp root");
    }

    fn assert_finalized_release_is_reusable(
        root: &Path,
        snapshot: &ValidatedInstallSnapshot,
        finalized: &FinalizedReleaseBuild,
    ) {
        let release_build_id = finalized.record.release_build_id;
        let complete_build = snapshot
            .complete_build
            .as_ref()
            .expect("complete build snapshot");
        let reused = load_reused_install_build(root, complete_build, release_build_id)
            .expect("reuse finalized release build");
        assert_eq!(
            reused
                .outputs
                .iter()
                .map(|output| output.role.as_str())
                .collect::<Vec<_>>(),
            vec!["root", "app"]
        );
        assert_eq!(reused.infrastructure_outputs.len(), 3);
        let repeated = emit_manifest_with_phase(
            root,
            snapshot,
            &reused.outputs,
            &reused.infrastructure_outputs,
            None,
        )
        .expect("revalidate reused manifests and artifact bytes");
        assert_eq!(
            repeated
                .finalized_release_build
                .expect("repeated finalized release build")
                .record,
            finalized.record
        );

        let mut changed_app = complete_build.clone();
        changed_app.targets[1].spec.package_name = "different-package".to_string();
        assert!(
            load_reused_install_build(root, &changed_app, release_build_id).is_err(),
            "a finalized build from a different current App package must be rejected"
        );
    }

    #[test]
    fn application_union_failure_leaves_release_build_planned() {
        let root = temp_dir("normal-install-application-union-failure");
        let plan = plan_release_build(&root).expect("plan release build");
        let release_build_id = plan.record.release_build_id;
        let topology = topology();
        let root_output = build_output(&root, "root", release_build_id);
        let app_output = build_output(&root, "app", release_build_id);
        fs::write(
            &app_output.output.wasm_gz_path,
            gzip(b"\0asm\x01\0\0\0different"),
        )
        .expect("replace app gzip");
        let complete_build = complete_build_snapshot(&root, &topology, &root_output, &app_output);
        let snapshot = ValidatedInstallSnapshot {
            app_id: "demo".to_string(),
            complete_build: Some(complete_build),
            release_build: Some(plan),
        };

        assert!(
            emit_manifest_with_phase(&root, &snapshot, &[root_output, app_output], &[], None,)
                .is_err(),
            "representation mismatch must block finalization"
        );
        assert_eq!(
            load_release_build_plan(&root, release_build_id)
                .expect("load planned release build")
                .state,
            ReleaseBuildPlanState::Planned
        );

        fs::remove_dir_all(root).expect("remove temp root");
    }

    fn topology() -> canic_core::bootstrap::compiled::ComponentTopology {
        parse_config_model(
            r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[component_specs.default]
component_role = "app"
maximum_instances = 1
"#,
        )
        .expect("valid config")
        .compile_component_topology()
        .expect("Component Topology")
    }

    fn complete_build_snapshot(
        root: &Path,
        topology: &canic_core::bootstrap::compiled::ComponentTopology,
        root_output: &CurrentCanisterArtifactBuildOutput,
        app_output: &CurrentCanisterArtifactBuildOutput,
    ) -> CompleteInstallBuildSnapshot {
        CompleteInstallBuildSnapshot {
            targets: vec![
                install_build_target(root, root_output),
                install_build_target(root, app_output),
            ],
            manifest: RootReleaseSetBuildSnapshot {
                icp_root: root.to_path_buf(),
                manifest_path: root.join(".icp/local/canisters/root/root.release-set.json"),
                release_version: "test-version".to_string(),
                targets: vec![
                    RootReleaseSetBuildTarget {
                        role: "root".to_string(),
                        expected_wasm_gz_path: root_output.output.wasm_gz_path.clone(),
                        publish_entry: false,
                    },
                    RootReleaseSetBuildTarget {
                        role: "app".to_string(),
                        expected_wasm_gz_path: app_output.output.wasm_gz_path.clone(),
                        publish_entry: true,
                    },
                ],
            },
            component_topology: topology.clone(),
            application_artifact_targets: vec![ApplicationArtifactBuildTarget {
                role: CanisterRole::from("app"),
                package: "app-package".to_string(),
                wasm_relative_path: ".icp/local/canisters/app/app.wasm".to_string(),
                wasm_gz_relative_path: ".icp/local/canisters/app/app.wasm.gz".to_string(),
            }],
        }
    }

    fn install_build_target(
        root: &Path,
        output: &CurrentCanisterArtifactBuildOutput,
    ) -> InstallBuildTarget {
        InstallBuildTarget {
            role: output.role.clone(),
            spec: CanisterArtifactBuildSpec {
                role: output.role.clone(),
                package_name: output.output.package_name.clone(),
                package_version: output.output.package_version.clone(),
                package_manifest_path: root.join("Cargo.toml"),
                cargo_workspace_root: root.to_path_buf(),
                artifact_root: output.output.artifact_root.clone(),
                wasm_path: output.output.wasm_path.clone(),
                wasm_gz_path: output.output.wasm_gz_path.clone(),
                did_path: output.output.did_path.clone(),
            },
        }
    }

    fn build_output(
        root: &Path,
        role: &str,
        release_build_id: ReleaseBuildId,
    ) -> CurrentCanisterArtifactBuildOutput {
        let artifact_root = root.join(".icp/local/canisters").join(role);
        fs::create_dir_all(&artifact_root).expect("create artifact root");
        let wasm_path = artifact_root.join(format!("{role}.wasm"));
        let wasm_gz_path = artifact_root.join(format!("{role}.wasm.gz"));
        let mut wasm = MINIMAL_WASM.to_vec();
        wasm.extend_from_slice(release_build_id.to_string().as_bytes());
        fs::write(&wasm_path, &wasm).expect("write Wasm");
        fs::write(&wasm_gz_path, gzip(&wasm)).expect("write gzip Wasm");
        CurrentCanisterArtifactBuildOutput {
            role: role.to_string(),
            output: CanisterArtifactBuildOutput {
                package_name: format!("{role}-package"),
                package_version: "0.101.51".to_string(),
                artifact_root,
                wasm_path,
                wasm_gz_path,
                did_path: root.join(format!("{role}.did")),
                transforms: Vec::new(),
            },
        }
    }

    fn infrastructure_outputs(
        root: &Path,
        release_build_id: ReleaseBuildId,
        root_output: &CurrentCanisterArtifactBuildOutput,
    ) -> Vec<CanicInfrastructureArtifactBuildOutput> {
        let coordinator = build_output(root, "fleet_coordinator", release_build_id);
        let wasm_store = build_output(root, "wasm_store", release_build_id);
        vec![
            CanicInfrastructureArtifactBuildOutput {
                role: crate::release_set::CanicInfrastructureRole::FleetCoordinator,
                package: "canic-fleet-coordinator".to_string(),
                release_build_id,
                wasm_path: coordinator.output.wasm_path,
                wasm_gz_path: coordinator.output.wasm_gz_path,
            },
            CanicInfrastructureArtifactBuildOutput {
                role: crate::release_set::CanicInfrastructureRole::FleetSubnetRoot,
                package: "root-package".to_string(),
                release_build_id,
                wasm_path: root_output.output.wasm_path.clone(),
                wasm_gz_path: root_output.output.wasm_gz_path.clone(),
            },
            CanicInfrastructureArtifactBuildOutput {
                role: crate::release_set::CanicInfrastructureRole::WasmStore,
                package: "canic-wasm-store".to_string(),
                release_build_id,
                wasm_path: wasm_store.output.wasm_path,
                wasm_gz_path: wasm_store.output.wasm_gz_path,
            },
        ]
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::best());
        encoder.write_all(bytes).expect("write gzip");
        encoder.finish().expect("finish gzip")
    }
}
