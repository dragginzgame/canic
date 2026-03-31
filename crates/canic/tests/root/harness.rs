// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{
    Error,
    cdk::{types::Principal, utils::wasm::get_wasm_hash},
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryResponse,
    },
    ids::CanisterRole,
    protocol,
};
use canic_control_plane::{
    dto::template::{TemplateChunkInput, TemplateChunkSetPrepareInput, TemplateManifestInput},
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
    collections::HashMap,
    env, fs, io,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Mutex, MutexGuard, Once},
};

/// Environment variable override for providing a pre-built root canister wasm.
const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";

/// Default location of the root wasm relative to this crate’s manifest dir.
const ROOT_WASM_RELATIVE: &str = "../../.dfx/local/canisters/root/root.wasm.gz";
const ROOT_WASM_ARTIFACT_RELATIVE: &str = ".dfx/local/canisters/root/root.wasm.gz";
const CANISTER_WASM_ROOT_RELATIVE: &str = "../../.dfx/local/canisters";
const POCKET_IC_WASM_CHUNK_STORE_LIMIT_BYTES: usize = 100 * 1024 * 1024;
const DFX_BUILD_LOCK_RELATIVE: &str = ".dfx/canic-tests-build.lock";
// Maximum management-canister chunk-store payload accepted per call. Use the
// full 1 MiB limit to keep bootstrap round-trips low without exceeding bounds.
const WASM_STORE_BOOTSTRAP_PUBLISH_CHUNK_BYTES: usize = 1024 * 1024;
const WASM_STORE_BOOTSTRAP_TEMPLATE_ID: &str = "embedded:wasm_store";
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

            stage_root_release_set(&pic, root_id);
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
        if dfx_artifact_ready(
            &workspace_root,
            ROOT_WASM_ARTIFACT_RELATIVE,
            ROOT_WASM_WATCH_PATHS,
        ) {
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

fn stage_root_release_set(pic: &Pic, root_id: Principal) {
    let mut role_artifacts = load_release_set_artifacts();
    role_artifacts.sort_by(|(left, _), (right, _)| left.as_ref().cmp(right.as_ref()));

    if let Some(index) = role_artifacts
        .iter()
        .position(|(role, _)| role == &CanisterRole::WASM_STORE)
    {
        let bootstrap = role_artifacts.remove(index);
        stage_release_role(pic, root_id, &bootstrap.0, bootstrap.1);
    }

    for (role, wasm) in role_artifacts {
        stage_release_role(pic, root_id, &role, wasm);
    }

    let resumed: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_WASM_STORE_BOOTSTRAP_RESUME_ROOT_ADMIN,
            (),
        )
        .expect("resume root bootstrap call");
    resumed.expect("resume root bootstrap");
}

fn stage_release_role(pic: &Pic, root_id: Principal, role: &CanisterRole, wasm: Vec<u8>) {
    let version = TemplateVersion::new(env!("CARGO_PKG_VERSION"));
    let payload_hash = get_wasm_hash(&wasm);
    let chunks = wasm
        .chunks(WASM_STORE_BOOTSTRAP_PUBLISH_CHUNK_BYTES)
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    let chunk_hashes = chunks
        .iter()
        .map(|chunk| get_wasm_hash(chunk))
        .collect::<Vec<_>>();

    let manifest = TemplateManifestInput {
        template_id: TemplateId::from(format!("embedded:{role}")),
        role: role.clone(),
        version: version.clone(),
        payload_hash: payload_hash.clone(),
        payload_size_bytes: wasm.len() as u64,
        store_binding: WasmStoreBinding::new("bootstrap"),
        chunking_mode: TemplateChunkingMode::Chunked,
        manifest_state: TemplateManifestState::Approved,
        approved_at: None,
        created_at: 0,
    };
    let staged_manifest: Result<(), Error> = pic
        .update_call(
            root_id,
            if role.is_wasm_store() {
                protocol::CANIC_WASM_STORE_BOOTSTRAP_STAGE_MANIFEST_ADMIN
            } else {
                protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN
            },
            (manifest,),
        )
        .expect("manifest staging call");
    staged_manifest.expect("manifest staging");

    let prepared: Result<canic_control_plane::dto::template::TemplateChunkSetInfoResponse, Error> =
        pic.update_call(
            root_id,
            if role.is_wasm_store() {
                protocol::CANIC_WASM_STORE_BOOTSTRAP_PREPARE_ADMIN
            } else {
                protocol::CANIC_TEMPLATE_PREPARE_ADMIN
            },
            (TemplateChunkSetPrepareInput {
                template_id: TemplateId::from(format!("embedded:{role}")),
                version: version.clone(),
                payload_hash,
                payload_size_bytes: wasm.len() as u64,
                chunk_hashes,
            },),
        )
        .expect("prepare call");
    prepared.expect("prepare");

    for (chunk_index, bytes) in chunks.into_iter().enumerate() {
        let published: Result<(), Error> = pic
            .update_call(
                root_id,
                if role.is_wasm_store() {
                    protocol::CANIC_WASM_STORE_BOOTSTRAP_PUBLISH_CHUNK_ADMIN
                } else {
                    protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN
                },
                (TemplateChunkInput {
                    template_id: TemplateId::from(format!("embedded:{role}")),
                    version: version.clone(),
                    chunk_index: u32::try_from(chunk_index).expect("chunk index fits"),
                    bytes,
                },),
            )
            .expect("publish chunk call");
        published.expect("publish chunk");
    }
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

/// Load the compiled wasm_store canister wasm.
fn load_release_set_artifacts() -> Vec<(CanisterRole, Vec<u8>)> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_root = manifest_dir.join(CANISTER_WASM_ROOT_RELATIVE);
    let mut artifacts = Vec::new();

    let entries = fs::read_dir(&wasm_root)
        .unwrap_or_else(|err| panic!("read_dir {}: {err}", wasm_root.display()));

    for entry in entries {
        let entry =
            entry.unwrap_or_else(|err| panic!("read_dir entry {}: {err}", wasm_root.display()));
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(role_os) = path.file_name() else {
            continue;
        };
        let role = role_os.to_string_lossy().to_string();
        if role == "root" {
            continue;
        }

        let wasm_path = path.join(format!("{role}.wasm.gz"));
        if !wasm_path.is_file() {
            continue;
        }

        let bytes = fs::read(&wasm_path)
            .unwrap_or_else(|err| panic!("failed to read wasm at {}: {err}", wasm_path.display()));
        artifacts.push((CanisterRole::from(role), bytes));
    }

    assert!(
        artifacts
            .iter()
            .any(|(role, _)| role == &CanisterRole::WASM_STORE),
        "release-set staging requires a wasm_store artifact under {}",
        wasm_root.display()
    );

    artifacts
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
