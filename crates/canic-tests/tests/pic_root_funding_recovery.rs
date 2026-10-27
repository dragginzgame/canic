// Category C - Artifact test (built Wasm; no production funding surface).

use canic_testing_internal::pic::{
    CanicWasmBuildProfile, InternalTestWasms, build_internal_test_wasm_canisters, start_pocket_ic,
};
use ic_platform::{
    candid::contract::{CandidType, encode_one},
    principal::ExternalPrincipal as Principal,
};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};
use {
    ic_platform::pocket_ic::{PocketIc, PocketIcBuilder},
    ic_testkit::pic::InstallSpec,
    ic_testkit::pic::prelude::*,
};

const CANISTERS: [&str; 1] = ["root_funding_probe"];
const COORDINATOR_INSTALL_CYCLES: u128 = 50_000_000_000_000;
const FOREIGN_INSTALL_CYCLES: u128 = 10_000_000_000_000;
const ROOT_INSTALL_CYCLES: u128 = 5_000_000_000_000;
const GRANT_CYCLES: u128 = 1_000_000_000_000;
const ROOT_REQUEST_RECOVERY_FLOOR_CYCLES: u128 = 42_200_000_000;
const ROOT_ICP_REFILL_RECOVERY_FLOOR_CYCLES: u128 = 42_200_000_000;
const OPERATION_ID: [u8; 32] = [0x17; 32];
static BUILD_ONCE: OnceLock<InternalTestWasms> = OnceLock::new();

struct BalanceObservation {
    coordinator_before_fresh: u128,
    coordinator_after_fresh: u128,
    coordinator_after_replay: u128,
    root_before_fresh: u128,
    root_after_fresh: u128,
    root_after_replay: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
enum ProbeInit {
    Coordinator,
    Root { coordinator: Principal },
    EmergencyRoot { ledger: Principal, cmc: Principal },
    Ledger,
    Cmc,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
struct GrantRequest {
    operation_id: [u8; 32],
    amount: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
struct GrantReceipt {
    operation_id: [u8; 32],
    coordinator: ic_platform::Principal,
    root: ic_platform::Principal,
    amount: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
struct AcceptanceObservation {
    balance_before_accept: u128,
    balance_after_accept: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
struct RootRequestObservation {
    result: IssueResult,
    call_cost: u128,
    balance_before_request: u128,
    balance_after_request: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
struct EmergencyTransferRequest {
    operation_id: [u8; 32],
    from_subaccount: Option<[u8; 32]>,
    to_owner: ic_platform::Principal,
    to_subaccount: [u8; 32],
    amount_e8s: u64,
    fee_e8s: u64,
    memo: Vec<u8>,
    created_at_time_ns: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
enum EmergencyRefillPhase {
    Prepared,
    Transferred { block_index: u64 },
    Completed { block_index: u64, cycles_sent: u128 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
struct EmergencyRefillOperation {
    transfer: EmergencyTransferRequest,
    max_call_cost: u128,
    phase: EmergencyRefillPhase,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
enum IntentState {
    Prepared,
    Committed { receipt: GrantReceipt },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
struct GrantIntent {
    root: ic_platform::Principal,
    request: GrantRequest,
    call_cost: u128,
    call_reservation: u128,
    state: IntentState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct RetainedOperation {
    operation_id: [u8; 32],
    sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RetainedRootGrantSlot {
    current: Option<RetainedOperation>,
    last_terminal: Option<RetainedOperation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetainedSequenceDecision {
    CompetingCurrent,
    ExactCurrentRetry,
    ExactTerminalReplay,
    PrepareSuccessor,
    SequenceSkipped,
    SequenceStale,
}

impl RetainedRootGrantSlot {
    fn admit(&mut self, requested: RetainedOperation) -> RetainedSequenceDecision {
        if let Some(current) = self.current.as_ref() {
            return if current == &requested {
                RetainedSequenceDecision::ExactCurrentRetry
            } else {
                RetainedSequenceDecision::CompetingCurrent
            };
        }
        if let Some(last_terminal) = self.last_terminal.as_ref() {
            if last_terminal == &requested {
                return RetainedSequenceDecision::ExactTerminalReplay;
            }
            if requested.sequence <= last_terminal.sequence {
                return RetainedSequenceDecision::SequenceStale;
            }
            if requested.sequence != last_terminal.sequence.saturating_add(1) {
                return RetainedSequenceDecision::SequenceSkipped;
            }
        } else if requested.sequence != 1 {
            return RetainedSequenceDecision::SequenceSkipped;
        }

        self.current = Some(requested);
        RetainedSequenceDecision::PrepareSuccessor
    }

    const fn complete_current(&mut self) {
        self.last_terminal = self.current.take();
    }
}

#[test]
fn one_current_and_last_result_reject_old_skipped_and_competing_sequences() {
    let first = retained_operation(1, 0x11);
    let mut slot = RetainedRootGrantSlot {
        current: None,
        last_terminal: None,
    };

    assert_eq!(
        slot.admit(first.clone()),
        RetainedSequenceDecision::PrepareSuccessor
    );
    assert_eq!(
        slot.admit(first.clone()),
        RetainedSequenceDecision::ExactCurrentRetry
    );
    assert_eq!(
        slot.admit(retained_operation(1, 0x12)),
        RetainedSequenceDecision::CompetingCurrent
    );
    slot.complete_current();
    assert_eq!(
        slot.admit(first.clone()),
        RetainedSequenceDecision::ExactTerminalReplay
    );
    assert_eq!(
        slot.admit(retained_operation(3, 0x33)),
        RetainedSequenceDecision::SequenceSkipped
    );

    let second = retained_operation(2, 0x22);
    assert_eq!(
        slot.admit(second.clone()),
        RetainedSequenceDecision::PrepareSuccessor
    );
    slot.complete_current();
    assert_eq!(slot.last_terminal, Some(second));
    assert_eq!(slot.admit(first), RetainedSequenceDecision::SequenceStale);
}

const fn retained_operation(sequence: u64, byte: u8) -> RetainedOperation {
    RetainedOperation {
        operation_id: [byte; 32],
        sequence,
    }
}

#[test]
fn root_request_and_retry_measure_the_normal_threshold_floor() {
    let workspace_root = workspace_root();
    let wasms = build_canisters(&workspace_root);
    let wasm = wasms.wasm("root_funding_probe");
    let pic = start_pocket_ic(PocketIcBuilder::new().with_application_subnet());
    let coordinator = install_probe(
        &pic,
        wasm.clone(),
        ProbeInit::Coordinator,
        COORDINATOR_INSTALL_CYCLES,
    );
    let root = install_probe(
        &pic,
        wasm,
        ProbeInit::Root { coordinator },
        ROOT_INSTALL_CYCLES,
    );

    let fresh = request_from_root(&pic, root);
    assert!(matches!(
        fresh.result,
        IssueResult::Committed {
            accepted_cycles: GRANT_CYCLES,
            replay: false,
            ..
        }
    ));
    let fresh_execution = fresh
        .balance_before_request
        .checked_add(GRANT_CYCLES)
        .and_then(|balance| balance.checked_sub(fresh.balance_after_request))
        .expect("fresh Root request execution");

    let retry = request_from_root(&pic, root);
    assert!(matches!(retry.result, IssueResult::AlreadyCommitted { .. }));
    let retry_execution = retry
        .balance_before_request
        .checked_sub(retry.balance_after_request)
        .expect("retry Root request execution");
    assert_eq!(fresh.call_cost, retry.call_cost);
    let measured_floor = round_up_to_100m(
        fresh
            .call_cost
            .checked_add(fresh_execution.max(retry_execution))
            .expect("Root request/retry floor"),
    );
    assert_eq!(measured_floor, ROOT_REQUEST_RECOVERY_FLOOR_CYCLES);
    println!(
        "root_funding_request_probe: call_cost={} fresh_root_execution={} retry_root_execution={} measured_floor={}",
        fresh.call_cost, fresh_execution, retry_execution, measured_floor
    );
}

#[test]
fn emergency_refill_recovery_floor_covers_transfer_and_notify_response_loss() {
    let workspace_root = workspace_root();
    let wasms = build_canisters(&workspace_root);
    let wasm = wasms.wasm("root_funding_probe");
    let pic = start_pocket_ic(PocketIcBuilder::new().with_application_subnet());
    let ledger = install_probe(
        &pic,
        wasm.clone(),
        ProbeInit::Ledger,
        FOREIGN_INSTALL_CYCLES,
    );
    let cmc = install_probe(&pic, wasm.clone(), ProbeInit::Cmc, FOREIGN_INSTALL_CYCLES);
    let root = install_probe(
        &pic,
        wasm,
        ProbeInit::EmergencyRoot { ledger, cmc },
        ROOT_INSTALL_CYCLES,
    );
    let balance_before = pic.cycle_balance(root);

    let transfer_loss: Result<EmergencyRefillOperation, _> = ic_testkit::pic::update_wire(
        &pic,
        root,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec(
            "root_funding_probe_emergency_refill_transfer_then_trap",
            0x5445_5354_0000_0d04,
        ),
        &(),
    );
    assert!(transfer_loss.is_err());
    let prepared = emergency_operation(&pic, root).expect("prepared emergency operation");
    assert_eq!(prepared.phase, EmergencyRefillPhase::Prepared);

    let notify_loss: Result<EmergencyRefillOperation, _> = ic_testkit::pic::update_wire(
        &pic,
        root,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec(
            "root_funding_probe_emergency_refill_notify_then_trap",
            0x5445_5354_0000_0d05,
        ),
        &(),
    );
    assert!(notify_loss.is_err());
    let transferred = emergency_operation(&pic, root).expect("transferred emergency operation");
    assert_eq!(
        transferred.phase,
        EmergencyRefillPhase::Transferred { block_index: 77 }
    );

    let completed: EmergencyRefillOperation = ic_testkit::pic::update_wire(
        &pic,
        root,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec("root_funding_probe_emergency_refill", 0x5445_5354_0000_0d03),
        &(),
    )
    .expect("update Wire harness call");
    assert_eq!(
        completed.phase,
        EmergencyRefillPhase::Completed {
            block_index: 77,
            cycles_sent: 2_000_000_000_000,
        }
    );
    assert_eq!(completed.max_call_cost, prepared.max_call_cost);
    let recovery_execution = balance_before
        .checked_sub(pic.cycle_balance(root))
        .expect("emergency refill recovery execution");
    let measured_floor = round_up_to_100m(
        completed
            .max_call_cost
            .checked_add(recovery_execution)
            .expect("emergency refill floor"),
    );
    assert!(measured_floor > completed.max_call_cost);
    assert_eq!(measured_floor, ROOT_ICP_REFILL_RECOVERY_FLOOR_CYCLES);
    println!(
        "root_emergency_refill_probe: max_call_cost={} recovery_execution={} measured_floor={}",
        completed.max_call_cost, recovery_execution, measured_floor
    );
}

fn round_up_to_100m(value: u128) -> u128 {
    const QUANTUM: u128 = 100_000_000;
    value
        .checked_add(QUANTUM - 1)
        .and_then(|sum| sum.checked_div(QUANTUM))
        .and_then(|units| units.checked_mul(QUANTUM))
        .expect("round emergency floor")
}

#[test]
fn attached_cycles_recover_across_intent_call_and_receipt_boundaries() {
    let workspace_root = workspace_root();
    let wasms = build_canisters(&workspace_root);
    let wasm = wasms.wasm("root_funding_probe");
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

    prove_accept_then_trap_rolls_back(&pic, coordinator, root, &prepared);

    let coordinator_before_fresh = pic.cycle_balance(coordinator);
    let root_before_fresh = pic.cycle_balance(root);
    let trapped: Result<IssueResult, _> = ic_testkit::pic::update_wire(
        &pic,
        coordinator,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec("root_funding_probe_issue_then_trap", 0x5445_5354_0000_0d09),
        &(),
    );
    assert!(trapped.is_err(), "receipt-boundary callback must trap");

    let expected_receipt = GrantReceipt {
        operation_id: OPERATION_ID,
        coordinator: coordinator.into(),
        root: root.into(),
        amount: GRANT_CYCLES,
    };
    assert_eq!(receipt(&pic, root), Some(expected_receipt.clone()));
    prove_attached_cycles_are_excluded_before_acceptance(&pic, root);
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

fn prove_accept_then_trap_rolls_back(
    pic: &PocketIc,
    coordinator: Principal,
    root: Principal,
    prepared: &GrantIntent,
) {
    let coordinator_before_root_trap = pic.cycle_balance(coordinator);
    let root_before_root_trap = pic.cycle_balance(root);
    assert_eq!(
        issue_root_accept_then_trap(pic, coordinator),
        IssueResult::CallFailed
    );
    let coordinator_after_root_trap = pic.cycle_balance(coordinator);
    let root_after_root_trap = pic.cycle_balance(root);
    assert_eq!(receipt(pic, root), None);
    assert_eq!(acceptance_observation(pic, root), None);
    assert!(
        root_after_root_trap <= root_before_root_trap,
        "a trapping acceptance message must not commit the attached principal"
    );
    assert!(
        coordinator_before_root_trap
            .checked_sub(coordinator_after_root_trap)
            .expect("trapping call Coordinator spend")
            < GRANT_CYCLES,
        "a trapping acceptance message must refund the attached principal"
    );
    assert_eq!(intent(pic, coordinator), Some(prepared.clone()));
}

fn prove_attached_cycles_are_excluded_before_acceptance(pic: &PocketIc, root: Principal) {
    let fresh_acceptance = acceptance_observation(pic, root)
        .expect("fresh acceptance must retain its balance observation");
    assert_eq!(
        fresh_acceptance
            .balance_after_accept
            .checked_sub(fresh_acceptance.balance_before_accept),
        Some(GRANT_CYCLES),
        "canister_cycle_balance must exclude attached cycles until acceptance"
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
    pic.try_create_and_install(InstallSpec {
        wasm,
        init_bytes: encode_one(init).expect("encode probe init"),
        cycles,
        install_sender: None,
        label: Some("root_funding_probe".into()),
    })
    .expect("install test canister")
}

fn prepare(pic: &PocketIc, coordinator: Principal, root: Principal) -> GrantIntent {
    ic_testkit::pic::update_wire(
        pic,
        coordinator,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec("root_funding_probe_prepare", 0x5445_5354_0000_0d00),
        &PrepareRequest {
            root: root.into(),
            operation_id: OPERATION_ID,
            amount: GRANT_CYCLES,
        },
    )
    .expect("update Wire harness call")
}

fn issue(pic: &PocketIc, coordinator: Principal) -> IssueResult {
    ic_testkit::pic::update_wire(
        pic,
        coordinator,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec("root_funding_probe_issue", 0x5445_5354_0000_0d07),
        &(),
    )
    .expect("update Wire harness call")
}

fn issue_root_accept_then_trap(pic: &PocketIc, coordinator: Principal) -> IssueResult {
    ic_testkit::pic::update_wire(
        pic,
        coordinator,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec(
            "root_funding_probe_issue_root_accept_then_trap",
            0x5445_5354_0000_0d08,
        ),
        &(),
    )
    .expect("update Wire harness call")
}

fn request_from_root(pic: &PocketIc, root: Principal) -> RootRequestObservation {
    ic_testkit::pic::update_wire(
        pic,
        root,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec("root_funding_probe_request", 0x5445_5354_0000_0d02),
        &(),
    )
    .expect("update Wire harness call")
}

fn intent(pic: &PocketIc, coordinator: Principal) -> Option<GrantIntent> {
    ic_testkit::pic::query_wire(
        pic,
        coordinator,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec("root_funding_probe_intent", 0x5445_5354_0000_0d0a),
        &(),
    )
    .expect("query Wire harness call")
}

fn receipt(pic: &PocketIc, root: Principal) -> Option<GrantReceipt> {
    ic_testkit::pic::query_wire(
        pic,
        root,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec("root_funding_probe_receipt", 0x5445_5354_0000_0d0b),
        &(),
    )
    .expect("query Wire harness call")
}

fn acceptance_observation(pic: &PocketIc, root: Principal) -> Option<AcceptanceObservation> {
    ic_testkit::pic::query_wire(
        pic,
        root,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec(
            "root_funding_probe_acceptance_observation",
            0x5445_5354_0000_0d0c,
        ),
        &(),
    )
    .expect("query Wire harness call")
}

fn emergency_operation(pic: &PocketIc, root: Principal) -> Option<EmergencyRefillOperation> {
    ic_testkit::pic::query_wire(
        pic,
        root,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        funding_spec(
            "root_funding_probe_emergency_operation",
            0x5445_5354_0000_0d06,
        ),
        &(),
    )
    .expect("query Wire harness call")
}

fn build_canisters(workspace_root: &Path) -> &'static InternalTestWasms {
    BUILD_ONCE.get_or_init(|| {
        let target_dir = workspace_root.join("target").join("pic-runtime-wasm");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTERS,
            CanicWasmBuildProfile::Fast,
        )
    })
}

fn workspace_root() -> PathBuf {
    canic_testing_internal::workspace::discover()
        .expect("discover Canic test workspace through Cargo")
}

/// Exact native preparation authority, independent of IC installation encoding.
#[derive(serde::Serialize)]
struct PrepareRequest {
    root: ic_platform::Principal,
    operation_id: [u8; 32],
    amount: u128,
}

const fn funding_spec(method: &'static str, method_id: u64) -> ic_testkit::pic::WireCallSpec {
    let limits = ic_wire::Limits {
        bytes: 16_384,
        allocation: 65_536,
        nodes: 4096,
        depth: 32,
    };
    ic_testkit::pic::WireCallSpec {
        method,
        method_id,
        request_limits: limits,
        response_limits: limits,
    }
}
