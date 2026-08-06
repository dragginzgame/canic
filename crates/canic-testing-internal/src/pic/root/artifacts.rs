use candid::Principal;
use canic::{Error, ids::CanisterRole, protocol};
use canic_control_plane::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput,
    },
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    },
};
use canic_core::cdk::utils::hash::wasm_hash;
use ic_testkit::{
    artifacts::{
        ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheSpec, prepare_artifact_cache,
    },
    pic::{CandidCallExt, PocketIc, PocketIcTimeExt},
};
use std::{collections::BTreeMap, fs, io, path::PathBuf};

use crate::pic::artifacts::{
    INTERNAL_TEST_ENDPOINTS_ENV, INTERNAL_TEST_RELEASE_BUILD_ID,
    internal_test_artifact_prune_policy, report_artifact_cache_maintenance, run_icp_all_with_env,
};

use super::{RootBaselineSpec, progress, progress_elapsed};

/// Build or transactionally reuse the complete local `.icp` root artifact set.
///
/// # Panics
///
/// Panics if exact inputs cannot be captured, the external build fails, inputs
/// change during the build, or any required output cannot be committed.
pub fn ensure_root_release_artifacts_built(spec: &RootBaselineSpec<'_>) {
    progress(spec, "acquiring local ICP artifacts for root baseline");
    let started_at = std::time::Instant::now();
    let build_env = effective_build_env(spec);
    let outputs = root_release_artifact_outputs(spec);
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
    let timings = outcome.record().timings();
    progress_elapsed(
        spec,
        if outcome.is_reused() {
            "reused local ICP artifact set"
        } else {
            "built local ICP artifact set"
        },
        started_at,
    );
    eprintln!(
        "[root_setup] artifact cache {:?} (coordination {:?}, inputs {:?}, build {:?}, materialize {:?})",
        timings.total(),
        timings.coordination_lock_wait(),
        timings.input_capture(),
        timings.caller_build(),
        timings.materialization(),
    );
    report_artifact_cache_maintenance("root-artifacts", outcome.record().maintenance());
}

/// Load the built `root.wasm.gz` artifact used for PocketIC root installs.
///
/// # Panics
///
/// Panics if the root wasm artifact exists but cannot be read, or if it exceeds
/// the configured PocketIC chunk-store size limit.
#[must_use]
pub fn load_root_wasm(spec: &RootBaselineSpec<'_>) -> Option<Vec<u8>> {
    match fs::read(&spec.root_wasm_path) {
        Ok(bytes) => {
            assert!(
                bytes.len() < spec.pocket_ic_wasm_chunk_store_limit_bytes,
                "root wasm artifact is too large for PocketIC chunked install: {} bytes at {}. \
Use a compressed `.wasm.gz` artifact and/or build canister wasm with `RUSTFLAGS=\"-C debuginfo=0\"`.",
                bytes.len(),
                spec.root_wasm_path.display()
            );
            Some(bytes)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => panic!(
            "failed to read root wasm at {}: {err}",
            spec.root_wasm_path.display()
        ),
    }
}

// Stage the configured ordinary release set into root before bootstrap resumes.
pub(super) fn stage_managed_release_set(
    spec: &RootBaselineSpec<'_>,
    pic: &PocketIc,
    root_id: Principal,
) {
    let now_secs = root_time_secs(pic, root_id);
    let version = TemplateVersion::owned(spec.package_version.to_string());
    let roles = configured_release_roles(spec);
    let total = roles.len();

    for (index, role) in roles.into_iter().enumerate() {
        let role_name = role.as_str().to_string();
        progress(
            spec,
            &format!("staging release {}/{}: {role_name}", index + 1, total),
        );
        let wasm_module = load_release_wasm_gz(spec, &role_name);
        let template_id = TemplateId::owned(format!("embedded:{role}"));
        let payload_hash = wasm_hash(&wasm_module);
        let payload_size_bytes = wasm_module.len() as u64;
        let chunks = wasm_module
            .chunks(spec.root_release_chunk_bytes)
            .map(<[u8]>::to_vec)
            .collect::<Vec<_>>();

        let manifest = TemplateManifestInput {
            template_id: template_id.clone(),
            role: role.clone(),
            version: version.clone(),
            payload_hash: payload_hash.clone(),
            payload_size_bytes,
            store_binding: WasmStoreBinding::new("bootstrap"),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(now_secs),
            created_at: now_secs,
        };
        stage_manifest(pic, root_id, manifest);

        let prepare = TemplateChunkSetPrepareInput {
            template_id: template_id.clone(),
            version: version.clone(),
            payload_hash: payload_hash.clone(),
            payload_size_bytes,
            chunk_hashes: chunks.iter().map(|chunk| wasm_hash(chunk)).collect(),
        };
        prepare_chunk_set(pic, root_id, prepare);

        for (chunk_index, bytes) in chunks.into_iter().enumerate() {
            publish_chunk(
                pic,
                root_id,
                TemplateChunkInput {
                    template_id: template_id.clone(),
                    version: version.clone(),
                    chunk_index: u32::try_from(chunk_index)
                        .expect("release chunk index must fit into nat32"),
                    bytes,
                },
            );
        }
    }
}

// Load one built `.wasm.gz` artifact for a configured release role.
fn load_release_wasm_gz(spec: &RootBaselineSpec<'_>, role_name: &str) -> Vec<u8> {
    let artifact_path = spec
        .root_release_artifacts_dir
        .clone()
        .join(role_name)
        .join(format!("{role_name}.wasm.gz"));
    let bytes = fs::read(&artifact_path)
        .unwrap_or_else(|err| panic!("read {} failed: {err}", artifact_path.display()));
    assert!(
        !bytes.is_empty(),
        "release artifact must not be empty: {}",
        artifact_path.display()
    );
    bytes
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
    let mut cache = ArtifactCacheSpec::new(
        &spec
            .workspace_root
            .join("target/test-artifacts/external-artifact-cache"),
        "root-release-artifacts",
        "canic/root-release-artifacts/v1",
    )
    .with_coordination_scope("canic-external-artifact-builds")
    .with_arguments(&[
        "scripts/ci/build-ci-wasm-artifacts.sh",
        spec.build_profile.canic_wasm_profile_value(),
        config_path,
    ])
    .with_environment(&environment)
    .with_prune_policy(internal_test_artifact_prune_policy());

    for relative in spec.artifact_watch_paths {
        cache = cache.with_input(relative, &spec.workspace_root.join(relative));
    }
    for (name, path) in outputs {
        cache = cache.with_output(name, path);
    }
    cache
}

fn root_release_artifact_outputs(spec: &RootBaselineSpec<'_>) -> BTreeMap<String, PathBuf> {
    let mut outputs = BTreeMap::new();
    for role_name in ["root", "wasm_store"] {
        outputs.insert(
            role_name.to_string(),
            spec.root_release_artifacts_dir
                .join(role_name)
                .join(format!("{role_name}.wasm.gz")),
        );
    }
    for role in configured_release_roles(spec) {
        let role_name = role.as_str().to_string();
        outputs.insert(
            role_name.clone(),
            spec.root_release_artifacts_dir
                .join(&role_name)
                .join(format!("{role_name}.wasm.gz")),
        );
    }
    outputs
}

// Ensure internal PocketIC root baselines retain their explicit qualified-build
// identity and extra introspection surface.
fn effective_build_env<'a>(spec: &'a RootBaselineSpec<'a>) -> Vec<(&'a str, &'a str)> {
    let mut env = spec
        .build_extra_env
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();

    if env
        .iter()
        .all(|(key, _)| *key != INTERNAL_TEST_ENDPOINTS_ENV.0)
    {
        env.push(INTERNAL_TEST_ENDPOINTS_ENV);
    }
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

// Stage one manifest through the root admin surface.
fn stage_manifest(pic: &PocketIc, root_id: Principal, manifest: TemplateManifestInput) {
    let staged: Result<(), Error> = pic
        .update_candid(
            root_id,
            protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
            (manifest,),
        )
        .expect("stage release manifest transport");

    staged.expect("stage release manifest application");
}

// Prepare one staged chunk set through the root admin surface.
fn prepare_chunk_set(pic: &PocketIc, root_id: Principal, prepare: TemplateChunkSetPrepareInput) {
    let prepared: Result<TemplateChunkSetInfoResponse, Error> = pic
        .update_candid(root_id, protocol::CANIC_TEMPLATE_PREPARE_ADMIN, (prepare,))
        .expect("prepare release chunk set transport");

    let _ = prepared.expect("prepare release chunk set application");
}

// Publish one staged release chunk through the root admin surface.
fn publish_chunk(pic: &PocketIc, root_id: Principal, chunk: TemplateChunkInput) {
    let published: Result<(), Error> = pic
        .update_candid(
            root_id,
            protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            (chunk,),
        )
        .expect("publish release chunk transport");

    published.expect("publish release chunk application");
}

// Read the current PocketIC wall clock in whole seconds.
fn root_time_secs(pic: &PocketIc, _root_id: Principal) -> u64 {
    pic.current_time_nanos() / 1_000_000_000
}
