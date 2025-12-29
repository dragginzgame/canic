use std::{env, fs, io, path::PathBuf};

use candid::{Decode, Principal, encode_one};
use canic::{
    Error,
    core::{
        dto::{
            registry::SubnetRegistryView,
            rpc::CreateCanisterResponse,
            state::{AppCommand, AppModeView, AppStateView},
            subnet::SubnetIdentity,
        },
        ids::CanisterRole,
    },
};
use canic_internal::canister;
use pocket_ic::PocketIc;

const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";
const ROOT_WASM_RELATIVE: &str = "../../../../.dfx/local/canisters/root/root.wasm.gz";

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

///
/// TESTS
///

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

    let SubnetRegistryView(registry) =
        Decode!(&res, SubnetRegistryView).expect("decode registry entries");

    let expected = [
        (CanisterRole::ROOT, None),
        (canister::AUTH, Some(root_id)),
        (canister::BLANK, Some(root_id)),
        (canister::SCALE_HUB, Some(root_id)),
        (canister::SHARD_HUB, Some(root_id)),
    ];

    for (role, parent) in expected {
        let entry = registry
            .iter()
            .find_map(|(entry_role, entry)| (entry_role == &role).then_some(entry))
            .unwrap_or_else(|| panic!("missing {role} entry"));

        assert_eq!(entry.parent_pid, parent, "unexpected parent for {role}");
    }
}

#[test]
fn new_canister_inherits_app_state_after_enable() {
    let Some(root_wasm) = load_root_wasm() else {
        eprintln!(
            "skipping new_canister_inherits_app_state_after_enable — run `make test` to build canisters or set {ROOT_WASM_ENV}"
        );
        return;
    };

    let pic = PocketIc::new();

    // Create root canister with an anonymous controller
    let root_id = pic.create_canister();

    // Give it cycles to create children
    pic.add_cycles(root_id, 100_000_000_000_000);

    let init_args = encode_one(SubnetIdentity::Manual(Principal::from_slice(&[0xAA; 29])))
        .expect("encode root init args");

    // Install root WASM
    pic.install_canister(root_id, root_wasm, init_args, Some(Principal::anonymous()));

    // Allow root bootstrap timers to complete.
    for _ in 0..100 {
        pic.tick();
    }

    // Enable the app state on root (cascades to existing children).
    let res = pic
        .update_call(
            root_id,
            Principal::anonymous(),
            "canic_app",
            encode_one(AppCommand::Start).unwrap(),
        )
        .expect("call canic_app");
    let app_result: Result<(), Error> =
        Decode!(&res, Result<(), Error>).expect("decode canic_app response");
    if let Err(err) = app_result {
        panic!("canic_app failed: {err}");
    }

    // Create a new blank canister after app state is enabled.
    let res = pic
        .update_call(
            root_id,
            Principal::anonymous(),
            "create_blank",
            encode_one(()).unwrap(),
        )
        .expect("call create_blank");
    let create_result: Result<CreateCanisterResponse, Error> =
        Decode!(&res, Result<CreateCanisterResponse, Error>).expect("decode create_blank response");
    let new_pid = create_result.expect("create_blank failed").new_canister_pid;

    for _ in 0..10 {
        pic.tick();
    }

    // The newly created canister should inherit Enabled mode.
    let res = pic
        .query_call(
            new_pid,
            Principal::anonymous(),
            "canic_app_state",
            encode_one(()).unwrap(),
        )
        .expect("query canic_app_state");
    let app_state: AppStateView = Decode!(&res, AppStateView).expect("decode canic_app_state");
    assert_eq!(
        app_state.mode,
        AppModeView::Enabled,
        "new canister should inherit Enabled app state"
    );
}
