//! Integration: standalone waveform execution
//!
//! Responsibility: prove funding rejection, exact timer burns, and terminal abort in PocketIC.
//! Does not own: mainnet qualification, Dashboard fidelity, or production funding.
//! Boundary: all cycles are synthetic and every canister is local to the test instance.

use std::{fs, time::Duration};

use candid::{Principal, encode_one};
use ic_testkit::{
    artifacts::{WasmBuildSpec, build_wasm_canisters_cached, test_target_dir},
    pic::{CandidCallExt, PocketIc, PocketIcBuilder},
};
use pocket_ic::CreateCanisterParams;
use saltz_burner::{
    BurnerCommand, BurnerError, BurnerStatusRequest, BurnerStatusResponse, ReceiptPage,
    RejectionReason, RunPhase, TerminalReason,
};

const BILLION: u128 = 1_000_000_000;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const PACKAGE: &str = "saltz_burner";

#[test]
fn immutable_waveform_burns_exact_steps_and_abort_stops_future_burns() {
    let wasm = build_wasm();
    let pic = PocketIcBuilder::new().with_application_subnet().build();

    assert_arm_rejects_insufficient_funding(&pic, &wasm);
    assert_abort_stops_future_burns(&pic, &wasm);
    assert_trial_window_cannot_burn_a_later_step(&pic, &wasm);
}

fn assert_trial_window_cannot_burn_a_later_step(pic: &PocketIc, wasm: &[u8]) {
    let canister_id = install(pic, wasm, 2_000 * BILLION);
    let prepared = summary(pic, canister_id);
    let target_balance = prepared.required_cycles_to_arm + 100 * BILLION;
    assert!(prepared.current_balance_cycles < target_balance);
    pic.add_cycles(
        canister_id,
        target_balance - prepared.current_balance_cycles,
    );
    let chart_start_at_ns = aligned_chart_start(pic, &prepared);
    let armed = command(
        pic,
        canister_id,
        BurnerCommand::Arm {
            authorization_digest: prepared.authorization_digest,
            chart_start_at_ns,
        },
    )
    .expect("exact trial funding should arm");
    set_time_ns(
        pic,
        armed
            .schedule_start_at_ns
            .expect("armed run should expose schedule start"),
    );

    for step in 0..armed.initial_funding_step_count {
        if step > 0 {
            pic.advance_time(Duration::from_secs(armed.control_step_seconds));
        }
        pic.tick();
    }
    let funded = summary(pic, canister_id);
    assert_eq!(funded.phase, RunPhase::Running);
    assert_eq!(funded.receipt_count, armed.initial_funding_step_count);
    assert_eq!(funded.total_burned_cycles, armed.initial_funding_cycles);

    pic.advance_time(Duration::from_secs(armed.control_step_seconds));
    pic.tick();
    let stopped = summary(pic, canister_id);
    assert_eq!(stopped.phase, RunPhase::Failed);
    assert_eq!(
        stopped.terminal_reason,
        Some(TerminalReason::InsufficientBalance)
    );
    assert_eq!(stopped.receipt_count, armed.initial_funding_step_count);
    assert_eq!(stopped.total_burned_cycles, armed.initial_funding_cycles);
}

fn assert_arm_rejects_insufficient_funding(pic: &PocketIc, wasm: &[u8]) {
    let canister_id = install(pic, wasm, 2_000 * BILLION);
    let prepared = summary(pic, canister_id);
    let chart_start_at_ns = aligned_chart_start(pic, &prepared);
    let wrong_authorization = command(
        pic,
        canister_id,
        BurnerCommand::Arm {
            authorization_digest: vec![0; 32],
            chart_start_at_ns,
        },
    );
    assert!(matches!(
        wrong_authorization,
        Err(BurnerError::Rejected {
            reason: RejectionReason::Authorization
        })
    ));

    let unauthorized: Result<BurnerStatusResponse, BurnerError> = pic.query_candid_as_or_panic(
        canister_id,
        Principal::self_authenticating([0x55; 32]),
        "burner_status",
        (BurnerStatusRequest::Summary,),
    );
    assert!(matches!(unauthorized, Err(BurnerError::AccessDenied)));

    let result = command(
        pic,
        canister_id,
        BurnerCommand::Arm {
            authorization_digest: prepared.authorization_digest,
            chart_start_at_ns,
        },
    );

    match result {
        Err(BurnerError::Rejected {
            reason: RejectionReason::Funding { .. },
        }) => {}
        Err(error) => panic!("underfunded arm returned the wrong error: {error:?}"),
        Ok(_) => panic!("underfunded arm unexpectedly succeeded"),
    }
    assert_eq!(summary(pic, canister_id).phase, RunPhase::Prepared);
}

fn assert_abort_stops_future_burns(pic: &PocketIc, wasm: &[u8]) {
    let canister_id = install(pic, wasm, 430_000 * BILLION);
    let prepared = summary(pic, canister_id);
    assert!(prepared.required_cycles_to_arm < prepared.total_burn_cycles);
    assert_eq!(prepared.initial_funding_step_count, 42);
    let chart_start_at_ns = aligned_chart_start(pic, &prepared);
    let armed = command(
        pic,
        canister_id,
        BurnerCommand::Arm {
            authorization_digest: prepared.authorization_digest,
            chart_start_at_ns,
        },
    )
    .expect("abort fixture should arm");
    set_time_ns(
        pic,
        armed
            .schedule_start_at_ns
            .expect("armed run should expose schedule start"),
    );
    pic.tick();
    for _ in 1..3 {
        pic.advance_time(Duration::from_secs(armed.control_step_seconds));
        pic.tick();
    }

    let after_steps = summary(pic, canister_id);
    assert_eq!(after_steps.phase, RunPhase::Running);
    assert_eq!(after_steps.receipt_count, 3);
    let page = receipts(pic, canister_id, 0, 3);
    assert_eq!(page.receipts.len(), 3);
    assert_eq!(page.total_receipts, 3);
    assert_eq!(page.next_start, None);
    assert!(page.receipts.windows(2).all(|pair| {
        pair[1].expected_at_ns - pair[0].expected_at_ns
            == armed.control_step_seconds * NANOS_PER_SECOND
    }));
    assert_eq!(
        page.receipts
            .iter()
            .map(|receipt| receipt.burned_cycles)
            .sum::<u128>(),
        after_steps.total_burned_cycles
    );
    assert!(
        page.receipts
            .iter()
            .all(|receipt| receipt.burned_cycles == receipt.requested_cycles)
    );
    let aborted = command(pic, canister_id, BurnerCommand::Abort).expect("abort should succeed");
    assert_eq!(aborted.phase, RunPhase::Aborted);

    pic.advance_time(Duration::from_secs(10 * aborted.control_step_seconds));
    for _ in 0..3 {
        pic.tick();
    }
    let later = summary(pic, canister_id);
    assert_eq!(later.phase, RunPhase::Aborted);
    assert_eq!(later.receipt_count, 3);
    assert_eq!(later.total_burned_cycles, after_steps.total_burned_cycles);
}

fn install(pic: &PocketIc, wasm: &[u8], cycles: u128) -> Principal {
    let canister_id = pic
        .create_canister_with_params(
            None,
            CreateCanisterParams {
                cycles: Some(cycles),
                ..CreateCanisterParams::default()
            },
        )
        .expect("create exact-balance waveform canister");
    pic.install_canister(
        canister_id,
        wasm.to_vec(),
        encode_one(()).expect("init args"),
        None,
    );
    canister_id
}

fn command(
    pic: &PocketIc,
    canister_id: Principal,
    command: BurnerCommand,
) -> Result<saltz_burner::BurnerSummary, BurnerError> {
    pic.update_candid_or_panic(canister_id, "burner_command", (command,))
}

fn summary(pic: &PocketIc, canister_id: Principal) -> saltz_burner::BurnerSummary {
    let response: Result<BurnerStatusResponse, BurnerError> = pic.query_candid_or_panic(
        canister_id,
        "burner_status",
        (BurnerStatusRequest::Summary,),
    );
    match response.expect("controller summary should succeed") {
        BurnerStatusResponse::Summary(summary) => *summary,
        BurnerStatusResponse::Receipts(_) => panic!("summary request returned receipts"),
    }
}

fn receipts(pic: &PocketIc, canister_id: Principal, start: u32, limit: u16) -> ReceiptPage {
    let response: Result<BurnerStatusResponse, BurnerError> = pic.query_candid_or_panic(
        canister_id,
        "burner_status",
        (BurnerStatusRequest::Receipts { limit, start },),
    );
    match response.expect("controller receipts should succeed") {
        BurnerStatusResponse::Receipts(page) => page,
        BurnerStatusResponse::Summary(_) => panic!("receipts request returned summary"),
    }
}

fn aligned_chart_start(pic: &PocketIc, summary: &saltz_burner::BurnerSummary) -> u64 {
    let now_ns = time_ns(pic);
    let pre_roll_ns =
        u64::from(summary.pre_roll_step_count) * summary.control_step_seconds * NANOS_PER_SECOND;
    let earliest = now_ns + summary.minimum_arm_lead_ns + pre_roll_ns;
    let alignment = summary.chart_step_seconds * NANOS_PER_SECOND;
    earliest.div_ceil(alignment) * alignment
}

fn time_ns(pic: &PocketIc) -> u64 {
    pic.get_time().as_nanos_since_unix_epoch()
}

fn set_time_ns(pic: &PocketIc, timestamp_ns: u64) {
    pic.set_time((std::time::UNIX_EPOCH + Duration::from_nanos(timestamp_ns)).into());
}

fn build_wasm() -> Vec<u8> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("burner package should remain below apps/saltz")
        .to_path_buf();
    let target_dir = test_target_dir(&workspace_root, "saltz-burner-pic");
    let spec = WasmBuildSpec::new(&workspace_root, &target_dir, &[PACKAGE], "release")
        .with_cargo_profile_args(&["--locked", "--release"])
        .with_additional_inputs(&["docs/design/ideas/saltz/saltz_24h_waveform_floor_100B_860.csv"]);
    let outcome = build_wasm_canisters_cached(&spec).expect("build waveform Wasm");
    let artifact = outcome
        .record()
        .artifacts()
        .first()
        .expect("waveform build should produce one artifact");
    fs::read(artifact).expect("read waveform Wasm")
}
