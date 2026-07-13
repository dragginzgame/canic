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

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptStateView {
    Pending,
    Committed {
        source_canister: Principal,
        fingerprint: [u8; 32],
    },
    RolledBack {
        source_canister: Principal,
        fingerprint: [u8; 32],
    },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct ReceiptIntentView {
    payload_digest: [u8; 32],
    quantity: u64,
    revision: u64,
    state: ReceiptStateView,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptBeginStatus {
    Created,
    ExistingPending,
    ExistingCommitted,
    ExistingRolledBack,
    BindingConflict,
    CapacityExceeded {
        current_quantity: u64,
        requested_quantity: u64,
        limit: u64,
    },
    StoreCapacityReached {
        current_records: u64,
        limit: u64,
    },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct ReceiptBeginView {
    status: ReceiptBeginStatus,
    intent: Option<ReceiptIntentView>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptSettlementStatus {
    Settled,
    AlreadySettled,
    NotFound,
    RevisionConflict { actual_revision: u64 },
    BindingConflict,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct ReceiptSettlementView {
    status: ReceiptSettlementStatus,
    intent: Option<ReceiptIntentView>,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptDecisionView {
    Committed,
    RolledBack,
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

    assert_receipt_backed_adapter_conformance(&pic, authority_id, external_id, &authority_wasm);
}

fn assert_receipt_backed_adapter_conformance(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    evidence_source: Principal,
    authority_wasm: &[u8],
) {
    let pending = assert_pending_begin_decisions(pic, authority_id);
    assert_pending_settlement_decisions(pic, authority_id, evidence_source, &pending);
    upgrade_authority(pic, authority_id, authority_wasm);
    assert_eq!(load_receipt(pic, authority_id, 11), Some(pending.clone()));
    assert_committed_decisions(pic, authority_id, evidence_source, pending);

    let rollback_pending = assert_rollback_capacity_decisions(pic, authority_id);
    assert_rolled_back_decisions(pic, authority_id, evidence_source, rollback_pending);
}

fn assert_pending_begin_decisions(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
) -> ReceiptIntentView {
    let pending = ReceiptIntentView {
        payload_digest: [12; 32],
        quantity: 1,
        revision: 1,
        state: ReceiptStateView::Pending,
    };

    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 1, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::Created,
            intent: Some(pending.clone()),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 1, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::ExistingPending,
            intent: Some(pending.clone()),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 99, 1, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::BindingConflict,
            intent: None,
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 2, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::BindingConflict,
            intent: None,
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 21, 22, 1, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::CapacityExceeded {
                current_quantity: 1,
                requested_quantity: 1,
                limit: 1,
            },
            intent: None,
        }
    );

    pending
}

fn assert_pending_settlement_decisions(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    evidence_source: Principal,
    pending: &ReceiptIntentView,
) {
    assert_eq!(
        settle_receipt(
            pic,
            authority_id,
            99,
            99,
            1,
            ReceiptDecisionView::Committed,
            evidence_source,
            99,
        )
        .expect("missing receipt settlement"),
        ReceiptSettlementView {
            status: ReceiptSettlementStatus::NotFound,
            intent: None,
        }
    );
    assert_eq!(
        settle_receipt(
            pic,
            authority_id,
            11,
            12,
            0,
            ReceiptDecisionView::Committed,
            evidence_source,
            13,
        )
        .expect("stale receipt settlement"),
        ReceiptSettlementView {
            status: ReceiptSettlementStatus::RevisionConflict { actual_revision: 1 },
            intent: None,
        }
    );
    assert_eq!(
        settle_receipt(
            pic,
            authority_id,
            11,
            99,
            1,
            ReceiptDecisionView::Committed,
            evidence_source,
            13,
        )
        .expect("binding-conflict receipt settlement"),
        ReceiptSettlementView {
            status: ReceiptSettlementStatus::BindingConflict,
            intent: None,
        }
    );
    assert_eq!(load_receipt(pic, authority_id, 11), Some(pending.clone()),);
}

fn upgrade_authority(pic: &ic_testkit::pic::Pic, authority_id: Principal, authority_wasm: &[u8]) {
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
}

fn assert_committed_decisions(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    evidence_source: Principal,
    pending: ReceiptIntentView,
) {
    let committed = ReceiptIntentView {
        revision: 2,
        state: ReceiptStateView::Committed {
            source_canister: evidence_source,
            fingerprint: [13; 32],
        },
        ..pending
    };
    assert_eq!(
        settle_receipt(
            pic,
            authority_id,
            11,
            12,
            1,
            ReceiptDecisionView::Committed,
            evidence_source,
            13,
        )
        .expect("commit receipt settlement"),
        ReceiptSettlementView {
            status: ReceiptSettlementStatus::Settled,
            intent: Some(committed.clone()),
        }
    );
    assert_eq!(
        settle_receipt(
            pic,
            authority_id,
            11,
            12,
            1,
            ReceiptDecisionView::Committed,
            evidence_source,
            13,
        )
        .expect("replayed commit receipt settlement"),
        ReceiptSettlementView {
            status: ReceiptSettlementStatus::AlreadySettled,
            intent: Some(committed.clone()),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 1, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::ExistingCommitted,
            intent: Some(committed.clone()),
        }
    );

    let contradictory = settle_receipt(
        pic,
        authority_id,
        11,
        12,
        2,
        ReceiptDecisionView::RolledBack,
        evidence_source,
        14,
    );
    assert!(contradictory.is_err());
    assert_eq!(load_receipt(pic, authority_id, 11), Some(committed));
}

fn assert_rollback_capacity_decisions(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
) -> ReceiptIntentView {
    let rollback_pending = ReceiptIntentView {
        payload_digest: [32; 32],
        quantity: 1,
        revision: 1,
        state: ReceiptStateView::Pending,
    };
    assert_eq!(
        begin_receipt(pic, authority_id, 31, 32, 2, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::Created,
            intent: Some(rollback_pending.clone()),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 41, 42, 2, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::CapacityExceeded {
                current_quantity: 1,
                requested_quantity: 1,
                limit: 1,
            },
            intent: None,
        }
    );

    rollback_pending
}

fn assert_rolled_back_decisions(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    evidence_source: Principal,
    rollback_pending: ReceiptIntentView,
) {
    let rolled_back = ReceiptIntentView {
        revision: 2,
        state: ReceiptStateView::RolledBack {
            source_canister: evidence_source,
            fingerprint: [33; 32],
        },
        ..rollback_pending
    };
    assert_eq!(
        settle_receipt(
            pic,
            authority_id,
            31,
            32,
            1,
            ReceiptDecisionView::RolledBack,
            evidence_source,
            33,
        )
        .expect("rollback receipt settlement"),
        ReceiptSettlementView {
            status: ReceiptSettlementStatus::Settled,
            intent: Some(rolled_back.clone()),
        }
    );
    assert_eq!(
        settle_receipt(
            pic,
            authority_id,
            31,
            32,
            1,
            ReceiptDecisionView::RolledBack,
            evidence_source,
            33,
        )
        .expect("replayed rollback receipt settlement"),
        ReceiptSettlementView {
            status: ReceiptSettlementStatus::AlreadySettled,
            intent: Some(rolled_back.clone()),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 31, 32, 2, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::ExistingRolledBack,
            intent: Some(rolled_back),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 41, 42, 2, 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::Created,
            intent: Some(ReceiptIntentView {
                payload_digest: [42; 32],
                quantity: 1,
                revision: 1,
                state: ReceiptStateView::Pending,
            }),
        }
    );
}

fn begin_receipt(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    operation_seed: u8,
    payload_seed: u8,
    resource_seed: u8,
    quantity: u64,
) -> ReceiptBeginView {
    pic.update_call::<Result<ReceiptBeginView, String>, _>(
        authority_id,
        "begin_receipt",
        (operation_seed, payload_seed, resource_seed, quantity),
    )
    .expect("begin receipt transport")
    .expect("begin receipt application")
}

#[expect(clippy::too_many_arguments)]
fn settle_receipt(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    operation_seed: u8,
    payload_seed: u8,
    expected_revision: u64,
    decision: ReceiptDecisionView,
    source_canister: Principal,
    evidence_seed: u8,
) -> Result<ReceiptSettlementView, String> {
    pic.update_call::<Result<ReceiptSettlementView, String>, _>(
        authority_id,
        "settle_receipt",
        (
            operation_seed,
            payload_seed,
            expected_revision,
            decision,
            source_canister,
            evidence_seed,
        ),
    )
    .expect("settle receipt transport")
}

fn load_receipt(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    operation_seed: u8,
) -> Option<ReceiptIntentView> {
    pic.query_call::<Result<Option<ReceiptIntentView>, String>, _>(
        authority_id,
        "load_receipt",
        (operation_seed,),
    )
    .expect("load receipt transport")
    .expect("load receipt application")
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
