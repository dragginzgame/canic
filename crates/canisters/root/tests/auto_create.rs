use std::{env, fs, io, path::PathBuf};

use candid::{Decode, Principal, encode_one};
use canic::{
    PublicError,
    core::{
        dto::{
            canister::CanisterEntryView,
            rpc::{CreateCanisterParent, CreateCanisterRequest, Request, Response},
            state::{AppCommand, AppModeView, AppStateView},
            subnet::SubnetIdentity,
            topology::SubnetRegistryView,
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

fn tick_n(pic: &PocketIc, times: usize) {
    for _ in 0..times {
        pic.tick();
    }
}

fn fetch_registry(pic: &PocketIc, root_id: Principal) -> Vec<(CanisterRole, CanisterEntryView)> {
    let res = pic
        .query_call(
            root_id,
            Principal::anonymous(),
            "canic_subnet_registry",
            encode_one(()).expect("encode registry args"),
        )
        .expect("query registry");

    let SubnetRegistryView(registry) =
        Decode!(&res, SubnetRegistryView).expect("decode registry entries");

    registry
}

fn registry_has_role(registry: &[(CanisterRole, CanisterEntryView)], role: &CanisterRole) -> bool {
    registry.iter().any(|(entry_role, _)| entry_role == role)
}

fn root_response(pic: &PocketIc, root_id: Principal, request: Request) -> Response {
    let res = pic
        .update_call(
            root_id,
            root_id,
            "canic_response",
            encode_one(request).expect("encode canic_response args"),
        )
        .expect("call canic_response");

    let response: Result<Response, PublicError> =
        Decode!(&res, Result<Response, PublicError>).expect("decode canic_response result");

    response.expect("canic_response failed")
}

fn ensure_root_canister(pic: &PocketIc, root_id: Principal, role: CanisterRole) {
    let registry = fetch_registry(pic, root_id);
    if registry_has_role(&registry, &role) {
        return;
    }

    let request = Request::CreateCanister(CreateCanisterRequest {
        canister_role: role.clone(),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
    });

    match root_response(pic, root_id, request) {
        Response::CreateCanister(_) => {}
        other => panic!("unexpected response for {role}: {other:?}"),
    }
}

///
/// TESTS
///

#[test]
fn root_registers_explicit_canisters() {
    let Some(root_wasm) = load_root_wasm() else {
        eprintln!(
            "skipping root_registers_explicit_canisters — run `make test` to build canisters or set {ROOT_WASM_ENV}"
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

    // NOTE: Tests explicitly create required canisters.
    // Auto-create is async and must not be relied upon here.
    let required = [
        canister::AUTH,
        canister::BLANK,
        canister::SCALE_HUB,
        canister::SHARD_HUB,
    ];

    for role in required {
        ensure_root_canister(&pic, root_id, role);
    }

    tick_n(&pic, 5);

    let registry = fetch_registry(&pic, root_id);

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
    tick_n(&pic, 5);

    // Enable the app state on root (cascades to existing children).
    let res = pic
        .update_call(
            root_id,
            Principal::anonymous(),
            "canic_app",
            encode_one(AppCommand::Start).unwrap(),
        )
        .expect("call canic_app");
    let app_result: Result<(), PublicError> =
        Decode!(&res, Result<(), PublicError>).expect("decode canic_app response");
    if let Err(err) = app_result {
        panic!("canic_app failed: {err}");
    }

    // Create a new blank canister after app state is enabled.
    let request = Request::CreateCanister(CreateCanisterRequest {
        canister_role: canister::BLANK,
        parent: CreateCanisterParent::Root,
        extra_arg: None,
    });
    let response = root_response(&pic, root_id, request);
    let new_pid = match response {
        Response::CreateCanister(resp) => resp.new_canister_pid,
        other => panic!("unexpected response for create_blank: {other:?}"),
    };

    tick_n(&pic, 10);

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
