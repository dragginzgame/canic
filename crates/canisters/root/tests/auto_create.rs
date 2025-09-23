use candid::{Decode, Principal, encode_one};
use icu::memory::{CanisterEntry, CanisterStatus};
use icu::types::CanisterType;
use pocket_ic::PocketIc;

///
/// WASMS
///

const ROOT_WASM: &[u8] = include_bytes!("../../../../.dfx/local/canisters/root/root.wasm.gz");

#[test]
fn root_auto_creates_expected_canisters() {
    let pic = PocketIc::new();

    // Create root canister with an anonymous controller
    let root_id = pic.create_canister();

    // Give it cycles to create children
    pic.add_cycles(root_id, 100_000_000_000_000);

    // Install root WASM
    pic.install_canister(
        root_id,
        ROOT_WASM.to_vec(),
        vec![],
        Some(Principal::anonymous()),
    );

    // Timers queue `icu_install`, so tick Pocket IC until it drains
    for _ in 0..100 {
        pic.tick();
    }

    // Query the subnet registry
    let res = pic
        .query_call(
            root_id,
            Principal::anonymous(),
            "icu_subnet_registry",
            encode_one(()).unwrap(),
        )
        .expect("query registry");

    let registry: Vec<CanisterEntry> =
        Decode!(&res, Vec<CanisterEntry>).expect("decode registry entries");

    let expected = [
        (CanisterType::ROOT, None),
        (icu::canister::BLANK, Some(root_id)),
        (icu::canister::DELEGATION, Some(root_id)),
        (icu::canister::SHARDER, Some(root_id)),
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
