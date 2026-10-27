//! Qualify outbound inspection admission through the maintained Root endpoint.
//! Only the disposable fixture's freezing threshold changes; no funding policy changes.

use canic::{
    Error,
    dto::canister::{CanisterInspectionOutcome as RootResponse, CanisterInspectionReserveResponse},
};
use canic_testing_internal::pic::{CanicWasmBuildProfile, install_audit_root_probe};
use ic_platform::pocket_ic::{ErrorCode, PocketIc, RejectResponse};
use ic_platform::{candid::contract::Nat, principal::ExternalPrincipal as Principal};

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
    let fenced: Result<canic::dto::role::CycleBalanceStatusResponse, Error> =
        ic_testkit::pic::query_wire(
            pic,
            root,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::cycle_balance::METHOD,
                method_id: canic::protocol::cycle_balance::METHOD_ID,
                request_limits: canic::protocol::cycle_balance::REQUEST_LIMITS,
                response_limits: canic::protocol::cycle_balance::RESPONSE_LIMITS,
            },
            &(),
        )
        .unwrap();
    assert!(
        matches!(fenced, Err(error) if error.code() == canic::diagnostics::codes::LIFECYCLE_INACTIVE.raw_code())
    );
    assert!(inspection_reserve(pic, root, target, Principal::from_slice(&[99])).is_err());
    assert!(matches!(
        inspect(pic, root, target).unwrap(),
        Ok(RootResponse::Status(_))
    ));
    let denied = pic
        .update_call(
            root,
            Principal::from_slice(&[99]),
            canic::protocol::inspection::METHOD,
            ic_wire::native::encode_request(
                canic::protocol::inspection::METHOD_ID,
                &canic::dto::inspection::Request {
                    canister_id: target.into(),
                },
                canic::protocol::inspection::REQUEST_LIMITS,
            )
            .unwrap(),
        )
        .unwrap();
    let denied: Result<canic::dto::inspection::Response, Error> = ic_wire::native::decode_response(
        canic::protocol::inspection::METHOD_ID,
        &denied,
        canic::protocol::inspection::RESPONSE_LIMITS,
    )
    .unwrap();
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
        Ok(RootResponse::Status(_))
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
            Ok(Ok(RootResponse::Status(_))) => low = threshold.checked_add(1).unwrap(),
            Ok(Ok(RootResponse::ReserveRequired(evidence))) => {
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
    use canic::protocol::inspection_reserve as protocol;
    let input: ic_platform::Principal = target.into();
    let bytes = pic.query_call(
        root,
        caller,
        protocol::METHOD,
        ic_wire::native::encode_request(protocol::METHOD_ID, &input, protocol::REQUEST_LIMITS)
            .unwrap(),
    )?;
    let response: Result<canic::dto::inspection_reserve::InspectionReserve, Error> =
        ic_wire::native::decode_response(protocol::METHOD_ID, &bytes, protocol::RESPONSE_LIMITS)
            .unwrap();
    Ok(response.map(Into::into))
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
        canic::protocol::inspection::METHOD,
        ic_wire::native::encode_request(
            canic::protocol::inspection::METHOD_ID,
            &canic::dto::inspection::Request {
                canister_id: target.into(),
            },
            canic::protocol::inspection::REQUEST_LIMITS,
        )
        .unwrap(),
    )?;
    let response: Result<canic::dto::inspection::Response, Error> =
        ic_wire::native::decode_response(
            canic::protocol::inspection::METHOD_ID,
            &bytes,
            canic::protocol::inspection::RESPONSE_LIMITS,
        )
        .unwrap();
    Ok(response.map(|value| value.try_into().unwrap()))
}
