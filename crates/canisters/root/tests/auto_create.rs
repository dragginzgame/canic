use std::{env, fs, io, path::PathBuf};

use candid::{Decode, Principal, encode_one};
use canic::memory::{CanisterEntry, CanisterStatus};
use canic::types::CanisterType;
use pocket_ic::PocketIc;

const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";
const ROOT_WASM_RELATIVE: &str = "../../../../.dfx/local/canisters/root/root.wasm.gz";

fn load_root_wasm() -> Option<Vec<u8>> {
    if cfg!(canic_github_ci) {
        return None;
    }

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

#[test]
fn root_auto_creates_expected_canisters() {
    let Some(root_wasm) = load_root_wasm() else {
        eprintln!(
            "skipping root_auto_creates_expected_canisters — run `make test` to build canisters or set {ROOT_WASM_ENV}"
        );
        return;
    };

    let pic = PocketIc::new();

    // Create root canister with an anonymous controller
    let root_id = pic.create_canister();

    // Give it cycles to create children
    pic.add_cycles(root_id, 100_000_000_000_000);

    // Install root WASM
    pic.install_canister(root_id, root_wasm, vec![], Some(Principal::anonymous()));

    // Timers queue `canic_install`, so tick Pocket IC until it drains
    for _ in 0..100 {
        pic.tick();
    }

    // Query the subnet registry
    let res = pic
        .query_call(
            root_id,
            Principal::anonymous(),
            "canic_subnet_registry",
            encode_one(()).unwrap(),
        )
        .expect("query registry");

    let registry: Vec<CanisterEntry> =
        Decode!(&res, Vec<CanisterEntry>).expect("decode registry entries");

    let expected = [
        (CanisterType::ROOT, None),
        (canic::canister::BLANK, Some(root_id)),
        (canic::canister::DELEGATION, Some(root_id)),
        (canic::canister::SCALE_HUB, Some(root_id)),
        (canic::canister::SHARD_HUB, Some(root_id)),
    ];

    for (ty, parent) in expected {
        let entry = registry
            .iter()
            .find(|entry| entry.ty == ty)
            .unwrap_or_else(|| panic!("missing {ty} entry"));

        assert_eq!(
            entry.status,
            CanisterStatus::Installed,
            "{ty} not installed"
        );
        assert_eq!(entry.parent_pid, parent, "unexpected parent for {ty}");
    }
}
