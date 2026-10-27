use super::{RootBaselineSpec, progress, progress_elapsed};
use crate::pic::{
    artifacts::{
        INTERNAL_TEST_RELEASE_BUILD_ID, internal_test_artifact_maintenance_interval,
        internal_test_artifact_prune_policy, report_artifact_cache_maintenance,
        retained_artifact_path, run_icp_all_with_env, with_canonical_root_cargo_inputs,
    },
    progress as test_progress,
};
use canic::ids::CanisterRole;
use canic_host::{
    release_set::AppConfigSnapshot,
    role_contract::{
        CargoFeatureSelection, PackageValidationMode, RolePackageValidation,
        validate_declared_role_package,
    },
};
use ic_build::artifacts::{
    ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheRecord, ArtifactCacheSpec,
    WasmBuildSpec, prepare_artifact_cache, resolve_cargo_build_inputs,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::PathBuf,
};

/// Build or transactionally reuse the complete qualified root artifact set.
///
/// # Panics
///
/// Panics if exact inputs cannot be captured, the external build fails, inputs
/// change during the build, or any required output cannot be committed.

#[must_use]
pub fn ensure_root_release_artifacts_built(spec: &RootBaselineSpec<'_>) -> ArtifactCacheRecord {
    progress(spec, "acquiring local ICP artifacts for root baseline");
    let started_at = std::time::Instant::now();
    let build_env = effective_build_env(spec);
    let outputs = root_release_artifact_outputs(spec, &build_env);
    let cache_spec = root_release_artifact_cache_spec(spec, &build_env, &outputs);
    let outcome =
        match prepare_artifact_cache(&cache_spec).expect("prepare root release artifact cache") {
            ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
            ArtifactCachePreparation::Build(transaction) => {
                progress(spec, "building local ICP artifacts for root baseline");
                let output = run_icp_all_with_env(
                    &spec.workspace_root,
                    spec.build_network,
                    spec.build_profile,
                    &spec.build_config_path,
                    &build_env,
                );
                assert!(
                    output.status.success(),
                    "local artifact build failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                for (name, path) in &outputs {
                    transaction.import_output(name, path).unwrap_or_else(|err| {
                        panic!("import root release artifact `{name}`: {err}")
                    });
                }
                transaction
                    .commit()
                    .expect("commit root release artifact cache")
            }
        };
    progress_elapsed(
        spec,
        if outcome.is_reused() {
            "reused local ICP artifact set"
        } else {
            "built local ICP artifact set"
        },
        started_at,
    );
    test_progress::detail("ROOT", &format!("artifact cache: {outcome}"));
    report_artifact_cache_maintenance("root-artifacts", outcome.record().maintenance());
    outcome.record().clone()
}

/// Load the built `root.wasm.gz` artifact used for PocketIC root installs.
///
/// # Panics
///
/// Panics if the root wasm artifact exists but cannot be read, or if it exceeds
/// the configured PocketIC chunk-store size limit.
#[must_use]
pub fn load_root_wasm(
    spec: &RootBaselineSpec<'_>,
    artifacts: &ArtifactCacheRecord,
) -> Option<Vec<u8>> {
    let path = retained_artifact_path(artifacts, "root");
    match fs::read(path) {
        Ok(bytes) => {
            assert!(
                bytes.len() < spec.pocket_ic_wasm_chunk_store_limit_bytes,
                "root wasm artifact is too large for PocketIC chunked install: {} bytes at {}. \
Use a compressed `.wasm.gz` artifact and/or build canister wasm with `RUSTFLAGS=\"-C debuginfo=0\"`.",
                bytes.len(),
                path.display()
            );
            Some(bytes)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => panic!("failed to read root wasm at {}: {err}", path.display()),
    }
}

fn root_release_artifact_cache_spec(
    spec: &RootBaselineSpec<'_>,
    build_env: &[(&str, &str)],
    outputs: &BTreeMap<String, PathBuf>,
) -> ArtifactCacheSpec {
    let config_path = spec
        .build_config_path
        .strip_prefix(&spec.workspace_root)
        .expect("root build config must be workspace-confined")
        .to_str()
        .expect("root build config path UTF-8");
    let mut environment = vec![("ICP_ENVIRONMENT", spec.build_network.as_str())];
    environment.extend_from_slice(build_env);
    let cargo_build = root_release_cargo_build_spec(spec, &environment);
    let mut cache = ArtifactCacheSpec::new(
        &spec
            .workspace_root
            .join("target/test-artifacts/external-artifact-cache"),
        "root-release-artifacts",
        "canic/root-release-artifacts/v1",
    )
    .with_coordination_scope("canic-external-artifact-builds")
    .with_arguments([
        "scripts/ci/build-ci-wasm-artifacts.sh",
        spec.build_profile.canic_wasm_profile_value(),
        config_path,
    ])
    .with_environment(environment.iter().copied())
    .with_input("build-config", &spec.build_config_path)
    .with_prune_policy_at_most_every(
        internal_test_artifact_prune_policy(),
        internal_test_artifact_maintenance_interval(),
    );
    cache = with_canonical_root_cargo_inputs(
        cache,
        &spec.build_config_path,
        &spec.workspace_root.join("target/icp-build"),
        spec.build_profile,
        &environment,
    );
    // Materialize the generated Root package before capturing the application
    // package inputs: fixture configurations may live inside that package.
    let cargo_inputs =
        resolve_cargo_build_inputs(&cargo_build).expect("resolve root release Cargo build inputs");
    cache = cache.with_cargo_build_inputs("root-release-cargo", &cargo_build, &cargo_inputs);

    for relative in spec.artifact_watch_paths {
        cache = cache.with_input(relative, &spec.workspace_root.join(relative));
    }
    for (name, path) in outputs {
        cache = cache.with_output(name, path);
    }
    cache
}

fn root_release_cargo_build_spec(
    spec: &RootBaselineSpec<'_>,
    environment: &[(&str, &str)],
) -> WasmBuildSpec {
    let config = AppConfigSnapshot::load(&spec.build_config_path)
        .expect("load root release build config for Cargo inputs");
    let mut packages = BTreeSet::from(["canic-host".to_string(), "canic".to_string()]);
    for role in spec.release_roles.iter().copied() {
        if role == "root" {
            continue;
        }
        let validation = validate_declared_role_package(
            &spec.build_config_path,
            config.model(),
            &CanisterRole::owned(role.to_string()),
            PackageValidationMode::Build,
            &CargoFeatureSelection::default(),
        );
        let RolePackageValidation::Supported(evidence) = validation else {
            panic!("root release role `{role}` must resolve: {validation:?}");
        };
        assert_eq!(
            evidence.cargo_workspace_root, spec.workspace_root,
            "Root test roles must share the fixture Cargo workspace"
        );
        packages.insert(evidence.role_package_name);
    }
    let packages = packages.iter().map(String::as_str).collect::<Vec<_>>();

    WasmBuildSpec::new(
        &spec.workspace_root,
        &spec.workspace_root.join("target/icp-build"),
        &packages,
        crate::workspace::wasm_build_environment(),
        canic_host::canister_build::CARGO_CONFIGURATION_LIMITS,
    )
    .with_cargo_selection(spec.build_profile.cargo_selection())
    .with_environment(crate::workspace::wasm_build_environment().chain(
        (environment.iter().copied()).map(|(key, value)| {
            (
                std::ffi::OsString::from(key),
                std::ffi::OsString::from(value),
            )
        }),
    ))
}

fn root_release_artifact_outputs(
    spec: &RootBaselineSpec<'_>,
    build_env: &[(&str, &str)],
) -> BTreeMap<String, PathBuf> {
    let release_id = build_env
        .iter()
        .find(|(key, _)| *key == INTERNAL_TEST_RELEASE_BUILD_ID.0)
        .expect("effective root environment includes a release identity")
        .1
        .parse::<canic_contracts::ids::ReleaseBuildId>()
        .expect("root build release identity must be valid");
    let artifact_root = spec
        .workspace_root
        .join(".canic/release-builds")
        .join(release_id.to_string())
        .join("artifacts");
    let mut outputs = BTreeMap::new();
    for role_name in ["root", "wasm_store"] {
        outputs.insert(
            role_name.to_string(),
            artifact_root
                .join(role_name)
                .join(format!("{role_name}.wasm.gz")),
        );
    }
    for role in configured_release_roles(spec) {
        let role_name = role.as_str().to_string();
        outputs.insert(
            role_name.clone(),
            artifact_root
                .join(&role_name)
                .join(format!("{role_name}.wasm.gz")),
        );
    }
    outputs
}

// Ensure internal PocketIC root baselines retain their explicit qualified-build identity.
fn effective_build_env<'a>(spec: &'a RootBaselineSpec<'a>) -> Vec<(&'a str, &'a str)> {
    let mut env = spec
        .build_extra_env
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();

    if env
        .iter()
        .all(|(key, _)| *key != INTERNAL_TEST_RELEASE_BUILD_ID.0)
    {
        env.push(INTERNAL_TEST_RELEASE_BUILD_ID);
    }

    env
}

// Map the configured ordinary role names into stable `CanisterRole` values.
fn configured_release_roles(spec: &RootBaselineSpec<'_>) -> Vec<CanisterRole> {
    spec.release_roles
        .iter()
        .copied()
        .map(|role| CanisterRole::owned(role.to_string()))
        .collect()
}
