// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

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
    artifacts::{WasmBuildProfile, build_dfx_all, dfx_artifact_ready, workspace_root_for},
    pic::{ControllerSnapshots, Pic, pic},
};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    env, fs, io,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, Once},
};

/// Environment variable override for providing a pre-built root canister wasm.
const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";

/// Default location of the root wasm relative to this crate’s manifest dir.
const ROOT_WASM_RELATIVE: &str = "../../.dfx/local/canisters/root/root.wasm.gz";
const ROOT_WASM_ARTIFACT_RELATIVE: &str = ".dfx/local/canisters/root/root.wasm.gz";
const ROOT_RELEASE_ARTIFACTS_RELATIVE: &str = ".dfx/local/canisters";
const ROOT_CONFIG_RELATIVE: &str = "canisters/canic.toml";
const POCKET_IC_WASM_CHUNK_STORE_LIMIT_BYTES: usize = 100 * 1024 * 1024;
const ROOT_RELEASE_CHUNK_BYTES: usize = 1024 * 1024;
const DFX_BUILD_LOCK_RELATIVE: &str = ".dfx/canic-tests-build.lock";
// WARNING: `Pic` MUST NOT be cached/shared across tests by default.
// This toggle is intentionally opt-in for local experimentation only.
// Enabling it can reintroduce hangs or flaky behavior from retained runtime state.
const ROOT_SETUP_CACHE_ENV: &str = "CANIC_TEST_ROOT_SETUP_CACHE";
const BOOTSTRAP_TICK_LIMIT: usize = 120;
const ROOT_SETUP_MAX_ATTEMPTS: usize = 2;
const ROOT_WASM_WATCH_PATHS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "dfx.json",
    "crates",
    "scripts/app/build.sh",
];
static DFX_BUILD_ONCE: Once = Once::new();
static ROOT_SETUP_SERIAL: Mutex<()> = Mutex::new(());
thread_local! {
    static ROOT_SETUP_CACHE: RefCell<Option<ManuallyDrop<RootSetupState>>> = const { RefCell::new(None) };
}

///
/// RootSetupState
///

pub struct RootSetupState {
    pub pic: Pic,
    pub root_id: Principal,
    pub subnet_directory: HashMap<CanisterRole, Principal>,
    baseline_snapshots: Option<ControllerSnapshots>,
}

///
/// RootSetup
/// Result of setting up a fresh root canister for tests.
///

pub struct RootSetup {
    state: Option<RootSetupState>,
    _serial_guard: MutexGuard<'static, ()>,
}

impl Deref for RootSetup {
    type Target = RootSetupState;

    fn deref(&self) -> &Self::Target {
        self.state.as_ref().expect("root setup state must exist")
    }
}

impl DerefMut for RootSetup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state.as_mut().expect("root setup state must exist")
    }
}

impl Drop for RootSetup {
    fn drop(&mut self) {
        if let Some(state) = self.state.take()
            && root_setup_cache_enabled()
            && state.baseline_snapshots.is_some()
        {
            ROOT_SETUP_CACHE.with(|cache| {
                *cache.borrow_mut() = Some(ManuallyDrop::new(state));
            });
        }
    }
}

/// Acquire an isolated root setup for a test.
///
/// The first call creates a PocketIC instance and captures canister snapshots.
/// Later calls restore those snapshots instead of reinstalling all canisters.
pub fn setup_root() -> RootSetup {
    // Each setup spins up a full PocketIC topology; serialize to avoid
    // exhausting local temp storage under parallel test execution.
    let serial_guard = acquire_root_setup_serial_guard();

    if root_setup_cache_enabled()
        && let Some(mut cached) = ROOT_SETUP_CACHE.with(|cache| cache.borrow_mut().take())
    {
        // SAFETY: The cached value is taken out of thread-local storage exactly once
        // before being reused and rewrapped in `RootSetup`, so moving it out is sound.
        let state = unsafe { ManuallyDrop::take(&mut cached) };
        restore_cached_setup(&state);

        return RootSetup {
            state: Some(state),
            _serial_guard: serial_guard,
        };
    }

    ensure_local_artifacts_built();
    let root_wasm = load_root_wasm().expect("load root wasm");
    let state = initialize_setup(root_wasm);

    RootSetup {
        state: Some(state),
        _serial_guard: serial_guard,
    }
}

// Serialize full root PocketIC usage to avoid concurrent runtime contention.
fn acquire_root_setup_serial_guard() -> MutexGuard<'static, ()> {
    ROOT_SETUP_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// WARNING: DO NOT ENABLE THIS IN CI OR SHARED TEST RUNNERS.
// Root setup caching is opt-in because `Pic` is not safe to cache/share across
// arbitrary test scheduling patterns.
fn root_setup_cache_enabled() -> bool {
    match env::var(ROOT_SETUP_CACHE_ENV) {
        Ok(value) => matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => false,
    }
}

fn initialize_setup(root_wasm: Vec<u8>) -> RootSetupState {
    for attempt in 1..=ROOT_SETUP_MAX_ATTEMPTS {
        let wasm = root_wasm.clone();
        let attempt_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let pic = pic();
            let root_id = pic
                .create_and_install_root_canister(wasm)
                .expect("install root canister");

            stage_managed_release_set(&pic, root_id);
            resume_root_bootstrap(&pic, root_id);
            wait_for_bootstrap(&pic, root_id);

            let subnet_directory = fetch_subnet_directory(&pic, root_id);
            wait_for_children_ready(&pic, &subnet_directory);
            let baseline_snapshots = if root_setup_cache_enabled() {
                pic.capture_controller_snapshots(
                    root_id,
                    std::iter::once(root_id).chain(subnet_directory.values().copied()),
                )
            } else {
                None
            };

            RootSetupState {
                pic,
                root_id,
                subnet_directory,
                baseline_snapshots,
            }
        }));

        match attempt_result {
            Ok(state) => return state,
            Err(err) if attempt < ROOT_SETUP_MAX_ATTEMPTS => {
                eprintln!(
                    "setup_root attempt {attempt}/{ROOT_SETUP_MAX_ATTEMPTS} failed; retrying"
                );
                drop(err);
            }
            Err(err) => std::panic::resume_unwind(err),
        }
    }

    unreachable!("setup_root must return or panic")
}

fn restore_cached_setup(state: &RootSetupState) {
    let Some(baselines) = &state.baseline_snapshots else {
        return;
    };

    state
        .pic
        .restore_controller_snapshots(state.root_id, baselines);
    wait_for_bootstrap(&state.pic, state.root_id);
    wait_for_children_ready(&state.pic, &state.subnet_directory);
}

fn ensure_local_artifacts_built() {
    DFX_BUILD_ONCE.call_once(|| {
        let workspace_root = workspace_root();

        // `make test` already builds canisters before `cargo test`; avoid redundant
        // `dfx build --all` work unless artifacts are missing.
        if root_release_artifacts_ready(&workspace_root) {
            return;
        }

        build_dfx_all(
            &workspace_root,
            DFX_BUILD_LOCK_RELATIVE,
            "local",
            WasmBuildProfile::Debug,
        );
    });
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}

// Stage the configured ordinary release set into root before bootstrap resumes.
fn stage_managed_release_set(pic: &Pic, root_id: Principal) {
    let now_secs = root_time_secs(pic, root_id);
    let version = TemplateVersion::owned(env!("CARGO_PKG_VERSION").to_string());

    for role in configured_release_roles() {
        let role_name = role.as_str().to_string();
        let wasm_module = load_release_wasm_gz(&role_name);
        let template_id = TemplateId::owned(format!("embedded:{role}"));
        let payload_hash = canic::cdk::utils::wasm::get_wasm_hash(&wasm_module);
        let payload_size_bytes = wasm_module.len() as u64;
        let chunks = wasm_module
            .chunks(ROOT_RELEASE_CHUNK_BYTES)
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

// Read the current replica time from root so staged manifests use replica timestamps.
fn root_time_secs(pic: &Pic, root_id: Principal) -> u64 {
    let now_secs: Result<u64, Error> = pic
        .query_call(root_id, protocol::CANIC_TIME, ())
        .expect("query root time transport");

    now_secs.expect("query root time application")
}

// Return the configured ordinary release roles that must be staged into root.
fn configured_release_roles() -> Vec<CanisterRole> {
    let config_path = workspace_root().join(ROOT_CONFIG_RELATIVE);
    let config_source = fs::read_to_string(&config_path)
        .unwrap_or_else(|err| panic!("read {} failed: {err}", config_path.display()));
    let config = canic::__internal::core::bootstrap::parse_config_model(&config_source)
        .unwrap_or_else(|err| panic!("invalid {}: {err}", config_path.display()));
    let mut roles = BTreeSet::new();

    for subnet in config.subnets.values() {
        for role in subnet.canisters.keys() {
            if role.is_root() || role.is_wasm_store() {
                continue;
            }

            roles.insert(role.as_str().to_string());
        }
    }

    roles.into_iter().map(CanisterRole::owned).collect()
}

// Load one built `.wasm.gz` artifact for a configured release role.
fn load_release_wasm_gz(role_name: &str) -> Vec<u8> {
    let artifact_path = workspace_root()
        .join(ROOT_RELEASE_ARTIFACTS_RELATIVE)
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
fn root_release_artifacts_ready(workspace_root: &Path) -> bool {
    if !dfx_artifact_ready(
        workspace_root,
        ROOT_WASM_ARTIFACT_RELATIVE,
        ROOT_WASM_WATCH_PATHS,
    ) {
        return false;
    }

    configured_release_roles().into_iter().all(|role| {
        let role_name = role.as_str().to_string();
        let artifact_relative_path =
            format!("{ROOT_RELEASE_ARTIFACTS_RELATIVE}/{role_name}/{role_name}.wasm.gz");
        dfx_artifact_ready(
            workspace_root,
            &artifact_relative_path,
            ROOT_WASM_WATCH_PATHS,
        )
    })
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

fn wait_for_bootstrap(pic: &Pic, root_id: Principal) {
    pic.wait_for_ready(root_id, BOOTSTRAP_TICK_LIMIT, "root bootstrap");
}

fn wait_for_children_ready(pic: &Pic, subnet_directory: &HashMap<CanisterRole, Principal>) {
    pic.wait_for_all_ready(
        subnet_directory
            .iter()
            .filter(|(role, _)| !role.is_root())
            .map(|(_, pid)| *pid),
        BOOTSTRAP_TICK_LIMIT,
        "root children bootstrap",
    );
}

/// Load the compiled root canister wasm.
fn load_root_wasm() -> Option<Vec<u8>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_path = manifest_dir.join(ROOT_WASM_RELATIVE);

    let mut candidates = env::var(ROOT_WASM_ENV)
        .ok()
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();
    candidates.push(default_path);

    for path in candidates {
        match fs::read(&path) {
            Ok(bytes) => {
                assert!(
                    bytes.len() < POCKET_IC_WASM_CHUNK_STORE_LIMIT_BYTES,
                    "root wasm artifact is too large for PocketIC chunked install: {} bytes at {}. \
Use a compressed `.wasm.gz` artifact and/or build canister wasm with `RUSTFLAGS=\"-C debuginfo=0\"`.",
                    bytes.len(),
                    path.display()
                );
                return Some(bytes);
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => panic!("failed to read root wasm at {}: {}", path.display(), err),
        }
    }

    None
}

/// Fetch the subnet directory from root as a role → principal map.
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
