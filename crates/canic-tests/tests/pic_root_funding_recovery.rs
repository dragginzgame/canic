// Category C - Artifact test (built Wasm; no production funding surface).

use candid::{CandidType, Deserialize, Principal, encode_one};
use canic_testing_internal::pic::{
    CanicWasmBuildProfile, build_internal_test_wasm_canisters, start_pocket_ic,
};
use ic_testkit::{
    artifacts::{read_wasm, test_target_dir, workspace_root_for},
    pic::{CandidCallExt, InstallSpec, PocketIc, PocketIcBuilder, prelude::*},
};
use std::{
    path::{Path, PathBuf},
    sync::Once,
};

const CANISTERS: [&str; 1] = ["root_funding_probe"];
const COORDINATOR_INSTALL_CYCLES: u128 = 50_000_000_000_000;
const FOREIGN_INSTALL_CYCLES: u128 = 10_000_000_000_000;
const ROOT_INSTALL_CYCLES: u128 = 5_000_000_000_000;
const GRANT_CYCLES: u128 = 1_000_000_000_000;
const OPERATION_ID: [u8; 32] = [0x17; 32];
static BUILD_ONCE: Once = Once::new();

struct BalanceObservation {
    coordinator_before_fresh: u128,
    coordinator_after_fresh: u128,
    coordinator_after_replay: u128,
    root_before_fresh: u128,
    root_after_fresh: u128,
    root_after_replay: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum ProbeInit {
    Coordinator,
    Root { coordinator: Principal },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct GrantRequest {
    operation_id: [u8; 32],
    amount: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct GrantReceipt {
    operation_id: [u8; 32],
    coordinator: Principal,
    root: Principal,
    amount: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum IntentState {
    Prepared,
    Committed { receipt: GrantReceipt },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct GrantIntent {
    root: Principal,
    request: GrantRequest,
    call_cost: u128,
    call_reservation: u128,
    state: IntentState,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum IssueResult {
    Committed {
        receipt: GrantReceipt,
        accepted_cycles: u128,
        replay: bool,
    },
    AlreadyCommitted {
        receipt: GrantReceipt,
    },
    MissingIntent,
    CallFailed,
    RootDenied,
    RootBindingConflict,
}

#[test]
fn attached_cycles_recover_across_intent_call_and_receipt_boundaries() {
    let workspace_root = workspace_root();
    build_canisters(&workspace_root);
    let target_dir = test_target_dir(&workspace_root, "pic-runtime-wasm");
    let wasm = read_wasm(
        &target_dir,
        "root_funding_probe",
        CanicWasmBuildProfile::Fast.target_dir_name(),
    );
    let pic = start_pocket_ic(PocketIcBuilder::new().with_application_subnet());

    let coordinator = install_probe(
        &pic,
        wasm.clone(),
        ProbeInit::Coordinator,
        COORDINATOR_INSTALL_CYCLES,
    );
    let foreign_coordinator = install_probe(
        &pic,
        wasm.clone(),
        ProbeInit::Coordinator,
        FOREIGN_INSTALL_CYCLES,
    );
    let root = install_probe(
        &pic,
        wasm,
        ProbeInit::Root { coordinator },
        ROOT_INSTALL_CYCLES,
    );

    let prepared = prepare(&pic, coordinator, root);
    assert_eq!(prepared.request.operation_id, OPERATION_ID);
    assert_eq!(prepared.request.amount, GRANT_CYCLES);
    assert_eq!(prepared.call_reservation, GRANT_CYCLES + prepared.call_cost);
    assert!(prepared.call_cost > 0);

    pic.stop_canister(coordinator, None)
        .expect("stop Coordinator at durable-intent boundary");
    pic.start_canister(coordinator, None)
        .expect("restart Coordinator after durable-intent boundary");
    assert_eq!(intent(&pic, coordinator), Some(prepared.clone()));

    let foreign_prepared = prepare(&pic, foreign_coordinator, root);
    assert_eq!(foreign_prepared.request, prepared.request);
    assert_eq!(issue(&pic, foreign_coordinator), IssueResult::RootDenied);
    assert_eq!(receipt(&pic, root), None);

    pic.stop_canister(root, None)
        .expect("stop root at outbound-call boundary");
    assert_eq!(issue(&pic, coordinator), IssueResult::CallFailed);
    assert_eq!(intent(&pic, coordinator), Some(prepared.clone()));
    pic.start_canister(root, None)
        .expect("restart root after outbound-call boundary");

    let coordinator_before_fresh = pic.cycle_balance(coordinator);
    let root_before_fresh = pic.cycle_balance(root);
    let trapped: Result<IssueResult, _> =
        pic.update_candid(coordinator, "root_funding_probe_issue_then_trap", ());
    assert!(trapped.is_err(), "receipt-boundary callback must trap");

    let expected_receipt = GrantReceipt {
        operation_id: OPERATION_ID,
        coordinator,
        root,
        amount: GRANT_CYCLES,
    };
    assert_eq!(receipt(&pic, root), Some(expected_receipt.clone()));
    assert_eq!(intent(&pic, coordinator), Some(prepared.clone()));

    let coordinator_after_fresh = pic.cycle_balance(coordinator);
    let root_after_fresh = pic.cycle_balance(root);
    let replay = issue(&pic, coordinator);
    assert_eq!(
        replay,
        IssueResult::Committed {
            receipt: expected_receipt.clone(),
            accepted_cycles: 0,
            replay: true,
        }
    );
    assert_eq!(receipt(&pic, root), Some(expected_receipt.clone()));
    assert_eq!(
        intent(&pic, coordinator),
        Some(GrantIntent {
            state: IntentState::Committed {
                receipt: expected_receipt,
            },
            ..prepared.clone()
        })
    );

    assert_and_report_headroom(
        &prepared,
        BalanceObservation {
            coordinator_before_fresh,
            coordinator_after_fresh,
            coordinator_after_replay: pic.cycle_balance(coordinator),
            root_before_fresh,
            root_after_fresh,
            root_after_replay: pic.cycle_balance(root),
        },
    );
}

fn assert_and_report_headroom(prepared: &GrantIntent, balance: BalanceObservation) {
    let fresh_coordinator_spend = balance
        .coordinator_before_fresh
        .checked_sub(balance.coordinator_after_fresh)
        .expect("fresh Coordinator spend");
    let fresh_coordinator_headroom = fresh_coordinator_spend
        .checked_sub(GRANT_CYCLES)
        .expect("fresh grant principal included in Coordinator spend");
    let fresh_root_gain = balance
        .root_after_fresh
        .checked_sub(balance.root_before_fresh)
        .expect("fresh root balance gain");
    let fresh_root_headroom = GRANT_CYCLES
        .checked_sub(fresh_root_gain)
        .expect("root gain cannot exceed accepted grant");
    let replay_coordinator_headroom = balance
        .coordinator_after_fresh
        .checked_sub(balance.coordinator_after_replay)
        .expect("replay Coordinator spend");
    let replay_root_headroom = balance
        .root_after_fresh
        .checked_sub(balance.root_after_replay)
        .expect("replay root spend");

    assert!(
        replay_coordinator_headroom < GRANT_CYCLES,
        "unaccepted replay principal must return automatically"
    );
    assert!(
        balance.root_after_replay <= balance.root_after_fresh,
        "zero-accept replay must not increase root balance"
    );
    println!(
        "root_funding_probe: call_cost={} call_reservation={} fresh_coordinator_headroom={} replay_coordinator_headroom={} fresh_root_headroom={} replay_root_headroom={}",
        prepared.call_cost,
        prepared.call_reservation,
        fresh_coordinator_headroom,
        replay_coordinator_headroom,
        fresh_root_headroom,
        replay_root_headroom,
    );
}

fn install_probe(pic: &PocketIc, wasm: Vec<u8>, init: ProbeInit, cycles: u128) -> Principal {
    pic.create_and_install(
        InstallSpec::new(wasm, encode_one(init).expect("encode probe init"), cycles)
            .label("root_funding_probe"),
    )
}

fn prepare(pic: &PocketIc, coordinator: Principal, root: Principal) -> GrantIntent {
    pic.update_candid_or_panic(
        coordinator,
        "root_funding_probe_prepare",
        (root, OPERATION_ID, GRANT_CYCLES),
    )
}

fn issue(pic: &PocketIc, coordinator: Principal) -> IssueResult {
    pic.update_candid_or_panic(coordinator, "root_funding_probe_issue", ())
}

fn intent(pic: &PocketIc, coordinator: Principal) -> Option<GrantIntent> {
    pic.query_candid_or_panic(coordinator, "root_funding_probe_intent", ())
}

fn receipt(pic: &PocketIc, root: Principal) -> Option<GrantReceipt> {
    pic.query_candid_or_panic(root, "root_funding_probe_receipt", ())
}

fn build_canisters(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-runtime-wasm");
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
