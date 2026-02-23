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
use std::{collections::HashMap, env, fs, io, path::PathBuf};

/// Environment variable override for providing a pre-built root canister wasm.
const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";

/// Default location of the root wasm relative to this crate’s manifest dir.
const ROOT_WASM_RELATIVE: &str = "../../.dfx/local/canisters/root/root.wasm.gz";
const BOOTSTRAP_TICK_LIMIT: usize = 120;

///
/// RootSetup
/// Result of setting up a fresh root canister for tests.
///

pub struct RootSetup {
    pub pic: Pic,
    pub root_id: Principal,
    pub subnet_directory: HashMap<CanisterRole, Principal>,
}

pub fn load_root_wasm_bytes() -> Vec<u8> {
    load_root_wasm().expect("load root wasm")
}

/// Create a fresh PocketIC instance, install root, wait for bootstrap,
/// and validate global invariants.
pub fn setup_root() -> RootSetup {
    let root_wasm = load_root_wasm().expect("load root wasm");

    let pic = pic();
    let root_id = pic
        .create_and_install_root_canister(root_wasm)
        .expect("install root canister");

    wait_for_bootstrap(&pic, root_id);

    let subnet_directory = fetch_subnet_directory(&pic, root_id);
    wait_for_children_ready(&pic, &subnet_directory);

    RootSetup {
        pic,
        root_id,
        subnet_directory,
    }
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
