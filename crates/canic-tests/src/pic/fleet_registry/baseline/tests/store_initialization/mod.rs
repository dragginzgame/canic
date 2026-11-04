//! Store lifecycle qualification with the real generated adapter and a held user hook.

use super::*;
use crate::pic::CanicPicExt;
use ic_testkit::pic::CanisterInstallExt;
use ic_testkit::pocket_ic::common::rest::{
    CanisterHttpReply, CanisterHttpResponse, MockCanisterHttpResponse,
};

#[test]
pub(super) fn store_bootstrap_survives_held_pending_and_trapping_user_init() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = crate::workspace::discover().unwrap();
    let config = initial_shard_root_canister_config_path(&workspace);
    let absent_hook_wasm = crate::pic::lifecycle::observed_initialization_store_wasm();
    let wasm = crate::pic::lifecycle::held_initialization_store_wasm();
    let pic = build_pic();
    let coordinator = pic.create_canister().unwrap();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES)
        .unwrap();
    let fixture = install_fixture_root_with_store(
        &pic,
        coordinator,
        &config,
        &[],
        build_initial_shard_root_wasm(),
        build_initial_shard_component_wasms(),
        Some(wasm.clone()),
    );
    let root = fixture.root_id;
    let store = fixture.response.wasm_store;
    let (version, synchronization) = join_and_synchronize_root(&pic, coordinator, &fixture);
    activate_registry_and_prepare_component_registry(
        &pic,
        coordinator,
        &fixture,
        version,
        synchronization,
    );
    activate_root(&pic, root);
    for (index, mode) in ["complete", "pending", "trap"].into_iter().enumerate() {
        if index != 0 {
            pic.wait_out_install_code_rate_limit(Duration::from_mins(5))
                .unwrap();
            pic.upgrade_canister(store, wasm.clone(), crate::pic::upgrade_args(), Some(root))
                .unwrap();
        }
        let mut held = Vec::new();
        for _ in 0..120 {
            pic.tick().unwrap();
            held = pic.get_canister_http().unwrap();
            if !held.is_empty() {
                break;
            }
        }
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].url, "https://store-initialization.invalid/94");
        pic.wait_for_ready(
            store,
            root,
            120,
            "Store bootstrap while user initialization is held",
        );
        for _ in 0..20 {
            pic.tick().unwrap();
        }
        assert_eq!(pic.get_canister_http().unwrap().len(), 1);
        pic.mock_canister_http_response(MockCanisterHttpResponse {
            subnet_id: held[0].subnet_id,
            request_id: held[0].request_id,
            response: CanisterHttpResponse::CanisterHttpReply(CanisterHttpReply {
                status: 200,
                headers: Vec::new(),
                body: mode.as_bytes().to_vec(),
            }),
            additional_responses: Vec::new(),
        })
        .unwrap();
        for _ in 0..20 {
            pic.tick().unwrap();
        }
        pic.wait_for_ready(
            store,
            root,
            120,
            "Store readiness survives user callback outcome",
        );
    }
    pic.wait_out_install_code_rate_limit(Duration::from_mins(5))
        .unwrap();
    pic.upgrade_canister(
        store,
        absent_hook_wasm,
        crate::pic::upgrade_args(),
        Some(root),
    )
    .unwrap();
    pic.wait_for_ready(
        store,
        root,
        120,
        "Store bootstrap without optional user initialization",
    );
    assert!(pic.get_canister_http().unwrap().is_empty());
}
