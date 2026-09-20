//! Module: pic::fleet_registry::baseline::tests::release_artifacts
//!
//! Responsibility: build, seal and retain exact fixture release artifacts.
//! Boundary: cache build helpers and Cargo inputs independently of journey assertions.

mod audit_root;
#[cfg(test)]
pub(super) mod tests;

use crate::pic::{
    CanicWasmBuildProfile,
    artifacts::{
        INTERNAL_TEST_RELEASE_BUILD_ID, internal_test_artifact_maintenance_interval,
        internal_test_artifact_prune_policy, report_artifact_cache_maintenance,
        with_canonical_root_cargo_inputs,
    },
    timing::Span,
};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    ids::{BuildNetwork, CanisterRole, ReleaseBuildId, ReleaseBuildNonce},
};
use canic_host::{
    canister_build::{
        CanisterArtifactBuildOutput, CanisterArtifactBuilder, CanisterBuildProfile,
        WorkspaceBuildContext,
    },
    release_build::finalize_release_build_from_manifest,
    release_set::{
        AppConfigSnapshot, ApplicationArtifactBuildTarget, ApplicationArtifactFileBuildOutput,
        CanicInfrastructureArtifactBuildOutput, CanicInfrastructureRole,
        compile_and_persist_application_artifact_union,
        compile_and_persist_canic_infrastructure_artifact_manifest,
        compile_and_persist_current_release_set_manifest,
    },
    role_contract::{
        PackageValidationMode, RolePackageValidation, validate_declared_role_packages,
    },
};
use ciborium::Value;
use ic_testkit::artifacts::{
    ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheSpec, WasmBuildSpec,
    prepare_artifact_cache, resolve_cargo_build_inputs,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Instant,
};

const BUILD_HELPERS: [&str; 3] = [
    "crates/canic-testing-internal/src/pic/fleet_registry/baseline/tests/release_artifacts/mod.rs",
    "crates/canic-testing-internal/src/pic/fleet_registry/baseline/tests/release_artifacts/audit_root/mod.rs",
    "crates/canic-testing-internal/src/pic/artifacts.rs",
];

/// Exact selected release files staged for one disposable journey.
pub(super) struct LiteralZeroReleaseArtifacts {
    pub(super) component_wasms: BTreeMap<CanisterRole, Vec<u8>>,
    pub(super) coordinator_candid: String,
    pub(super) coordinator_wasm: String,
    pub(super) release_build_id: ReleaseBuildId,
    pub(super) root_candid: String,
    pub(super) root_wasm: String,
    pub(super) root_wasm_bytes: Vec<u8>,
    pub(super) store_candid: String,
    pub(super) store_wasm: String,
    pub(super) store_wasm_bytes: Vec<u8>,
}

pub(super) fn build_literal_zero_release_artifacts(
    workspace_root: &Path,
    adapter_root: &Path,
    config_path: &Path,
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    configured_roles: &[String],
    build_network: BuildNetwork,
    release_nonce: [u8; 32],
) -> LiteralZeroReleaseArtifacts {
    let span = Span::start("release_artifact_resolution");
    let mut phase = Span::start("artifact_recipe");
    let release_build_id =
        persist_internal_test_release_build_plan(adapter_root, build_network, release_nonce);
    let fixtures = prepare_generated_fixture_artifacts(
        adapter_root,
        &configuration.component_topology,
        release_build_id,
        configured_roles,
    );
    let mut outputs =
        literal_zero_release_artifact_outputs(adapter_root, release_build_id, configured_roles);
    append_generated_fixture_outputs(adapter_root, &fixtures, &mut outputs);
    let cache = literal_zero_release_artifact_cache_spec(
        workspace_root,
        &workspace_root.join("target/test-artifacts/external-artifact-cache"),
        config_path,
        configured_roles,
        &outputs,
        build_network,
        release_build_id,
    );
    let cache = bind_generated_fixture_cache_inputs(cache, adapter_root, &fixtures);
    phase = phase.next("artifact_cache_lookup");
    let started_at = Instant::now();
    let outcome = match prepare_artifact_cache(&cache)
        .expect("prepare literal-zero release artifact cache")
    {
        ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
        ArtifactCachePreparation::Build(transaction) => {
            phase = phase.next("artifact_build_and_seal");
            build_and_seal_literal_zero_release_artifacts(
                workspace_root,
                adapter_root,
                config_path,
                configuration,
                configured_roles,
                release_build_id,
                build_network,
            );
            phase = phase.next("artifact_cache_commit");
            for (name, path) in &outputs {
                transaction
                    .import_output(name, path)
                    .unwrap_or_else(|error| {
                        panic!("import literal-zero release artifact `{name}`: {error}")
                    });
            }
            transaction
                .commit()
                .expect("commit literal-zero release artifact cache")
        }
    };
    crate::pic::progress::timed(
        "FLEET",
        if outcome.is_reused() {
            crate::pic::progress::ProgressStatus::Cache
        } else {
            crate::pic::progress::ProgressStatus::Done
        },
        &format!(
            "literal-zero release artifacts ({})",
            outcome.record().artifacts().len()
        ),
        started_at.elapsed(),
    );
    crate::pic::progress::detail(
        "FLEET",
        &format!("literal-zero release artifact cache: {outcome}"),
    );
    report_artifact_cache_maintenance(
        "literal-zero-release-artifacts",
        outcome.record().maintenance(),
    );

    stage_retained_release_artifacts(outcome.record(), &outputs);

    phase = phase.next("artifact_load");
    let artifacts =
        load_literal_zero_release_artifacts(adapter_root, release_build_id, configured_roles);
    phase.finish();
    span.finish();
    artifacts
}

pub(super) fn literal_zero_release_artifact_cache_spec(
    workspace_root: &Path,
    cache_root: &Path,
    config_path: &Path,
    configured_roles: &[String],
    outputs: &BTreeMap<String, PathBuf>,
    build_network: BuildNetwork,
    release_build_id: ReleaseBuildId,
) -> ArtifactCacheSpec {
    let snapshot = AppConfigSnapshot::load(config_path)
        .expect("load literal-zero release build config for Cargo inputs");
    let mut packages = BTreeSet::from(["canic".to_string(), "canic-host".to_string()]);
    if audit_root::uses_audit_root(config_path) {
        packages.insert("root_probe".to_string());
    }
    let roles = configured_roles
        .iter()
        .map(|role| CanisterRole::from(role.clone()))
        .filter(|role| !role.is_root())
        .collect::<Vec<_>>();
    let validations = validate_declared_role_packages(
        config_path,
        snapshot.model(),
        &roles,
        PackageValidationMode::Passive,
    );
    for (role, validation) in roles.iter().zip(validations) {
        let RolePackageValidation::Supported(evidence) = validation else {
            panic!("literal-zero role `{role}` must resolve to one supported package");
        };
        packages.insert(evidence.role_package_name);
    }
    let packages = packages.iter().map(String::as_str).collect::<Vec<_>>();
    let network = build_network.to_string();
    let release_identity = release_build_id.to_string();
    let environment = [
        ("CARGO_INCREMENTAL", "0"),
        ("ICP_ENVIRONMENT", network.as_str()),
        (INTERNAL_TEST_RELEASE_BUILD_ID.0, release_identity.as_str()),
    ];
    let cargo_build = WasmBuildSpec::new(
        workspace_root,
        &canic_host::canister_build::canister_build_target_root(workspace_root),
        &packages,
        CanisterBuildProfile::Fast.target_dir_name(),
    )
    .with_cargo_profile_args(["--profile", "fast", "--locked"])
    .with_extra_env(environment);
    let config_relative = config_path
        .strip_prefix(workspace_root)
        .expect("literal-zero config must be workspace-confined")
        .to_str()
        .expect("literal-zero config path UTF-8");
    let mut cache = ArtifactCacheSpec::new(
        cache_root,
        "literal-zero-release-artifacts",
        "canic/literal-zero-release-artifacts/v1",
    )
    .with_coordination_scope("canic-external-artifact-builds")
    .with_arguments([
        "literal-zero-release-build",
        "fast",
        network.as_str(),
        config_relative,
    ])
    .with_environment(environment)
    .with_input("build-config", config_path)
    .with_input("icp-config", &workspace_root.join("icp.yaml"))
    .with_prune_policy_at_most_every(
        internal_test_artifact_prune_policy(),
        internal_test_artifact_maintenance_interval(),
    );
    cache = with_canonical_root_cargo_inputs(
        cache,
        config_path,
        &canic_host::canister_build::canister_build_target_root(workspace_root),
        CanicWasmBuildProfile::Fast,
        &environment,
    );
    // All three generated packages live beneath the selected audit package.
    // Materialize their manifests and locks before freezing that package's inputs.
    let context = literal_zero_build_context(
        workspace_root,
        workspace_root,
        config_path,
        build_network,
        release_build_id,
    );
    canic_host::canister_build::prepare_workspace_infrastructure_packages(&context)
        .expect("prepare infrastructure packages before freezing fixture inputs");
    let cargo_inputs = resolve_cargo_build_inputs(&cargo_build)
        .expect("resolve literal-zero release Cargo build inputs");
    cache =
        cache.with_cargo_build_inputs("literal-zero-release-cargo", &cargo_build, &cargo_inputs);
    cache = bind_build_helper_inputs(cache, workspace_root);
    for (name, path) in outputs {
        cache = cache.with_output(name, path);
    }
    cache
}

pub(super) fn prepare_generated_fixture_artifacts(
    root: &Path,
    topology: &canic_core::bootstrap::compiled::ComponentTopology,
    release: ReleaseBuildId,
    roles: &[String],
) -> canic_host::release_set::fixture::PersistedFixtureArtifactManifest {
    let inputs = if roles.iter().any(|role| role == "user_shard") {
        let path = root.join("fixture-source/user_shard.bin");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"reviewed fixture source").unwrap();
        vec![canic_host::release_set::fixture::FixtureSourceInput {
            role: "user_shard".into(),
            format_hash: [2; 32],
            completion_summary: [3; 32],
            chunk_paths: vec!["fixture-source/user_shard.bin".to_string()],
        }]
    } else {
        Vec::new()
    };
    canic_host::release_set::fixture::compile_and_persist_fixture_artifact_manifest(
        root, topology, release, &inputs,
    )
    .expect("retain neutral generated fixture authority before artifact caching")
}

pub(super) fn bind_generated_fixture_cache_inputs(
    mut cache: ArtifactCacheSpec,
    root: &Path,
    fixtures: &canic_host::release_set::fixture::PersistedFixtureArtifactManifest,
) -> ArtifactCacheSpec {
    let selection = root.join("fixture-source/selection.json");
    std::fs::create_dir_all(selection.parent().unwrap()).unwrap();
    std::fs::write(&selection, serde_json::to_vec(&fixtures.manifest).unwrap()).unwrap();
    cache = cache.with_input("fixture-selection", &selection);
    if !fixtures.manifest.entries.is_empty() {
        cache = cache.with_input(
            "fixture-source",
            &root.join("fixture-source/user_shard.bin"),
        );
    }
    cache
}

pub(super) fn append_generated_fixture_outputs(
    root: &Path,
    fixtures: &canic_host::release_set::fixture::PersistedFixtureArtifactManifest,
    outputs: &mut BTreeMap<String, PathBuf>,
) {
    let release_root = literal_zero_release_root(root, fixtures.manifest.release_build_id);
    for entry in &fixtures.manifest.entries {
        let content = hex_bytes(entry.content_id);
        for index in 0..entry.descriptor.chunks.len() {
            outputs.insert(
                format!("fixture-{content}-{index}"),
                release_root.join(format!("fixture-content/{content}/{index}.bin")),
            );
        }
    }
}

pub(super) fn literal_zero_release_artifact_outputs(
    adapter_root: &Path,
    release_build_id: ReleaseBuildId,
    configured_roles: &[String],
) -> BTreeMap<String, PathBuf> {
    let release_root = literal_zero_release_root(adapter_root, release_build_id);
    let mut outputs = BTreeMap::from([
        (
            "application-manifest".to_string(),
            release_root.join("application-artifact-union.json"),
        ),
        (
            "current-manifest".to_string(),
            release_root.join("current-release-set-manifest.json"),
        ),
        (
            "fixture-manifest".to_string(),
            release_root.join("fixture-artifact-manifest.json"),
        ),
        (
            "infrastructure-manifest".to_string(),
            release_root.join("infrastructure-artifact-manifest.json"),
        ),
        ("release-plan".to_string(), release_root.join("plan.cbor")),
    ]);
    let mut roles = configured_roles.iter().cloned().collect::<BTreeSet<_>>();
    roles.extend([
        CanicInfrastructureRole::FleetCoordinator
            .as_str()
            .to_string(),
        CanicInfrastructureRole::WasmStore.as_str().to_string(),
    ]);
    for role in roles {
        for extension in ["did", "wasm", "wasm.gz"] {
            outputs.insert(
                format!("{role}-{extension}"),
                literal_zero_role_artifact_path(adapter_root, release_build_id, &role, extension),
            );
        }
    }
    outputs
}

fn load_literal_zero_release_artifacts(
    adapter_root: &Path,
    release_build_id: ReleaseBuildId,
    configured_roles: &[String],
) -> LiteralZeroReleaseArtifacts {
    let artifact_path = |role: &str, extension: &str| {
        literal_zero_role_artifact_path(adapter_root, release_build_id, role, extension)
    };
    let coordinator_role = CanicInfrastructureRole::FleetCoordinator.as_str();
    let root_role = CanisterRole::ROOT.as_str();
    let store_role = CanicInfrastructureRole::WasmStore.as_str();
    let component_wasms = configured_roles
        .iter()
        .filter(|role| role.as_str() != root_role)
        .map(|role| {
            (
                CanisterRole::from(role.clone()),
                std::fs::read(artifact_path(role, "wasm"))
                    .expect("read cached literal-zero Component Wasm"),
            )
        })
        .collect();
    let coordinator_wasm_path = artifact_path(coordinator_role, "wasm");
    let root_wasm_path = artifact_path(root_role, "wasm");
    let store_wasm_path = artifact_path(store_role, "wasm");

    LiteralZeroReleaseArtifacts {
        component_wasms,
        coordinator_candid: relative_artifact_path(
            adapter_root,
            &artifact_path(coordinator_role, "did"),
        ),
        coordinator_wasm: relative_artifact_path(adapter_root, &coordinator_wasm_path),
        release_build_id,
        root_candid: relative_artifact_path(adapter_root, &artifact_path(root_role, "did")),
        root_wasm: relative_artifact_path(adapter_root, &root_wasm_path),
        root_wasm_bytes: std::fs::read(root_wasm_path).expect("read cached literal-zero Root Wasm"),
        store_candid: relative_artifact_path(adapter_root, &artifact_path(store_role, "did")),
        store_wasm: relative_artifact_path(adapter_root, &store_wasm_path),
        store_wasm_bytes: std::fs::read(store_wasm_path)
            .expect("read cached literal-zero Store Wasm"),
    }
}

pub(super) fn literal_zero_release_root(
    adapter_root: &Path,
    release_build_id: ReleaseBuildId,
) -> PathBuf {
    adapter_root
        .join(".canic/release-builds")
        .join(release_build_id.to_string())
}

pub(super) fn literal_zero_role_artifact_path(
    adapter_root: &Path,
    release_build_id: ReleaseBuildId,
    role: &str,
    extension: &str,
) -> PathBuf {
    literal_zero_release_root(adapter_root, release_build_id)
        .join("artifacts")
        .join(role)
        .join(format!("{role}.{extension}"))
}

fn literal_zero_build_context(
    workspace_root: &Path,
    adapter_root: &Path,
    config_path: &Path,
    build_network: BuildNetwork,
    release_build_id: ReleaseBuildId,
) -> WorkspaceBuildContext {
    WorkspaceBuildContext {
        role: "root".into(),
        profile: CanisterBuildProfile::Fast,
        environment: build_network.to_string(),
        build_network,
        workspace_root: workspace_root.to_path_buf(),
        icp_root: adapter_root.to_path_buf(),
        config_path: config_path.to_path_buf(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: Some(release_build_id),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one helper builds and seals the complete literal-zero release authority"
)]
fn build_and_seal_literal_zero_release_artifacts(
    workspace_root: &Path,
    adapter_root: &Path,
    config_path: &Path,
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    configured_roles: &[String],
    release_build_id: ReleaseBuildId,
    build_network: BuildNetwork,
) {
    let context = literal_zero_build_context(
        workspace_root,
        adapter_root,
        config_path,
        build_network,
        release_build_id,
    );
    let builder = CanisterArtifactBuilder::for_profile(context.profile)
        .expect("preflight literal-zero artifact toolchain");
    // Match the production App path: Cargo stays serial while captured
    // infrastructure outputs finalize alongside later compilation.
    let mut phase = Span::start("app_artifacts_build");
    let audit_root = audit_root::uses_audit_root(config_path);
    let compiled_roles = configured_roles
        .iter()
        .filter(|role| !audit_root || role.as_str() != "root")
        .cloned()
        .collect::<Vec<_>>();
    let app = builder
        .build_workspace_app_artifacts(&context, &compiled_roles)
        .expect("build literal-zero App artifacts");
    let coordinator = app.coordinator.output;
    let store = app.store.output;
    let configured = app.configured;
    let root = if audit_root {
        // This fixture installs the audit Root; do not compile a canonical
        // Root only to discard its declaration, runtime and finalization.
        audit_root::build_audit_root(&context)
    } else {
        configured
            .iter()
            .find(|output| output.role == "root")
            .expect("literal-zero Root artifact")
            .output
            .clone()
    };
    let components = configured
        .iter()
        .filter(|output| output.role != "root")
        .map(|output| {
            (
                CanisterRole::from(output.role.clone()),
                output.output.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert!(!components.is_empty(), "literal-zero Component artifacts");

    phase = phase.next("seal_release_manifests");
    let infrastructure = compile_and_persist_canic_infrastructure_artifact_manifest(
        adapter_root,
        release_build_id,
        &[
            infrastructure_build_output(
                CanicInfrastructureRole::FleetCoordinator,
                release_build_id,
                &coordinator,
            ),
            infrastructure_build_output(
                CanicInfrastructureRole::FleetSubnetRoot,
                release_build_id,
                &root,
            ),
            infrastructure_build_output(
                CanicInfrastructureRole::WasmStore,
                release_build_id,
                &store,
            ),
        ],
    )
    .expect("persist literal-zero infrastructure authority");
    let application_targets = components
        .iter()
        .map(|(role, component)| ApplicationArtifactBuildTarget {
            role: role.clone(),
            package: component.package_name.clone(),
            wasm_relative_path: relative_artifact_path(adapter_root, &component.wasm_path),
            wasm_gz_relative_path: relative_artifact_path(adapter_root, &component.wasm_gz_path),
        })
        .collect::<Vec<_>>();
    let application_outputs = components
        .iter()
        .map(|(role, component)| ApplicationArtifactFileBuildOutput {
            role: role.clone(),
            package: component.package_name.clone(),
            release_build_id,
            wasm_path: component.wasm_path.clone(),
            wasm_gz_path: component.wasm_gz_path.clone(),
            candid_sha256: component.candid_sha256,
            protocol_profile_digest: component.protocol_profile_digest,
        })
        .collect::<Vec<_>>();
    let application = compile_and_persist_application_artifact_union(
        adapter_root,
        &configuration.component_topology,
        release_build_id,
        &application_targets,
        &application_outputs,
    )
    .expect("persist literal-zero application authority");
    let fixtures = prepare_generated_fixture_artifacts(
        adapter_root,
        &configuration.component_topology,
        release_build_id,
        configured_roles,
    );
    let current = compile_and_persist_current_release_set_manifest(
        adapter_root,
        &configuration.component_topology,
        release_build_id,
        &application,
        &infrastructure,
        &fixtures,
    )
    .expect("persist literal-zero current release authority");
    finalize_release_build_from_manifest(adapter_root, release_build_id, &current.path)
        .expect("finalize literal-zero current release authority");
    phase.finish();
}

pub(super) fn persist_internal_test_release_build_plan(
    root: &Path,
    build_network: BuildNetwork,
    release_nonce: [u8; 32],
) -> ReleaseBuildId {
    let nonce = ReleaseBuildNonce::from_random_bytes(release_nonce);
    let release_build_id = ReleaseBuildId::from_nonce(nonce);
    let value = Value::Array(vec![
        Value::Bytes(nonce.as_bytes().to_vec()),
        Value::Bytes(release_build_id.as_bytes().to_vec()),
        Value::Text(env!("CARGO_PKG_VERSION").to_string()),
        Value::Text("fast".to_string()),
        Value::Text(build_network.to_string()),
        Value::Array(vec![Value::Integer(0.into())]),
    ]);
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&value, &mut bytes)
        .expect("encode deterministic internal release-build plan");
    let path = canic_host::release_build::release_build_plan_path(root, release_build_id);
    std::fs::create_dir_all(path.parent().expect("release-build plan parent"))
        .expect("create release-build plan parent");
    std::fs::write(path, bytes).expect("write deterministic internal release-build plan");
    let planned = canic_host::release_build::load_release_build_plan(root, release_build_id)
        .expect("validate fixture release-build authority before building artifacts");
    assert_eq!(planned.build_network, build_network);
    release_build_id
}

// Stage from immutable retained inputs into this invocation's private release root.
pub(super) fn stage_retained_release_artifacts(
    record: &ic_testkit::artifacts::ArtifactCacheRecord,
    outputs: &BTreeMap<String, PathBuf>,
) {
    assert_eq!(record.artifacts().len(), outputs.len());
    for (name, destination) in outputs {
        let source = crate::pic::artifacts::retained_artifact_path(record, name);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(source, destination).expect("stage retained release artifact");
    }
}

fn infrastructure_build_output(
    role: CanicInfrastructureRole,
    release_build_id: ReleaseBuildId,
    output: &CanisterArtifactBuildOutput,
) -> CanicInfrastructureArtifactBuildOutput {
    CanicInfrastructureArtifactBuildOutput {
        role,
        package: output.package_name.clone(),
        protocol_release_identity: output.protocol_release_identity.clone(),
        protocol_role: output.protocol_role.clone(),
        protocol_capabilities: output.protocol_capabilities.clone(),
        release_build_id,
        wasm_path: output.wasm_path.clone(),
        wasm_gz_path: output.wasm_gz_path.clone(),
        candid_sha256: output.candid_sha256,
        protocol_profile_digest: output.protocol_profile_digest,
    }
}

pub(super) fn relative_artifact_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("literal-zero artifact stays inside its operator root")
        .to_str()
        .expect("literal-zero artifact path UTF-8")
        .to_string()
}

fn bind_build_helper_inputs(mut cache: ArtifactCacheSpec, workspace: &Path) -> ArtifactCacheSpec {
    for path in BUILD_HELPERS {
        cache = cache.with_input(path, &workspace.join(path));
    }
    cache
}
