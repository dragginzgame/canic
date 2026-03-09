// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{
    cdk::types::Principal,
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryResponse,
    },
    ids::CanisterRole,
    protocol,
};
use canic_testkit::pic::{Pic, pic};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    env, fs, io,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard, Once, TryLockError},
    thread,
    time::{Duration, Instant},
};

/// Environment variable override for providing a pre-built root canister wasm.
const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";

/// Default location of the root wasm relative to this crate’s manifest dir.
const ROOT_WASM_RELATIVE: &str = "../../.dfx/local/canisters/root/root.wasm.gz";
const POCKET_IC_WASM_CHUNK_STORE_LIMIT_BYTES: usize = 100 * 1024 * 1024;
const DFX_BUILD_LOCK_RELATIVE: &str = ".dfx/canic-tests-build.lock";
// WARNING: `Pic` MUST NOT be cached/shared across tests by default.
// This toggle is intentionally opt-in for local experimentation only.
// Enabling it can reintroduce hangs or flaky behavior from retained runtime state.
const ROOT_SETUP_CACHE_ENV: &str = "CANIC_TEST_ROOT_SETUP_CACHE";
const BOOTSTRAP_TICK_LIMIT: usize = 120;
const ROOT_SETUP_MAX_ATTEMPTS: usize = 2;
static DFX_BUILD_ONCE: Once = Once::new();
static ROOT_SETUP_SERIAL: Mutex<()> = Mutex::new(());
thread_local! {
    static ROOT_SETUP_CACHE: RefCell<Option<ManuallyDrop<RootSetupState>>> = const { RefCell::new(None) };
}

pub struct RootSetupState {
    pub pic: Pic,
    pub root_id: Principal,
    pub subnet_directory: HashMap<CanisterRole, Principal>,
    baseline_snapshots: Option<HashMap<Principal, BaselineSnapshot>>,
}

struct BaselineSnapshot {
    snapshot_id: Vec<u8>,
    sender: Option<Principal>,
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

// Acquire the global setup lock with a bounded wait so blocked tests fail fast.
fn acquire_root_setup_serial_guard() -> MutexGuard<'static, ()> {
    const WAIT_STEP: Duration = Duration::from_millis(100);
    const WAIT_WARN: Duration = Duration::from_secs(5);
    const WAIT_TIMEOUT: Duration = Duration::from_secs(120);

    let started = Instant::now();
    let mut warned = false;

    loop {
        match ROOT_SETUP_SERIAL.try_lock() {
            Ok(guard) => return guard,
            Err(TryLockError::Poisoned(err)) => return err.into_inner(),
            Err(TryLockError::WouldBlock) => {
                let elapsed = started.elapsed();
                if !warned && elapsed >= WAIT_WARN {
                    warned = true;
                    eprintln!(
                        "setup_root: waiting for setup lock (>{}s); another root test is still active",
                        WAIT_WARN.as_secs()
                    );
                }

                assert!(
                    elapsed < WAIT_TIMEOUT,
                    "setup_root: timed out after {}s waiting for setup lock",
                    WAIT_TIMEOUT.as_secs()
                );

                thread::sleep(WAIT_STEP);
            }
        }
    }
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

            wait_for_bootstrap(&pic, root_id);

            let subnet_directory = fetch_subnet_directory(&pic, root_id);
            wait_for_children_ready(&pic, &subnet_directory);
            let baseline_snapshots = if root_setup_cache_enabled() {
                capture_baseline_snapshots(&pic, root_id, &subnet_directory)
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

    for (canister_id, baseline) in baselines {
        restore_canister_snapshot_with_sender(&state.pic, state.root_id, *canister_id, baseline);
    }

    wait_for_bootstrap(&state.pic, state.root_id);
    wait_for_children_ready(&state.pic, &state.subnet_directory);
}

fn capture_baseline_snapshots(
    pic: &Pic,
    root_id: Principal,
    subnet_directory: &HashMap<CanisterRole, Principal>,
) -> Option<HashMap<Principal, BaselineSnapshot>> {
    let mut tracked = HashSet::new();
    tracked.insert(root_id);
    tracked.extend(subnet_directory.values().copied());

    let mut snapshots = HashMap::new();
    for canister_id in tracked {
        let Some(baseline) = try_take_canister_snapshot_with_sender(pic, root_id, canister_id)
        else {
            eprintln!(
                "setup_root: snapshot capture unavailable for {canister_id}; disabling root setup cache"
            );
            return None;
        };
        snapshots.insert(canister_id, baseline);
    }

    Some(snapshots)
}

fn try_take_canister_snapshot_with_sender(
    pic: &Pic,
    root_id: Principal,
    canister_id: Principal,
) -> Option<BaselineSnapshot> {
    let candidates = snapshot_sender_candidates(root_id, canister_id);
    let mut last_err = None;

    for sender in candidates {
        match pic.take_canister_snapshot(canister_id, sender, None) {
            Ok(snapshot) => {
                return Some(BaselineSnapshot {
                    snapshot_id: snapshot.id,
                    sender,
                });
            }
            Err(err) => last_err = Some((sender, err)),
        }
    }

    if let Some((sender, err)) = last_err {
        eprintln!(
            "failed to capture canister snapshot for {canister_id} using sender {sender:?}: {err}"
        );
    }
    None
}

// Prefer the likely controller sender first to avoid expected management-call
// rejections being printed by PocketIC during snapshot capture.
fn snapshot_sender_candidates(
    root_id: Principal,
    canister_id: Principal,
) -> [Option<Principal>; 2] {
    if canister_id == root_id {
        [None, Some(root_id)]
    } else {
        [Some(root_id), None]
    }
}

fn restore_canister_snapshot_with_sender(
    pic: &Pic,
    root_id: Principal,
    canister_id: Principal,
    baseline: &BaselineSnapshot,
) {
    let fallback_sender = if baseline.sender.is_some() {
        None
    } else {
        Some(root_id)
    };
    let candidates = [baseline.sender, fallback_sender];
    let mut last_err = None;

    for sender in candidates {
        match pic.load_canister_snapshot(canister_id, sender, baseline.snapshot_id.clone()) {
            Ok(()) => return,
            Err(err) => last_err = Some((sender, err)),
        }
    }

    let (sender, err) = last_err.expect("snapshot restore must have at least one sender attempt");
    panic!("failed to restore canister snapshot for {canister_id} using sender {sender:?}: {err}");
}

fn ensure_local_artifacts_built() {
    DFX_BUILD_ONCE.call_once(|| {
        let workspace_root = workspace_root();

        // `make test` already builds canisters before `cargo test`; avoid redundant
        // `dfx build --all` work unless artifacts are missing.
        if local_artifacts_ready(&workspace_root) {
            return;
        }

        let output = run_dfx_build_with_lock(&workspace_root);
        assert!(
            output.status.success(),
            "dfx build --all failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}

fn local_artifacts_ready(workspace_root: &Path) -> bool {
    let root_wasm = workspace_root.join(".dfx/local/canisters/root/root.wasm.gz");
    match fs::metadata(root_wasm) {
        Ok(meta) => meta.is_file() && meta.len() > 0,
        Err(_) => false,
    }
}

fn run_dfx_build_with_lock(workspace_root: &Path) -> std::process::Output {
    let lock_file = workspace_root.join(DFX_BUILD_LOCK_RELATIVE);
    if let Some(parent) = lock_file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Use a file lock so multiple integration-test binaries do not race on
    // `.dfx` artifacts and Cargo's shared target directories.
    match Command::new("flock")
        .current_dir(workspace_root)
        .arg(lock_file.as_os_str())
        .arg("dfx")
        .env("DFX_NETWORK", "local")
        .env("RELEASE", "0")
        .args(["build", "--all"])
        .output()
    {
        Ok(output) => output,
        Err(err) if err.kind() == io::ErrorKind::NotFound => run_dfx_build(workspace_root),
        Err(err) => panic!("failed to run `flock` for `dfx build --all`: {err}"),
    }
}

fn run_dfx_build(workspace_root: &Path) -> std::process::Output {
    Command::new("dfx")
        .current_dir(workspace_root)
        .env("DFX_NETWORK", "local")
        .env("RELEASE", "0")
        .args(["build", "--all"])
        .output()
        .expect("failed to run `dfx build --all`")
}

fn wait_for_bootstrap(pic: &Pic, root_id: Principal) {
    for _ in 0..BOOTSTRAP_TICK_LIMIT {
        pic.tick();
        if fetch_ready(pic, root_id) {
            return;
        }
    }

    panic!("root bootstrap did not signal readiness after {BOOTSTRAP_TICK_LIMIT} ticks");
}

fn wait_for_children_ready(pic: &Pic, subnet_directory: &HashMap<CanisterRole, Principal>) {
    let child_pids: Vec<Principal> = subnet_directory
        .iter()
        .filter(|(role, _)| !role.is_root())
        .map(|(_, pid)| *pid)
        .collect();

    for _ in 0..BOOTSTRAP_TICK_LIMIT {
        pic.tick();
        let all_children_ready = child_pids.iter().all(|pid| fetch_ready(pic, *pid));

        if all_children_ready {
            return;
        }
    }

    panic!("children did not become ready after {BOOTSTRAP_TICK_LIMIT} ticks");
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

fn fetch_ready(pic: &Pic, canister_id: Principal) -> bool {
    pic.query_call(canister_id, protocol::CANIC_READY, ())
        .expect("query canic_ready")
}
