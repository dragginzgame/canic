use canic::{
    cdk::types::Principal,
    core::{
        PublicError,
        dto::{
            page::{Page, PageRequest},
            topology::DirectoryEntryView,
            validation::ValidationReport,
        },
        ids::CanisterRole,
        protocol,
    },
};
use canic_testkit::pic::{Pic, pic};
use std::{collections::HashMap, env, fs, io, path::PathBuf};

/// Environment variable override for providing a pre-built root canister wasm.
const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";

/// Default location of the root wasm relative to this crate’s manifest dir.
const ROOT_WASM_RELATIVE: &str = "../../.dfx/local/canisters/root/root.wasm.gz";

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

    // Allow async bootstrap / cascades to settle.
    pic.tick_n(10);

    validate_root_state(&pic, root_id);

    let subnet_directory = fetch_subnet_directory(&pic, root_id);

    RootSetup {
        pic,
        root_id,
        subnet_directory,
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
            Ok(bytes) => return Some(bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => panic!("failed to read root wasm at {}: {}", path.display(), err),
        }
    }

    None
}

/// Assert that the root canister reports a valid global state.
fn validate_root_state(pic: &Pic, root_id: Principal) {
    let report: Result<ValidationReport, PublicError> = pic
        .query_call(root_id, protocol::CANIC_ROOT_VALIDATE_STATE, ())
        .expect("validate root state transport");
    let report = report.expect("validate root state failed");

    assert!(
        report.ok,
        "root state invalid after bootstrap:\n{:#?}",
        report.issues
    );
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
