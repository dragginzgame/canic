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
    collections::HashMap,
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard, Once},
};

/// Environment variable override for providing a pre-built root canister wasm.
const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";

/// Default location of the root wasm relative to this crate’s manifest dir.
const ROOT_WASM_RELATIVE: &str = "../../.dfx/local/canisters/root/root.wasm.gz";
const DFX_BUILD_LOCK_RELATIVE: &str = ".dfx/canic-tests-build.lock";
const BOOTSTRAP_TICK_LIMIT: usize = 120;
const ROOT_SETUP_MAX_ATTEMPTS: usize = 2;
static DFX_BUILD_ONCE: Once = Once::new();
static ROOT_SETUP_SERIAL: Mutex<()> = Mutex::new(());

///
/// RootSetup
/// Result of setting up a fresh root canister for tests.
///

pub struct RootSetup {
    pub pic: Pic,
    pub root_id: Principal,
    pub subnet_directory: HashMap<CanisterRole, Principal>,
    _serial_guard: MutexGuard<'static, ()>,
}

/// Create a fresh PocketIC instance, install root, wait for bootstrap,
/// and validate global invariants.
pub fn setup_root() -> RootSetup {
    // Each setup spins up a full PocketIC topology; serialize to avoid
    // exhausting local temp storage under parallel test execution.
    let serial_guard = ROOT_SETUP_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    ensure_local_artifacts_built();
    let root_wasm = load_root_wasm().expect("load root wasm");

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

            (pic, root_id, subnet_directory)
        }));

        match attempt_result {
            Ok((pic, root_id, subnet_directory)) => {
                return RootSetup {
                    pic,
                    root_id,
                    subnet_directory,
                    _serial_guard: serial_guard,
                };
            }
            Err(err) if attempt < ROOT_SETUP_MAX_ATTEMPTS => {
                eprintln!(
                    "setup_root attempt {attempt}/{ROOT_SETUP_MAX_ATTEMPTS} failed; retrying"
                );
                drop(err);
            }
            Err(err) => std::panic::resume_unwind(err),
        }
    }

    unreachable!("setup_root must return or panic");
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
            Ok(bytes) => return Some(bytes),
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
