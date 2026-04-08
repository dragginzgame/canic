use canic::{Error, cdk::types::Principal, ids::CanisterRole, protocol};
use canic_control_plane::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput,
    },
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    },
};
use canic_testkit::{
    artifacts::{
        INTERNAL_TEST_ENDPOINTS_ENV, WatchedInputSnapshot, build_dfx_all_with_env,
        dfx_artifact_ready_with_snapshot,
    },
    pic::Pic,
};
use std::{fs, io};

use super::{RootBaselineSpec, progress, progress_elapsed};

/// Build the local `.dfx` root artifacts once unless all required outputs are already fresh.
pub fn ensure_root_release_artifacts_built(spec: &RootBaselineSpec<'_>) {
    if root_release_artifacts_ready(spec) {
        progress(spec, "reusing existing root release artifacts");
        return;
    }

    progress(spec, "building local DFX artifacts for root baseline");
    let started_at = std::time::Instant::now();
    build_dfx_all_with_env(
        &spec.workspace_root,
        spec.dfx_build_lock_relative,
        spec.build_network,
        spec.build_profile,
        &effective_build_env(spec),
    );
    progress_elapsed(spec, "finished local DFX artifact build", started_at);
}

/// Load the built `root.wasm.gz` artifact used for PocketIC root installs.
#[must_use]
pub fn load_root_wasm(spec: &RootBaselineSpec<'_>) -> Option<Vec<u8>> {
    let path = spec.workspace_root.join(spec.root_wasm_relative);
    match fs::read(&path) {
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
        Err(err) => panic!("failed to read root wasm at {}: {}", path.display(), err),
    }
}

// Stage the configured ordinary release set into root before bootstrap resumes.
pub(super) fn stage_managed_release_set(
    spec: &RootBaselineSpec<'_>,
    pic: &Pic,
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
        let payload_hash = canic::cdk::utils::wasm::get_wasm_hash(&wasm_module);
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
            chunk_hashes: chunks
                .iter()
                .map(|chunk| canic::cdk::utils::wasm::get_wasm_hash(chunk))
                .collect(),
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
        .workspace_root
        .join(spec.root_release_artifacts_relative)
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

// Confirm the root bootstrap artifact and every managed ordinary release artifact are fresh.
fn root_release_artifacts_ready(spec: &RootBaselineSpec<'_>) -> bool {
    let build_env = effective_build_env(spec);
    let Ok(watched_inputs) =
        WatchedInputSnapshot::capture(&spec.workspace_root, spec.artifact_watch_paths)
    else {
        return false;
    };

    if !dfx_artifact_ready_with_snapshot(
        &spec.workspace_root,
        spec.root_wasm_artifact_relative,
        watched_inputs,
        spec.build_network,
        spec.build_profile,
        &build_env,
    ) {
        return false;
    }

    configured_release_roles(spec).into_iter().all(|role| {
        let role_name = role.as_str().to_string();
        let artifact_relative_path = format!(
            "{}/{role_name}/{role_name}.wasm.gz",
            spec.root_release_artifacts_relative
        );
        dfx_artifact_ready_with_snapshot(
            &spec.workspace_root,
            &artifact_relative_path,
            watched_inputs,
            spec.build_network,
            spec.build_profile,
            &build_env,
        )
    })
}

// Ensure internal PocketIC root baselines keep the extra introspection surface
// even though production canister builds now omit those test-only queries.
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
fn stage_manifest(pic: &Pic, root_id: Principal, manifest: TemplateManifestInput) {
    let staged: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
            (manifest,),
        )
        .expect("stage release manifest transport");

    staged.expect("stage release manifest application");
}

// Prepare one staged chunk set through the root admin surface.
fn prepare_chunk_set(pic: &Pic, root_id: Principal, prepare: TemplateChunkSetPrepareInput) {
    let prepared: Result<TemplateChunkSetInfoResponse, Error> = pic
        .update_call(root_id, protocol::CANIC_TEMPLATE_PREPARE_ADMIN, (prepare,))
        .expect("prepare release chunk set transport");

    let _ = prepared.expect("prepare release chunk set application");
}

// Publish one staged release chunk through the root admin surface.
fn publish_chunk(pic: &Pic, root_id: Principal, chunk: TemplateChunkInput) {
    let published: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            (chunk,),
        )
        .expect("publish release chunk transport");

    published.expect("publish release chunk application");
}

// Resume the root bootstrap flow once the ordinary release set is staged.
pub(super) fn resume_root_bootstrap(pic: &Pic, root_id: Principal) {
    let resumed: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_WASM_STORE_BOOTSTRAP_RESUME_ROOT_ADMIN,
            (),
        )
        .expect("resume root bootstrap transport");

    resumed.expect("resume root bootstrap application");
}

// Read the current PocketIC wall clock in whole seconds.
fn root_time_secs(pic: &Pic, _root_id: Principal) -> u64 {
    pic.current_time_nanos() / 1_000_000_000
}
