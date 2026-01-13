use canic::{
    cdk::types::Principal,
    dto::{
        page::{Page, PageRequest},
        topology::{DirectoryEntryView, SubnetRegistryEntryView},
    },
    ids::CanisterRole,
    protocol,
};
use canic_internal::canister;
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

    RootSetup {
        pic,
        root_id,
        subnet_directory,
    }
}

fn wait_for_bootstrap(pic: &Pic, root_id: Principal) {
    let expected_roles = [
        CanisterRole::ROOT,
        canister::APP,
        canister::AUTH,
        canister::SCALE_HUB,
        canister::SHARD_HUB,
        canister::TEST,
    ];

    for _ in 0..BOOTSTRAP_TICK_LIMIT {
        pic.tick();

        let registry = fetch_registry(pic, root_id);
        if expected_roles
            .iter()
            .all(|role| registry.iter().any(|entry| &entry.role == role))
        {
            return;
        }
    }

    let registry = fetch_registry(pic, root_id);
    let roles: Vec<CanisterRole> = registry.into_iter().map(|entry| entry.role).collect();
    panic!(
        "root bootstrap did not create required canisters after {BOOTSTRAP_TICK_LIMIT} ticks; registry roles: {roles:?}"
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
            Ok(bytes) => return Some(bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => panic!("failed to read root wasm at {}: {}", path.display(), err),
        }
    }

    None
}

fn fetch_registry(pic: &Pic, root_id: Principal) -> Vec<SubnetRegistryEntryView> {
    pic.query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query registry")
}

/// Fetch the subnet directory from root as a role → principal map.
fn fetch_subnet_directory(pic: &Pic, root_id: Principal) -> HashMap<CanisterRole, Principal> {
    let page: Page<DirectoryEntryView> = pic
        .query_call(
            root_id,
            protocol::CANIC_SUBNET_DIRECTORY,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("query subnet directory");

    page.entries
        .into_iter()
        .map(|entry| (entry.role, entry.pid))
        .collect()
}
