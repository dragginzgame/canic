// Category C - Artifact test (built wasm; no runtime config).

use candid::{Principal, decode_one, encode_one};
use canic_testkit::{
    artifacts::{
        WasmBuildProfile, build_wasm_canisters, read_wasm, test_target_dir, wasm_artifacts_ready,
        workspace_root_for,
    },
    pic::{acquire_pic_serial_guard, pic},
};
use std::{
    path::{Path, PathBuf},
    sync::Once,
};

const INSTALL_CYCLES: u128 = 1_000_000_000_000;
const CANISTERS: [&str; 3] = ["intent_authority", "intent_external", "intent_client"];
static BUILD_ONCE: Once = Once::new();

#[test]
fn intent_race_capacity_one() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    println!("intent_race: workspace_root={}", workspace_root.display());
    build_canisters(&workspace_root);

    let authority_wasm = read_wasm(&target_dir, "intent_authority", WasmBuildProfile::Fast);
    let external_wasm = read_wasm(&target_dir, "intent_external", WasmBuildProfile::Fast);
    let client_wasm = read_wasm(&target_dir, "intent_client", WasmBuildProfile::Fast);
    println!(
        "intent_race: wasm sizes authority={} external={} client={}",
        authority_wasm.len(),
        external_wasm.len(),
        client_wasm.len()
    );

    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    println!("intent_race: PocketIC ready");

    let external_id = pic.create_canister();
    pic.add_cycles(external_id, INSTALL_CYCLES);
    pic.install_canister(external_id, external_wasm, encode_one(()).unwrap(), None);
    println!("intent_race: installed external={external_id}");

    let authority_id = pic.create_canister();
    pic.add_cycles(authority_id, INSTALL_CYCLES);
    pic.install_canister(
        authority_id,
        authority_wasm,
        encode_one(external_id).unwrap(),
        None,
    );
    println!("intent_race: installed authority={authority_id}");

    let client_a = pic.create_canister();
    pic.add_cycles(client_a, INSTALL_CYCLES);
    pic.install_canister(client_a, client_wasm.clone(), encode_one(()).unwrap(), None);
    println!("intent_race: installed client_a={client_a}");

    let client_b = pic.create_canister();
    pic.add_cycles(client_b, INSTALL_CYCLES);
    pic.install_canister(client_b, client_wasm, encode_one(()).unwrap(), None);
    println!("intent_race: installed client_b={client_b}");

    let msg_a = pic
        .submit_call(
            client_a,
            Principal::anonymous(),
            "call_buy",
            encode_one(authority_id).unwrap(),
        )
        .expect("submit call A");
    let msg_b = pic
        .submit_call(
            client_b,
            Principal::anonymous(),
            "call_buy",
            encode_one(authority_id).unwrap(),
        )
        .expect("submit call B");
    println!("intent_race: submitted msg_a={msg_a:?} msg_b={msg_b:?}");

    pic.tick();
    pic.tick();
    println!("intent_race: ticked");

    let res_a = pic.await_call(msg_a).expect("await call A");
    let res_b = pic.await_call(msg_b).expect("await call B");
    println!("intent_race: awaited msg_a msg_b");

    let out_a: Result<(), String> = decode_one(&res_a).expect("decode call A");
    let out_b: Result<(), String> = decode_one(&res_b).expect("decode call B");
    println!("intent_race: results out_a={out_a:?} out_b={out_b:?}");

    let success_count = [out_a.is_ok(), out_b.is_ok()]
        .into_iter()
        .filter(|ok| *ok)
        .count();
    assert_eq!(success_count, 1, "expected exactly one success");
    println!("intent_race: success_count={success_count}");
}

fn build_canisters(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-wasm");
        if wasm_artifacts_ready(&target_dir, &CANISTERS, WasmBuildProfile::Fast) {
            return;
        }

        build_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTERS,
            WasmBuildProfile::Fast,
            &[],
        );
    });
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
