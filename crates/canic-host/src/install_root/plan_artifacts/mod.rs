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
        build_snapshot::{
            CompleteInstallBuildSnapshot, ValidatedInstallSnapshot, WorkspaceInstallBuildSnapshot,
        },
        clock::current_unix_timestamp_label,
        operations::{EmitRootManifestOperation, InstallPhaseLabel},
        phase_receipts::CompletedInstallPhase,
        reused_build::validate_reused_install_build,
    },
    release_build::{
        FinalizedReleaseBuild, finalize_release_build_from_manifest,
        validate_finalized_release_build_manifest,
    },
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

struct InstallManifestPaths {
    manifest: std::path::PathBuf,
    application_union: Option<std::path::PathBuf>,
    infrastructure_manifest: Option<std::path::PathBuf>,
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
    let paths = emit_install_manifest_files(
        icp_root,
        install_snapshot,
        build_outputs,
        infrastructure_build_outputs,
        plan_artifacts,
    )?;
    let emit_manifest_duration = emit_manifest_started_at.elapsed();
    let finalized_release_build = install_snapshot.release_build.as_ref().map_or_else(
        || Ok(None),
        |planned| {
            let finalized = if matches!(
                install_snapshot.complete_build.as_ref(),
                Some(CompleteInstallBuildSnapshot::Finalized(_))
            ) {
                validate_finalized_release_build_manifest(
                    icp_root,
                    planned.record.release_build_id,
                    &paths.manifest,
                )?
            } else {
                finalize_release_build_from_manifest(
                    icp_root,
                    planned.record.release_build_id,
                    &paths.manifest,
                )?
            };
            Ok::<_, Box<dyn std::error::Error>>(Some(finalized))
        },
    )?;
    let mut evidence = EmitRootManifestOperation::evidence(&paths.manifest);
    if let Some(path) = paths.application_union {
        evidence.push(format!(
            "application_artifact_union_path:{}",
            path.display()
        ));
    }
    if let Some(path) = paths.infrastructure_manifest {
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

fn emit_install_manifest_files(
    icp_root: &Path,
    install_snapshot: &ValidatedInstallSnapshot,
    build_outputs: &[CurrentCanisterArtifactBuildOutput],
    infrastructure_build_outputs: &[CanicInfrastructureArtifactBuildOutput],
    plan_artifacts: Option<&PreparedPlanArtifacts>,
) -> Result<InstallManifestPaths, Box<dyn std::error::Error>> {
    if let Some(plan_artifacts) = plan_artifacts {
        return Ok(InstallManifestPaths {
            manifest: plan_artifacts.emit_release_set_manifest()?,
            application_union: None,
            infrastructure_manifest: None,
        });
    }
    let complete_build = install_snapshot
        .complete_build
        .as_ref()
        .ok_or_else(|| "normal install is missing its complete-build snapshot".to_string())?;
    let release_build = install_snapshot
        .release_build
        .as_ref()
        .ok_or_else(|| "normal install is missing its planned release build".to_string())?;
    match complete_build {
        CompleteInstallBuildSnapshot::Workspace(workspace) => {
            let operation = EmitRootManifestOperation::new(&workspace.manifest, build_outputs);
            let manifest = operation.execute()?;
            let application_outputs = application_file_build_outputs(
                workspace,
                release_build.record.release_build_id,
                build_outputs,
            );
            let application = compile_and_persist_application_artifact_union(
                icp_root,
                &workspace.component_topology,
                release_build.record.release_build_id,
                &workspace.application_artifact_targets,
                &application_outputs,
            )?;
            let infrastructure = compile_and_persist_canic_infrastructure_artifact_manifest(
                icp_root,
                release_build.record.release_build_id,
                infrastructure_build_outputs,
            )?;
            Ok(InstallManifestPaths {
                manifest,
                application_union: Some(application.path),
                infrastructure_manifest: Some(infrastructure.path),
            })
        }
        CompleteInstallBuildSnapshot::Finalized(finalized) => {
            if !build_outputs.is_empty() || !infrastructure_build_outputs.is_empty() {
                return Err(
                    "finalized artifact reuse unexpectedly received current build outputs".into(),
                );
            }
            validate_reused_install_build(icp_root, finalized)?;
            Ok(InstallManifestPaths {
                manifest: finalized.root_manifest_path.clone(),
                application_union: None,
                infrastructure_manifest: None,
            })
        }
    }
}

fn application_file_build_outputs(
    complete_build: &WorkspaceInstallBuildSnapshot,
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
            candid_sha256: output.output.candid_sha256,
            protocol_profile_digest: output.output.protocol_profile_digest,
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
            build_snapshot::{
                CompleteInstallBuildSnapshot, FinalizedInstallBuildSnapshot, InstallBuildTarget,
                WorkspaceInstallBuildSnapshot,
            },
            current_install_build_inputs,
            fleet_install_session::{PlanFleetInstallSessionRequest, plan_fleet_install_session},
            icp_context::InstallIcpContext,
            options::InstallRootOptions,
        },
        release_build::{
            PlannedReleaseBuild, ReleaseBuildPlanState, load_release_build_plan,
            plan_release_build, plan_test_release_build_for_builder,
        },
        release_set::{
            ApplicationArtifactBuildTarget, RootReleaseSetBuildSnapshot, RootReleaseSetBuildTarget,
            load_persisted_application_artifact_union,
            load_persisted_canic_infrastructure_artifact_manifest,
        },
        test_support::temp_dir,
    };
    use std::{collections::BTreeMap, fs, io::Write, path::PathBuf};

    use canic_core::{
        bootstrap::parse_config_model,
        ids::{CanisterRole, CanonicalNetworkId},
    };
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

    #[test]
    fn retained_1091_session_reuses_finalized_artifacts_without_cargo_or_artifact_mutation() {
        let fixture = retained_1091_fixture();
        let before = file_snapshot(&fixture.root);

        let (context, snapshot) = current_install_build_inputs(
            &fixture.root,
            &fixture.root,
            &fixture.config_path,
            &fixture.icp,
            &fixture.options,
        )
        .expect("resolve retained 0.109.1 build inputs without current role validation");
        assert_eq!(context.release_build_id, Some(fixture.release_build_id));
        let Some(CompleteInstallBuildSnapshot::Finalized(finalized)) =
            snapshot.complete_build.as_ref()
        else {
            panic!("retained session must select finalized-artifact authority");
        };
        assert_eq!(finalized.builder_version, "0.109.1");
        validate_reused_install_build(&fixture.root, finalized)
            .expect("validate retained manifests and bytes");
        let emitted = emit_manifest_with_phase(&fixture.root, &snapshot, &[], &[], None)
            .expect("reuse finalized root manifest without rewriting it");
        let finalized_release_build = emitted
            .finalized_release_build
            .as_ref()
            .expect("retained finalized build");
        assert_eq!(finalized_release_build.record.builder_version, "0.109.1");
        let replayed_session = plan_fleet_install_session(PlanFleetInstallSessionRequest {
            root: &fixture.root,
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_name: "primary".parse().expect("Fleet name"),
            app: "demo".into(),
            finalized_release_build,
            decision_release_build_id: None,
            fresh_fleet_plan_digest:
                "abababababababababababababababababababababababababababababababab",
        })
        .expect("reach exact retained Fleet install journal replay");
        assert_eq!(replayed_session.release_build_id, fixture.release_build_id);
        assert_eq!(file_snapshot(&fixture.root), before);

        assert_retained_recovery_drift_rejected(&fixture, finalized);
        fs::remove_dir_all(&fixture.root).expect("remove retained recovery fixture");
    }

    fn assert_retained_recovery_drift_rejected(
        fixture: &Retained1091Fixture,
        finalized: &FinalizedInstallBuildSnapshot,
    ) {
        let app_manifest = fixture.root.join("app/Cargo.toml");
        let app_source = fs::read_to_string(&app_manifest).expect("read App manifest");
        fs::write(
            &app_manifest,
            app_source.replace("name = \"app-package\"", "name = \"other-package\""),
        )
        .expect("change current App package identity");
        assert!(
            current_install_build_inputs(
                &fixture.root,
                &fixture.root,
                &fixture.config_path,
                &fixture.icp,
                &fixture.options,
            )
            .is_err(),
            "package drift must reject before reuse"
        );
        fs::write(&app_manifest, app_source).expect("restore App manifest");

        let config_source = fs::read_to_string(&fixture.config_path).expect("read App config");
        fs::write(
            &fixture.config_path,
            config_source.replace("maximum_instances = 1", "maximum_instances = 2"),
        )
        .expect("change Component topology");
        assert!(
            current_install_build_inputs(
                &fixture.root,
                &fixture.root,
                &fixture.config_path,
                &fixture.icp,
                &fixture.options,
            )
            .is_err(),
            "topology drift must reject before reuse"
        );
        fs::write(&fixture.config_path, config_source).expect("restore App config");

        let raw_app = fixture.root.join(".icp/local/canisters/app/app.wasm");
        let raw_app_bytes = fs::read(&raw_app).expect("read retained App Wasm");
        let mut changed_app_bytes = raw_app_bytes.clone();
        changed_app_bytes.push(0);
        fs::write(&raw_app, changed_app_bytes).expect("change retained App Wasm");
        assert!(
            validate_reused_install_build(&fixture.root, finalized).is_err(),
            "artifact byte drift must reject before replay"
        );
        fs::write(&raw_app, raw_app_bytes).expect("restore retained App Wasm");

        let root_manifest = finalized.root_manifest_path.clone();
        let root_manifest_bytes = fs::read(&root_manifest).expect("read retained root manifest");
        let mut changed_manifest = root_manifest_bytes.clone();
        changed_manifest.push(b'\n');
        fs::write(&root_manifest, changed_manifest).expect("change retained root manifest");
        assert!(
            current_install_build_inputs(
                &fixture.root,
                &fixture.root,
                &fixture.config_path,
                &fixture.icp,
                &fixture.options,
            )
            .is_err(),
            "manifest digest drift must reject before reuse"
        );
        fs::write(&root_manifest, root_manifest_bytes).expect("restore retained root manifest");
    }

    fn assert_finalized_release_is_reusable(
        root: &Path,
        snapshot: &ValidatedInstallSnapshot,
        finalized: &FinalizedReleaseBuild,
    ) {
        let release_build_id = finalized.record.release_build_id;
        let CompleteInstallBuildSnapshot::Workspace(workspace) = snapshot
            .complete_build
            .as_ref()
            .expect("complete build snapshot")
        else {
            panic!("fresh manifest test must start from a workspace snapshot");
        };
        let finalized_snapshot = FinalizedInstallBuildSnapshot {
            release_build_id,
            builder_version: "0.101.51".to_string(),
            root_role: CanisterRole::from("root"),
            root_manifest_path: workspace.manifest.manifest_path.clone(),
            component_topology: workspace.component_topology.clone(),
            package_by_role: std::collections::BTreeMap::from([
                (CanisterRole::from("root"), "root-package".to_string()),
                (CanisterRole::from("app"), "app-package".to_string()),
            ]),
        };
        let roles = validate_reused_install_build(root, &finalized_snapshot)
            .expect("reuse finalized release build");
        assert_eq!(
            roles,
            vec!["app", "root", "fleet_coordinator", "wasm_store"]
        );
        let repeated_snapshot = ValidatedInstallSnapshot {
            app_id: snapshot.app_id.clone(),
            complete_build: Some(CompleteInstallBuildSnapshot::Finalized(
                finalized_snapshot.clone(),
            )),
            release_build: Some(PlannedReleaseBuild {
                record: finalized.record.clone(),
                path: finalized.path.clone(),
            }),
        };
        let repeated = emit_manifest_with_phase(root, &repeated_snapshot, &[], &[], None)
            .expect("revalidate reused manifests and artifact bytes");
        assert_eq!(
            repeated
                .finalized_release_build
                .expect("repeated finalized release build")
                .record,
            finalized.record
        );

        let mut changed_app = finalized_snapshot;
        changed_app
            .package_by_role
            .insert(CanisterRole::from("app"), "different-package".to_string());
        assert!(
            validate_reused_install_build(root, &changed_app).is_err(),
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

    struct Retained1091Fixture {
        root: PathBuf,
        config_path: PathBuf,
        release_build_id: ReleaseBuildId,
        icp: InstallIcpContext,
        options: InstallRootOptions,
    }

    fn retained_1091_fixture() -> Retained1091Fixture {
        let root = temp_dir("retained-1091-install-recovery");
        fs::create_dir_all(&root).expect("create retained recovery root");
        let config_path = write_retained_1091_consumer_source(&root);
        let (release_build_id, finalized) = finalize_retained_1091_build(&root);
        retain_root_manifest_under_release_build(&root, release_build_id);
        plan_fleet_install_session(PlanFleetInstallSessionRequest {
            root: &root,
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_name: "primary".parse().expect("Fleet name"),
            app: "demo".into(),
            finalized_release_build: &finalized,
            decision_release_build_id: None,
            fresh_fleet_plan_digest:
                "abababababababababababababababababababababababababababababababab",
        })
        .expect("retain interrupted Fleet install session");
        let options = retained_1091_install_options(&root);
        let icp = InstallIcpContext::new("icp", &root, "proof");
        Retained1091Fixture {
            root,
            config_path,
            release_build_id,
            icp,
            options,
        }
    }

    fn write_retained_1091_consumer_source(root: &Path) -> PathBuf {
        fs::write(
            root.join("icp.yaml"),
            "environments:\n  - name: proof\n    network: ic\n",
        )
        .expect("write ICP environment");
        fs::write(
            root.join("Cargo.lock"),
            r#"version = 4

[[package]]
name = "canic"
version = "0.109.1"
"#,
        )
        .expect("write retained consumer graph");
        let config_path = root.join("canic.toml");
        fs::write(
            &config_path,
            r#"[app]
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
        .expect("write retained App config");
        write_consumer_package(root, "root", "root-package");
        write_consumer_package(root, "app", "app-package");
        config_path
    }

    fn finalize_retained_1091_build(root: &Path) -> (ReleaseBuildId, FinalizedReleaseBuild) {
        let plan = plan_test_release_build_for_builder(
            root,
            "0.109.1",
            crate::canister_build::CanisterBuildProfile::Release,
        )
        .expect("plan retained 0.109.1 release build");
        let release_build_id = plan.record.release_build_id;
        let topology = topology();
        let mut root_output = build_output(root, "root", release_build_id);
        root_output.output.protocol_release_identity = "0.109.1".to_string();
        let mut app_output = build_output(root, "app", release_build_id);
        app_output.output.protocol_release_identity = "0.109.1".to_string();
        let mut infrastructure_outputs =
            infrastructure_outputs(root, release_build_id, &root_output);
        for output in &mut infrastructure_outputs {
            output.protocol_release_identity = "0.109.1".to_string();
        }
        let complete_build = complete_build_snapshot(root, &topology, &root_output, &app_output);
        let install_snapshot = ValidatedInstallSnapshot {
            app_id: "demo".to_string(),
            complete_build: Some(complete_build),
            release_build: Some(plan),
        };
        let emitted = emit_manifest_with_phase(
            root,
            &install_snapshot,
            &[root_output, app_output],
            &infrastructure_outputs,
            None,
        )
        .expect("finalize retained 0.109.1 artifacts");
        let finalized = emitted
            .finalized_release_build
            .expect("retained finalized release");
        (release_build_id, finalized)
    }

    fn retain_root_manifest_under_release_build(root: &Path, release_build_id: ReleaseBuildId) {
        let retained_manifest_path = root
            .join(".canic/release-builds")
            .join(release_build_id.to_string())
            .join("artifacts/root/root.release-set.json");
        fs::create_dir_all(
            retained_manifest_path
                .parent()
                .expect("retained manifest parent"),
        )
        .expect("create retained manifest directory");
        fs::copy(
            root.join(".icp/local/canisters/root/root.release-set.json"),
            &retained_manifest_path,
        )
        .expect("retain exact finalized root manifest");
    }

    fn retained_1091_install_options(root: &Path) -> InstallRootOptions {
        InstallRootOptions {
            root_canister: "root".to_string(),
            root_build_target: "root".to_string(),
            icp_executable: "icp".to_string(),
            environment: "proof".to_string(),
            fleet_name: "primary".to_string(),
            icp_root: Some(root.to_path_buf()),
            build_profile: None,
            release_build_id: None,
            config_path: Some("canic.toml".to_string()),
            fleet_install_input_path: None,
            expected_fresh_fleet_plan_digest: None,
            admitted_fresh_fleet_plan_digest: None,
            expected_app: Some("demo".to_string()),
            retained_root_repair_adoption: None,
            retained_root_repair_funding_authorization: None,
            interactive_config_selection: false,
            deployment_plan_override: None,
        }
    }

    fn write_consumer_package(root: &Path, role: &str, package: &str) {
        let directory = root.join(role);
        fs::create_dir_all(&directory).expect("create retained consumer package");
        fs::write(
            directory.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{package}\"\nversion = \"test-version\"\nedition = \"2024\"\n"
            ),
        )
        .expect("write retained consumer package");
    }

    fn file_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn collect(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(directory).expect("read fixture directory") {
                let entry = entry.expect("read fixture entry");
                let path = entry.path();
                if entry.file_type().expect("fixture file type").is_dir() {
                    collect(root, &path, files);
                } else {
                    files.insert(
                        path.strip_prefix(root)
                            .expect("fixture relative path")
                            .to_path_buf(),
                        fs::read(path).expect("read fixture file"),
                    );
                }
            }
        }

        let mut files = BTreeMap::new();
        collect(root, root, &mut files);
        files
    }

    fn complete_build_snapshot(
        root: &Path,
        topology: &canic_core::bootstrap::compiled::ComponentTopology,
        root_output: &CurrentCanisterArtifactBuildOutput,
        app_output: &CurrentCanisterArtifactBuildOutput,
    ) -> CompleteInstallBuildSnapshot {
        CompleteInstallBuildSnapshot::Workspace(WorkspaceInstallBuildSnapshot {
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
        })
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
                canic_version: "0.101.51".to_string(),
                capabilities: std::collections::BTreeSet::new(),
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
                protocol_release_identity: "0.101.51".to_string(),
                protocol_role: canic_core::ids::CanisterRole::owned(role.to_string()),
                protocol_capabilities: std::collections::BTreeSet::new(),
                artifact_root,
                wasm_path,
                wasm_gz_path,
                did_path: root.join(format!("{role}.did")),
                candid_sha256: [3; 32],
                protocol_profile_digest:
                    canic_core::role_contract::ProtocolProfileDigest::from_bytes([4; 32]),
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
                protocol_release_identity: coordinator.output.protocol_release_identity,
                protocol_role: coordinator.output.protocol_role,
                protocol_capabilities: coordinator.output.protocol_capabilities,
                release_build_id,
                wasm_path: coordinator.output.wasm_path,
                wasm_gz_path: coordinator.output.wasm_gz_path,
                candid_sha256: coordinator.output.candid_sha256,
                protocol_profile_digest: coordinator.output.protocol_profile_digest,
            },
            CanicInfrastructureArtifactBuildOutput {
                role: crate::release_set::CanicInfrastructureRole::FleetSubnetRoot,
                package: "root-package".to_string(),
                protocol_release_identity: root_output.output.protocol_release_identity.clone(),
                protocol_role: root_output.output.protocol_role.clone(),
                protocol_capabilities: root_output.output.protocol_capabilities.clone(),
                release_build_id,
                wasm_path: root_output.output.wasm_path.clone(),
                wasm_gz_path: root_output.output.wasm_gz_path.clone(),
                candid_sha256: root_output.output.candid_sha256,
                protocol_profile_digest: root_output.output.protocol_profile_digest,
            },
            CanicInfrastructureArtifactBuildOutput {
                role: crate::release_set::CanicInfrastructureRole::WasmStore,
                package: "canic-wasm-store".to_string(),
                protocol_release_identity: wasm_store.output.protocol_release_identity,
                protocol_role: wasm_store.output.protocol_role,
                protocol_capabilities: wasm_store.output.protocol_capabilities,
                release_build_id,
                wasm_path: wasm_store.output.wasm_path,
                wasm_gz_path: wasm_store.output.wasm_gz_path,
                candid_sha256: wasm_store.output.candid_sha256,
                protocol_profile_digest: wasm_store.output.protocol_profile_digest,
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
