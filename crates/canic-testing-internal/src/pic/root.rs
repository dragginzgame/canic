use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryResponse,
    },
    ids::CanisterRole,
    protocol,
};
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
        INTERNAL_TEST_ENDPOINTS_ENV, WasmBuildProfile, build_dfx_all_with_env,
        dfx_artifact_ready_for_build,
    },
    pic::{CachedPicBaseline, Pic, PicBuilder},
};
use std::{collections::HashMap, fs, io, io::Write, path::PathBuf, time::Instant};

///
/// RootBaselineSpec
///

#[derive(Clone)]
pub struct RootBaselineSpec<'a> {
    pub progress_prefix: &'a str,
    pub workspace_root: PathBuf,
    pub root_wasm_relative: &'a str,
    pub root_wasm_artifact_relative: &'a str,
    pub root_release_artifacts_relative: &'a str,
    pub artifact_watch_paths: &'a [&'a str],
    pub release_roles: &'a [&'a str],
    pub dfx_build_lock_relative: &'a str,
    pub build_network: &'a str,
    pub build_profile: WasmBuildProfile,
    pub build_extra_env: Vec<(String, String)>,
    pub bootstrap_tick_limit: usize,
    pub root_setup_max_attempts: usize,
    pub pocket_ic_wasm_chunk_store_limit_bytes: usize,
    pub root_release_chunk_bytes: usize,
    pub package_version: &'a str,
}

///
/// RootBaselineMetadata
///

pub struct RootBaselineMetadata {
    pub root_id: Principal,
    pub subnet_directory: HashMap<CanisterRole, Principal>,
}

// Print one progress line for a root-test setup phase and flush immediately.
fn progress(spec: &RootBaselineSpec<'_>, phase: &str) {
    eprintln!("[{}] {phase}", spec.progress_prefix);
    let _ = std::io::stderr().flush();
}

// Print one completed phase with wall-clock timing.
fn progress_elapsed(spec: &RootBaselineSpec<'_>, phase: &str, started_at: Instant) {
    progress(
        spec,
        &format!("{phase} in {:.2}s", started_at.elapsed().as_secs_f32()),
    );
}

/// Build the local `.dfx` root artifacts once unless all required outputs are already fresh.
pub fn ensure_root_release_artifacts_built(spec: &RootBaselineSpec<'_>) {
    if root_release_artifacts_ready(spec) {
        progress(spec, "reusing existing root release artifacts");
        return;
    }

    progress(spec, "building local DFX artifacts for root baseline");
    let started_at = Instant::now();
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

/// Build one fresh root topology and capture immutable controller snapshots for cache reuse.
#[must_use]
pub fn build_root_cached_baseline(
    spec: &RootBaselineSpec<'_>,
    root_wasm: Vec<u8>,
) -> CachedPicBaseline<RootBaselineMetadata> {
    let initialized = setup_root_topology(spec, root_wasm);
    let controller_ids = std::iter::once(initialized.metadata.root_id)
        .chain(initialized.metadata.subnet_directory.values().copied())
        .collect::<Vec<_>>();

    progress(spec, "capturing cached root snapshots");
    let started_at = Instant::now();
    let baseline = CachedPicBaseline::capture(
        initialized.pic,
        initialized.metadata.root_id,
        controller_ids,
        initialized.metadata,
    )
    .expect("cached root snapshots must be available");
    progress_elapsed(spec, "captured cached root snapshots", started_at);
    baseline
}

/// Restore one cached root topology and wait until root plus children are ready again.
pub fn restore_root_cached_baseline(
    spec: &RootBaselineSpec<'_>,
    baseline: &CachedPicBaseline<RootBaselineMetadata>,
) {
    progress(spec, "restoring cached root snapshots");
    let restore_started_at = Instant::now();
    baseline.restore(baseline.metadata().root_id);
    progress_elapsed(spec, "restored cached root snapshots", restore_started_at);

    progress(spec, "waiting for restored root bootstrap");
    let root_wait_started_at = Instant::now();
    wait_for_bootstrap(spec, baseline.pic(), baseline.metadata().root_id);
    progress_elapsed(spec, "restored root bootstrap ready", root_wait_started_at);

    progress(spec, "waiting for restored child canisters ready");
    let child_wait_started_at = Instant::now();
    wait_for_children_ready(spec, baseline.pic(), &baseline.metadata().subnet_directory);
    progress_elapsed(
        spec,
        "restored child canisters ready",
        child_wait_started_at,
    );
}

/// Install root, stage one ordinary release profile, resume bootstrap, and fetch the subnet map.
#[must_use]
pub fn setup_root_topology(
    spec: &RootBaselineSpec<'_>,
    root_wasm: Vec<u8>,
) -> InitializedRootTopology {
    for attempt in 1..=spec.root_setup_max_attempts {
        progress(
            spec,
            &format!(
                "initialize root setup attempt {attempt}/{}",
                spec.root_setup_max_attempts
            ),
        );
        let wasm = root_wasm.clone();
        let attempt_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            progress(spec, "starting PocketIC instance");
            let pic_started_at = Instant::now();
            let pic = PicBuilder::new()
                .with_ii_subnet()
                .with_application_subnet()
                .build();
            progress_elapsed(spec, "PocketIC instance ready", pic_started_at);

            progress(spec, "installing root canister");
            let root_install_started_at = Instant::now();
            let root_id = pic
                .create_and_install_root_canister(wasm)
                .expect("install root canister");
            progress_elapsed(spec, "root canister installed", root_install_started_at);

            progress(spec, "staging managed release set");
            let stage_started_at = Instant::now();
            stage_managed_release_set(spec, &pic, root_id);
            progress_elapsed(spec, "staged managed release set", stage_started_at);

            progress(spec, "resuming root bootstrap");
            let resume_started_at = Instant::now();
            resume_root_bootstrap(&pic, root_id);
            progress_elapsed(spec, "resumed root bootstrap", resume_started_at);

            progress(spec, "waiting for root bootstrap");
            let root_wait_started_at = Instant::now();
            wait_for_bootstrap(spec, &pic, root_id);
            progress_elapsed(spec, "root bootstrap ready", root_wait_started_at);

            progress(spec, "fetching subnet directory");
            let directory_started_at = Instant::now();
            let subnet_directory = fetch_subnet_directory(&pic, root_id);
            progress_elapsed(spec, "fetched subnet directory", directory_started_at);

            progress(spec, "waiting for child canisters ready");
            let child_wait_started_at = Instant::now();
            wait_for_children_ready(spec, &pic, &subnet_directory);
            progress_elapsed(spec, "child canisters ready", child_wait_started_at);

            InitializedRootTopology {
                pic,
                metadata: RootBaselineMetadata {
                    root_id,
                    subnet_directory,
                },
            }
        }));

        match attempt_result {
            Ok(state) => return state,
            Err(err) if attempt < spec.root_setup_max_attempts => {
                eprintln!(
                    "setup_root attempt {attempt}/{} failed; retrying",
                    spec.root_setup_max_attempts
                );
                drop(err);
            }
            Err(err) => std::panic::resume_unwind(err),
        }
    }

    unreachable!("setup_root must return or panic")
}

///
/// InitializedRootTopology
///

pub struct InitializedRootTopology {
    pub pic: Pic,
    pub metadata: RootBaselineMetadata,
}

// Stage the configured ordinary release set into root before bootstrap resumes.
fn stage_managed_release_set(spec: &RootBaselineSpec<'_>, pic: &Pic, root_id: Principal) {
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

    if !dfx_artifact_ready_for_build(
        &spec.workspace_root,
        spec.root_wasm_artifact_relative,
        spec.artifact_watch_paths,
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
        dfx_artifact_ready_for_build(
            &spec.workspace_root,
            &artifact_relative_path,
            spec.artifact_watch_paths,
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
fn resume_root_bootstrap(pic: &Pic, root_id: Principal) {
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

// Wait until root reports `canic_ready`.
fn wait_for_bootstrap(spec: &RootBaselineSpec<'_>, pic: &Pic, root_id: Principal) {
    pic.wait_for_ready(root_id, spec.bootstrap_tick_limit, "root bootstrap");
}

// Wait until every child canister reports `canic_ready`.
fn wait_for_children_ready(
    spec: &RootBaselineSpec<'_>,
    pic: &Pic,
    subnet_directory: &HashMap<CanisterRole, Principal>,
) {
    pic.wait_for_all_ready(
        subnet_directory
            .iter()
            .filter(|(role, _)| !role.is_root())
            .map(|(_, pid)| *pid),
        spec.bootstrap_tick_limit,
        "root children bootstrap",
    );
}

// Fetch the subnet directory from root as a role → principal map.
fn fetch_subnet_directory(pic: &Pic, root_id: Principal) -> HashMap<CanisterRole, Principal> {
    let page: Result<Page<DirectoryEntryResponse>, canic::Error> = pic
        .query_call(
            root_id,
            protocol::CANIC_SUBNET_DIRECTORY,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("query subnet directory transport");

    let page = page.expect("query subnet directory application");

    page.entries
        .into_iter()
        .map(|entry| (entry.role, entry.pid))
        .collect()
}
