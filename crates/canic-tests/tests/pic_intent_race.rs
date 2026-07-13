// Category C - Artifact test (built wasm; no runtime config).

use candid::{CandidType, Deserialize, Principal, decode_one, encode_one};
use canic_testing_internal::pic::{CanicWasmBuildProfile, build_internal_test_wasm_canisters};
use ic_testkit::{
    artifacts::{read_wasm, test_target_dir, workspace_root_for},
    pic::{InstallSpec, acquire_pic_serial_guard, pic},
};
use std::{
    path::{Path, PathBuf},
    sync::Once,
    time::Duration,
};

const INSTALL_CYCLES: u128 = 1_000_000_000_000;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);
const INSTALL_CODE_RETRY_LIMIT: usize = 3;
const CANISTERS: [&str; 3] = ["intent_authority", "intent_external", "intent_client"];
static BUILD_ONCE: Once = Once::new();

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptStateView {
    Pending,
    Committed { fingerprint: [u8; 32] },
    RolledBack { fingerprint: [u8; 32] },
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
struct ReceiptIntentView {
    payload_digest: [u8; 32],
    quantity: u64,
    revision: u64,
    state: ReceiptStateView,
}

#[test]
fn intent_race_capacity_one() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    println!("intent_race: workspace_root={}", workspace_root.display());
    build_canisters(&workspace_root);

    let profile_dir = CanicWasmBuildProfile::Fast.target_dir_name();
    let authority_wasm = read_wasm(&target_dir, "intent_authority", profile_dir);
    let external_wasm = read_wasm(&target_dir, "intent_external", profile_dir);
    let client_wasm = read_wasm(&target_dir, "intent_client", profile_dir);
    println!(
        "intent_race: wasm sizes authority={} external={} client={}",
        authority_wasm.len(),
        external_wasm.len(),
        client_wasm.len()
    );

    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    println!("intent_race: PocketIC ready");

    let external_id = pic.create_and_install(
        InstallSpec::new(external_wasm, encode_one(()).unwrap(), INSTALL_CYCLES)
            .label("intent_external"),
    );
    println!("intent_race: installed external={external_id}");

    let authority_id = pic.create_and_install(
        InstallSpec::new(
            authority_wasm.clone(),
            encode_one(external_id).unwrap(),
            INSTALL_CYCLES,
        )
        .label("intent_authority"),
    );
    println!("intent_race: installed authority={authority_id}");

    let client_a = pic.create_and_install(
        InstallSpec::new(client_wasm.clone(), encode_one(()).unwrap(), INSTALL_CYCLES)
            .label("intent_client_a"),
    );
    println!("intent_race: installed client_a={client_a}");

    let client_b = pic.create_and_install(
        InstallSpec::new(client_wasm, encode_one(()).unwrap(), INSTALL_CYCLES)
            .label("intent_client_b"),
    );
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

    assert_receipt_backed_facade_survives_upgrade(&pic, authority_id, &authority_wasm);
}

fn assert_receipt_backed_facade_survives_upgrade(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    authority_wasm: &[u8],
) {
    let operation_seed = 11_u8;
    let payload_seed = 12_u8;
    let evidence_seed = 13_u8;
    let pending = ReceiptIntentView {
        payload_digest: [payload_seed; 32],
        quantity: 1,
        revision: 1,
        state: ReceiptStateView::Pending,
    };

    let created: Option<ReceiptIntentView> = pic
        .update_call::<Result<Option<ReceiptIntentView>, String>, _>(
            authority_id,
            "begin_receipt",
            (operation_seed, payload_seed, 1_u64),
        )
        .expect("begin receipt transport")
        .expect("begin receipt application");
    assert_eq!(created, Some(pending));

    let replayed: Option<ReceiptIntentView> = pic
        .update_call::<Result<Option<ReceiptIntentView>, String>, _>(
            authority_id,
            "begin_receipt",
            (operation_seed, payload_seed, 1_u64),
        )
        .expect("replay receipt transport")
        .expect("replay receipt application");
    assert_eq!(replayed, Some(pending));

    let rejected: Option<ReceiptIntentView> = pic
        .update_call::<Result<Option<ReceiptIntentView>, String>, _>(
            authority_id,
            "begin_receipt",
            (21_u8, 22_u8, 1_u64),
        )
        .expect("capacity receipt transport")
        .expect("capacity receipt application");
    assert_eq!(rejected, None);

    pic.wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    pic.retry_install_code_ok(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN, || {
        pic.upgrade_canister(
            authority_id,
            authority_wasm.to_vec(),
            encode_one(()).expect("encode authority upgrade"),
            None,
        )
        .map_err(|err| err.to_string())
    })
    .expect("upgrade intent authority");

    let after_upgrade: Option<ReceiptIntentView> = pic
        .query_call::<Result<Option<ReceiptIntentView>, String>, _>(
            authority_id,
            "load_receipt",
            (operation_seed,),
        )
        .expect("load receipt after upgrade transport")
        .expect("load receipt after upgrade application");
    assert_eq!(after_upgrade, Some(pending));

    let committed = ReceiptIntentView {
        revision: 2,
        state: ReceiptStateView::Committed {
            fingerprint: [evidence_seed; 32],
        },
        ..pending
    };
    let settled: Option<ReceiptIntentView> = pic
        .update_call::<Result<Option<ReceiptIntentView>, String>, _>(
            authority_id,
            "commit_receipt",
            (operation_seed, payload_seed, evidence_seed),
        )
        .expect("settle receipt transport")
        .expect("settle receipt application");
    assert_eq!(settled, Some(committed));

    let settled_replay: Option<ReceiptIntentView> = pic
        .update_call::<Result<Option<ReceiptIntentView>, String>, _>(
            authority_id,
            "commit_receipt",
            (operation_seed, payload_seed, evidence_seed),
        )
        .expect("replay settlement transport")
        .expect("replay settlement application");
    assert_eq!(settled_replay, Some(committed));
}

fn build_canisters(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-wasm");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTERS,
            CanicWasmBuildProfile::Fast,
        );
    });
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
