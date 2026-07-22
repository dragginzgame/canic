// Category C - Artifact test (built wasm; no runtime config).

use candid::{CandidType, Deserialize, Principal, encode_one};
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
const NANOS_PER_HOUR: u64 = 60 * 60 * 1_000_000_000;
const MAX_REPLAY_WINDOW_NS: u64 = NANOS_PER_HOUR;
const CANISTERS: [&str; 1] = ["intent_authority"];
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
    ReplayWindowClosed {
        replay_deadline_ns: u64,
    },
    ReplayWindowTooLong {
        remaining_ns: u64,
        maximum_ns: u64,
    },
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
fn receipt_backed_intent_conformance() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    println!(
        "receipt_backed_intent: workspace_root={}",
        workspace_root.display()
    );
    build_canisters(&workspace_root);

    let profile_dir = CanicWasmBuildProfile::Fast.target_dir_name();
    let authority_wasm = read_wasm(&target_dir, "intent_authority", profile_dir);
    println!(
        "receipt_backed_intent: wasm size authority={}",
        authority_wasm.len()
    );

    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    println!("receipt_backed_intent: PocketIC ready");

    let authority_id = pic.create_and_install(
        InstallSpec::new(
            authority_wasm.clone(),
            encode_one(()).unwrap(),
            INSTALL_CYCLES,
        )
        .label("intent_authority"),
    );
    println!("receipt_backed_intent: installed authority={authority_id}");

    assert_receipt_backed_adapter_conformance(&pic, authority_id, authority_id, &authority_wasm);
}

fn assert_receipt_backed_adapter_conformance(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    evidence_source: Principal,
    authority_wasm: &[u8],
) {
    let replay_deadline_ns = pic.current_time_nanos() + NANOS_PER_HOUR;
    let pending = assert_pending_begin_decisions(pic, authority_id, replay_deadline_ns);
    assert_pending_settlement_decisions(pic, authority_id, evidence_source, &pending);
    upgrade_authority(pic, authority_id, authority_wasm);
    assert_eq!(load_receipt(pic, authority_id, 11), Some(pending.clone()));
    assert_committed_decisions(
        pic,
        authority_id,
        evidence_source,
        pending,
        replay_deadline_ns,
    );

    let rollback_pending =
        assert_rollback_capacity_decisions(pic, authority_id, replay_deadline_ns);
    assert_rolled_back_decisions(
        pic,
        authority_id,
        evidence_source,
        rollback_pending,
        replay_deadline_ns,
    );
    assert_terminal_reclamation(pic, authority_id, replay_deadline_ns);
}

fn assert_pending_begin_decisions(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    replay_deadline_ns: u64,
) -> ReceiptIntentView {
    let pending = ReceiptIntentView {
        payload_digest: [12; 32],
        quantity: 1,
        revision: 1,
        state: ReceiptStateView::Pending,
    };

    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 1, 1, replay_deadline_ns),
        ReceiptBeginView {
            status: ReceiptBeginStatus::Created,
            intent: Some(pending.clone()),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 1, 1, replay_deadline_ns),
        ReceiptBeginView {
            status: ReceiptBeginStatus::ExistingPending,
            intent: Some(pending.clone()),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 99, 1, 1, replay_deadline_ns),
        ReceiptBeginView {
            status: ReceiptBeginStatus::BindingConflict,
            intent: None,
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 2, 1, replay_deadline_ns),
        ReceiptBeginView {
            status: ReceiptBeginStatus::BindingConflict,
            intent: None,
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 1, 1, replay_deadline_ns + 1),
        ReceiptBeginView {
            status: ReceiptBeginStatus::BindingConflict,
            intent: None,
        }
    );

    let now_ns = pic.current_time_nanos();
    assert_eq!(
        begin_receipt(pic, authority_id, 51, 52, 1, 1, now_ns),
        ReceiptBeginView {
            status: ReceiptBeginStatus::ReplayWindowClosed {
                replay_deadline_ns: now_ns,
            },
            intent: None,
        }
    );
    assert_eq!(load_receipt(pic, authority_id, 51), None);

    let maximum_ns = MAX_REPLAY_WINDOW_NS;
    let overlong_deadline_ns = pic.current_time_nanos() + maximum_ns + NANOS_PER_HOUR;
    let overlong = begin_receipt(pic, authority_id, 53, 54, 1, 1, overlong_deadline_ns);
    let ReceiptBeginStatus::ReplayWindowTooLong {
        remaining_ns,
        maximum_ns: observed_maximum_ns,
    } = overlong.status
    else {
        panic!("overlong receipt window should reject before capacity");
    };
    assert!(overlong.intent.is_none());
    assert!(remaining_ns > maximum_ns);
    assert_eq!(observed_maximum_ns, maximum_ns);
    assert_eq!(load_receipt(pic, authority_id, 53), None);

    assert_eq!(
        begin_receipt(pic, authority_id, 21, 22, 1, 1, replay_deadline_ns),
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
    replay_deadline_ns: u64,
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
        begin_receipt(pic, authority_id, 11, 12, 1, 1, replay_deadline_ns),
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
    replay_deadline_ns: u64,
) -> ReceiptIntentView {
    let rollback_pending = ReceiptIntentView {
        payload_digest: [32; 32],
        quantity: 1,
        revision: 1,
        state: ReceiptStateView::Pending,
    };
    assert_eq!(
        begin_receipt(pic, authority_id, 31, 32, 2, 1, replay_deadline_ns),
        ReceiptBeginView {
            status: ReceiptBeginStatus::Created,
            intent: Some(rollback_pending.clone()),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 41, 42, 2, 1, replay_deadline_ns),
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
    replay_deadline_ns: u64,
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
        begin_receipt(pic, authority_id, 31, 32, 2, 1, replay_deadline_ns),
        ReceiptBeginView {
            status: ReceiptBeginStatus::ExistingRolledBack,
            intent: Some(rolled_back),
        }
    );
    assert_eq!(
        begin_receipt(pic, authority_id, 41, 42, 2, 1, replay_deadline_ns),
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

fn assert_terminal_reclamation(
    pic: &ic_testkit::pic::Pic,
    authority_id: Principal,
    replay_deadline_ns: u64,
) {
    pic.advance_time(Duration::from_mins(61));
    pic.tick();
    pic.tick();

    assert_eq!(load_receipt(pic, authority_id, 11), None);
    assert_eq!(load_receipt(pic, authority_id, 31), None);
    assert!(matches!(
        load_receipt(pic, authority_id, 41),
        Some(ReceiptIntentView {
            state: ReceiptStateView::Pending,
            ..
        })
    ));
    assert_eq!(
        begin_receipt(pic, authority_id, 11, 12, 1, 1, replay_deadline_ns),
        ReceiptBeginView {
            status: ReceiptBeginStatus::ReplayWindowClosed { replay_deadline_ns },
            intent: None,
        }
    );
    assert_eq!(
        begin_receipt(
            pic,
            authority_id,
            21,
            22,
            1,
            1,
            pic.current_time_nanos() + NANOS_PER_HOUR,
        ),
        ReceiptBeginView {
            status: ReceiptBeginStatus::CapacityExceeded {
                current_quantity: 1,
                requested_quantity: 1,
                limit: 1,
            },
            intent: None,
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
    replay_deadline_ns: u64,
) -> ReceiptBeginView {
    pic.update_call::<Result<ReceiptBeginView, String>, _>(
        authority_id,
        "begin_receipt",
        (
            operation_seed,
            payload_seed,
            resource_seed,
            quantity,
            replay_deadline_ns,
        ),
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
