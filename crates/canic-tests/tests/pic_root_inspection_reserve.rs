//! Qualify outbound inspection admission through the maintained Root endpoint.
//! Only the disposable fixture's freezing threshold changes; no funding policy changes.

use candid::{CandidType, Deserialize, Nat, Principal, decode_one, encode_one};
use canic::{
    Error,
    dto::canister::{
        CanisterInspectionRequest, CanisterInspectionReserveResponse, CanisterStatusResponse,
    },
};
use canic_testing_internal::pic::{CanicWasmBuildProfile, install_audit_root_probe};
use ic_testkit::pic::{ErrorCode, PocketIc, RejectResponse};

#[derive(CandidType)]
enum RootCommand {
    InspectCanister(CanisterInspectionRequest),
}

#[derive(CandidType, Deserialize)]
enum RootResponse {
    InspectCanister(Box<CanisterStatusResponse>),
    InspectionReserveRequired(CanisterInspectionReserveResponse),
}

#[derive(CandidType)]
enum ReserveRequest {
    CycleBalance,
    InspectionReserve(CanisterInspectionRequest),
}

#[derive(CandidType, Deserialize)]
enum ReserveResponse {
    InspectionReserve(CanisterInspectionReserveResponse),
}

#[test]
fn positive_native_balance_can_fail_root_outbound_inspection_admission() {
    let fixture = install_audit_root_probe(CanicWasmBuildProfile::Fast);
    let pic = &fixture.pic;
    let root = fixture.canister_id;
    let target = pic.create_canister();
    pic.set_controllers(target, None, vec![root]).unwrap();
    set_freezing_threshold(pic, root, 0);
    let reserve = inspection_reserve(pic, root, target, Principal::anonymous()).unwrap();
    assert_eq!(reserve.caller, root);
    assert_eq!(reserve.canister_id, target);
    assert!(reserve.required_liquid_cycles > 0);
    assert!(reserve.available_liquid_cycles >= reserve.required_liquid_cycles);
    let longer = inspection_reserve(
        pic,
        root,
        Principal::from_slice(&[7; 29]),
        Principal::anonymous(),
    )
    .unwrap();
    assert!(longer.required_liquid_cycles > reserve.required_liquid_cycles);
    let fenced = pic
        .query_call(
            root,
            Principal::anonymous(),
            canic::protocol::CANIC_OBSERVABILITY,
            encode_one(ReserveRequest::CycleBalance).unwrap(),
        )
        .unwrap();
    let fenced: Result<ReserveResponse, Error> = decode_one(&fenced).unwrap();
    assert!(
        matches!(fenced, Err(error) if error.code() == canic::diagnostics::codes::LIFECYCLE_INACTIVE.raw_code())
    );
    assert!(inspection_reserve(pic, root, target, Principal::from_slice(&[99])).is_err());
    assert!(matches!(
        inspect(pic, root, target).unwrap(),
        Ok(RootResponse::InspectCanister(_))
    ));
    let denied = pic
        .update_call(
            root,
            Principal::from_slice(&[99]),
            canic::protocol::CANIC_ROOT_COMMAND,
            encode_one(RootCommand::InspectCanister(CanisterInspectionRequest {
                canister_id: target,
            }))
            .unwrap(),
        )
        .unwrap();
    let denied: Result<RootResponse, Error> = decode_one(&denied).unwrap();
    assert!(
        denied.is_err(),
        "numerical reserve evidence remains controller-only"
    );
    let (threshold, evidence) = find_inspection_failure(pic, root, target);
    assert_eq!(evidence.caller, root);
    assert_eq!(evidence.canister_id, target);
    assert!(evidence.native_cycles > 0);
    assert!(evidence.available_liquid_cycles < evidence.required_liquid_cycles);
    assert!(evidence.available_liquid_cycles <= evidence.native_cycles);
    let before = pic.cycle_balance(root);
    assert!(before > 0);
    // Query the actual near-freeze Root without spending another update's reserve.
    let preview = inspection_reserve(pic, root, target, Principal::anonymous()).unwrap();
    assert_eq!(preview.caller, root);
    assert_eq!(preview.canister_id, target);
    assert_eq!(
        preview.required_liquid_cycles,
        evidence.required_liquid_cycles
    );
    assert_eq!(pic.cycle_balance(root), before);
    assert!(preview.available_liquid_cycles <= preview.native_cycles);
    let shortfall = find_query_shortfall(pic, root, target);
    assert_eq!(shortfall.caller, root);
    assert_eq!(shortfall.canister_id, target);
    assert!(shortfall.native_cycles > 0);
    assert_eq!(
        shortfall.required_liquid_cycles,
        evidence.required_liquid_cycles
    );
    eprintln!(
        "Root inspection reserve: threshold_seconds={threshold}, native_cycles={before}, evidence={evidence:?}, query={preview:?}, query_shortfall={shortfall:?}"
    );
    set_freezing_threshold(pic, root, 0);
    let recovered = inspection_reserve(pic, root, target, Principal::anonymous()).unwrap();
    assert!(recovered.available_liquid_cycles >= recovered.required_liquid_cycles);
    assert!(matches!(
        inspect(pic, root, target).unwrap(),
        Ok(RootResponse::InspectCanister(_))
    ));
    assert!(
        pic.cycle_balance(root) <= before,
        "inspection recovers without adding cycles"
    );
}

fn find_inspection_failure(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
) -> (u64, CanisterInspectionReserveResponse) {
    let status = pic.canister_status(root, None).unwrap();
    let native = u128::try_from(status.cycles.0).unwrap();
    let daily_idle = u128::try_from(status.idle_cycles_burned_per_day.0).unwrap();
    assert!(daily_idle > 0);
    // Search the fixture's own measured reserve interval, not a production cycle constant.
    let mut low = 0_u64;
    let mut high = u64::try_from(native.checked_mul(86_400).unwrap().div_ceil(daily_idle)).unwrap();
    let mut observed = None;
    for _ in 0..u64::BITS {
        if low > high {
            break;
        }
        let threshold = low + (high - low) / 2;
        set_freezing_threshold(pic, root, threshold);
        match inspect(pic, root, target) {
            Ok(Ok(RootResponse::InspectCanister(_))) => low = threshold.checked_add(1).unwrap(),
            Ok(Ok(RootResponse::InspectionReserveRequired(evidence))) => {
                observed = Some((threshold, evidence));
                break;
            }
            Ok(Err(error)) => panic!("unexpected inspection failure: {error}"),
            Err(error) => {
                assert_eq!(error.error_code, ErrorCode::CanisterOutOfCycles);
                high = threshold.checked_sub(1).unwrap();
            }
        }
    }
    observed.expect("an admitted update reaches its outbound reserve barrier")
}

fn find_query_shortfall(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
) -> CanisterInspectionReserveResponse {
    let status = pic.canister_status(root, None).unwrap();
    let native = u128::try_from(status.cycles.0).unwrap();
    let daily_idle = u128::try_from(status.idle_cycles_burned_per_day.0).unwrap();
    let mut low = 0_u64;
    let mut high = u64::try_from(native.checked_mul(86_400).unwrap().div_ceil(daily_idle)).unwrap();
    for _ in 0..u64::BITS {
        if low > high {
            break;
        }
        let threshold = low + (high - low) / 2;
        set_freezing_threshold(pic, root, threshold);
        let before = pic.cycle_balance(root);
        let result = reserve_query(pic, root, target, Principal::anonymous());
        assert_eq!(
            pic.cycle_balance(root),
            before,
            "reserve query has no cycle transfer or replicated effect"
        );
        match result {
            Ok(Ok(evidence))
                if evidence.available_liquid_cycles < evidence.required_liquid_cycles =>
            {
                return evidence;
            }
            Ok(Ok(_)) => low = threshold.checked_add(1).unwrap(),
            Ok(Err(error)) => panic!("unexpected reserve query error: {error}"),
            Err(error) => {
                assert_eq!(error.error_code, ErrorCode::CanisterOutOfCycles);
                high = threshold.checked_sub(1).unwrap();
            }
        }
    }
    panic!("query exposes a shortfall before scheduling inspection");
}

fn inspection_reserve(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
    caller: Principal,
) -> Result<CanisterInspectionReserveResponse, Error> {
    reserve_query(pic, root, target, caller).unwrap()
}

fn reserve_query(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
    caller: Principal,
) -> Result<Result<CanisterInspectionReserveResponse, Error>, RejectResponse> {
    let bytes = pic.query_call(
        root,
        caller,
        canic::protocol::CANIC_OBSERVABILITY,
        encode_one(ReserveRequest::InspectionReserve(
            CanisterInspectionRequest {
                canister_id: target,
            },
        ))
        .unwrap(),
    )?;
    let response: Result<ReserveResponse, Error> = decode_one(&bytes).unwrap();
    Ok(response.map(|ReserveResponse::InspectionReserve(evidence)| evidence))
}

fn set_freezing_threshold(pic: &PocketIc, root: Principal, seconds: u64) {
    let settings = serde_json::from_value(serde_json::json!({
        "freezing_threshold": Nat::from(seconds),
    }))
    .unwrap();
    pic.update_canister_settings(root, None, settings).unwrap();
}

fn inspect(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
) -> Result<Result<RootResponse, Error>, RejectResponse> {
    let bytes = pic.update_call(
        root,
        Principal::anonymous(),
        canic::protocol::CANIC_ROOT_COMMAND,
        encode_one(RootCommand::InspectCanister(CanisterInspectionRequest {
            canister_id: target,
        }))
        .unwrap(),
    )?;
    Ok(decode_one(&bytes).unwrap())
}
