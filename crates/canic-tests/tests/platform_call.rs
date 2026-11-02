//! Installed Platform-owned execution through Canic's stateless metrics/error boundary.

mod platform_call_support;

use canic::{Error, diagnostics::codes};
use canic_testing_internal::pic::install_lifecycle_boundary_fixture;
use ic_platform::{
    pocket_ic::{ErrorCode, PocketIc, common::rest::RawMessageId},
    principal::ExternalPrincipal as Principal,
};
use ic_testkit::pic::{
    WireCallSpec,
    fixtures::{RawReplyLimits, RawReplyMethod, ReplyMode, raw_reply_canister},
    query_wire, update_wire,
};
use platform_call_support::*;
use std::time::Duration;

const fn spec(method: &'static str, method_id: u64) -> WireCallSpec {
    WireCallSpec {
        method,
        method_id,
        request_limits: LIMITS,
        response_limits: LIMITS,
    }
}
fn request(
    target: Principal,
    id: u8,
    hold: bool,
    reply: TargetReply,
    after: AfterCall,
) -> CallRequest {
    CallRequest {
        target: target.into(),
        method: String::from("platform_call_target"),
        bytes: ic_wire::native::encode_request(
            TARGET,
            &TargetRequest {
                id,
                token: 100 + u64::from(id),
                accept: 100,
                hold,
                reply,
            },
            LIMITS,
        )
        .unwrap(),
        cycles: 400,
        timeout_seconds: None,
        after,
    }
}
fn execute(pic: &PocketIc, sender: Principal, input: &CallRequest) -> CallObservation {
    finish(pic, submit(pic, sender, input))
}
fn submit(pic: &PocketIc, sender: Principal, input: &CallRequest) -> RawMessageId {
    pic.submit_call(
        sender,
        Principal::anonymous(),
        "platform_call_execute",
        ic_wire::native::encode_request(EXECUTE, input, LIMITS).unwrap(),
    )
    .unwrap()
}
fn wait(
    pic: &PocketIc,
    ingress: RawMessageId,
) -> Result<Vec<u8>, ic_platform::pocket_ic::RejectResponse> {
    for _ in 0..200 {
        pic.tick();
        if let Some(reply) = pic.ingress_status(ingress.clone()) {
            return reply;
        }
    }
    panic!("controlled callback did not finish within the fixture round bound");
}
fn finish(pic: &PocketIc, ingress: RawMessageId) -> CallObservation {
    let bytes = wait(pic, ingress).unwrap();
    ic_wire::native::decode_response::<Result<CallObservation, Error>>(EXECUTE, &bytes, LIMITS)
        .unwrap()
        .unwrap()
}
fn metrics(pic: &PocketIc, sender: Principal) -> CallMetrics {
    query_wire::<_, Result<CallMetrics, Error>>(
        pic,
        sender,
        Principal::anonymous(),
        spec("platform_call_metrics", METRICS),
        &(),
    )
    .unwrap()
    .unwrap()
}
fn target(pic: &PocketIc, receiver: Principal) -> TargetObservation {
    query_wire::<_, Result<TargetObservation, Error>>(
        pic,
        receiver,
        Principal::anonymous(),
        spec("platform_call_observe", OBSERVE),
        &(),
    )
    .unwrap()
    .unwrap()
}
fn release(pic: &PocketIc, receiver: Principal, id: u8) {
    update_wire::<_, Result<(), Error>>(
        pic,
        receiver,
        Principal::anonymous(),
        spec("platform_call_release", RELEASE),
        &id,
    )
    .unwrap()
    .unwrap();
}
fn until_held(pic: &PocketIc, receiver: Principal, ids: &[usize]) {
    for _ in 0..100 {
        pic.tick();
        if ids.iter().all(|&id| target(pic, receiver).held[id]) {
            return;
        }
    }
    panic!("target did not enter the controlled real-call gate");
}
fn tick(pic: &PocketIc) {
    for _ in 0..20 {
        pic.tick();
    }
}
fn counts(pic: &PocketIc, sender: Principal, expected: (u64, u64, u64, u64)) {
    let m = metrics(pic, sender);
    assert_eq!((m.started, m.completed, m.failed, m.dropped), expected);
}

#[test]
fn inert_requests_and_liquid_refusal_do_not_issue_remote_effects() {
    let f = install_lifecycle_boundary_fixture();
    let sender = f.install_runtime_probe_canister();
    let receiver = f.install_runtime_probe_canister();
    let inert = execute(
        &f.pic,
        sender,
        &request(
            receiver,
            0,
            false,
            TargetReply::Wire,
            AfterCall::DropUnpolled,
        ),
    );
    assert!(inert.result.is_none());
    assert_eq!(inert.refunded, None);
    assert_eq!(target(&f.pic, receiver).calls[0], 0);
    counts(&f.pic, sender, (0, 0, 0, 1));
    let mut refused = request(receiver, 0, false, TargetReply::Wire, AfterCall::Raw);
    refused.cycles = u128::MAX;
    refused.timeout_seconds = Some(17);
    let refused = execute(&f.pic, sender, &refused);
    assert_eq!(
        refused.result,
        Some(Err(Error::from_registered(
            codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES
        )))
    );
    assert_eq!(refused.refunded, None);
    assert_eq!(refused.rejection_code, None);
    assert_eq!(refused.cost, u128::MAX);
    assert_eq!(target(&f.pic, receiver).calls[0], 0);
    counts(&f.pic, sender, (1, 0, 1, 2));
    assert_eq!(metrics(&f.pic, sender).bounded_started, 1);
    let ok = execute(
        &f.pic,
        sender,
        &request(receiver, 0, false, TargetReply::Wire, AfterCall::DecodeWire),
    );
    assert_eq!(ok.decoded, Some(Ok(100)));
    assert_eq!(ok.refunded, Some(300));
    assert!(ok.cost >= 400);
    counts(&f.pic, sender, (2, 1, 1, 3));
}

#[test]
fn reverse_callback_order_preserves_each_attachment_and_refund() {
    let f = install_lifecycle_boundary_fixture();
    let sender = f.install_runtime_probe_canister();
    let receiver = f.install_runtime_probe_canister();
    let first = submit(
        &f.pic,
        sender,
        &request(receiver, 0, true, TargetReply::Wire, AfterCall::DecodeWire),
    );
    let mut other = request(receiver, 1, true, TargetReply::Wire, AfterCall::DecodeWire);
    other.cycles = 700;
    other.timeout_seconds = Some(300);
    let second = submit(&f.pic, sender, &other);
    until_held(&f.pic, receiver, &[0, 1]);
    let m = metrics(&f.pic, sender);
    assert_eq!(
        (
            m.started,
            m.completed,
            m.bounded_started,
            m.unbounded_started
        ),
        (2, 0, 1, 1)
    );
    release(&f.pic, receiver, 1);
    let second = finish(&f.pic, second);
    assert_eq!(second.decoded, Some(Ok(101)));
    assert_eq!(second.refunded, Some(600));
    assert!(f.pic.ingress_status(first.clone()).is_none());
    release(&f.pic, receiver, 0);
    let first = finish(&f.pic, first);
    assert_eq!(first.decoded, Some(Ok(100)));
    assert_eq!(first.refunded, Some(300));
    counts(&f.pic, sender, (2, 2, 0, 2));
    let t = target(&f.pic, receiver);
    assert_eq!(&t.calls[..2], &[1, 1]);
    assert_eq!(&t.accepted[..2], &[100, 100]);
}

#[test]
fn malformed_replies_and_rejection_keep_committed_effects_and_metric_stages() {
    let f = install_lifecycle_boundary_fixture();
    let sender = f.install_runtime_probe_canister();
    let receiver = f.install_runtime_probe_canister();
    let malformed = execute(
        &f.pic,
        sender,
        &request(
            receiver,
            0,
            false,
            TargetReply::Malformed,
            AfterCall::DecodeWire,
        ),
    );
    assert_eq!(malformed.result, Some(Ok(vec![0, 1, 2])));
    assert_eq!(
        malformed.decoded,
        Some(Err(Error::from_registered(codes::CODEC_INVALID)))
    );
    assert_eq!(malformed.refunded, Some(300));
    counts(&f.pic, sender, (1, 1, 0, 1));
    let rejected = execute(
        &f.pic,
        sender,
        &request(receiver, 1, false, TargetReply::Reject, AfterCall::Raw),
    );
    assert_eq!(
        rejected.result,
        Some(Err(Error::from_registered(codes::PLATFORM_UNAVAILABLE)))
    );
    assert_eq!(rejected.rejection_code, Some(4));
    assert_eq!(rejected.refunded, Some(300));
    counts(&f.pic, sender, (2, 1, 1, 2));
    tick(&f.pic);
    let t = target(&f.pic, receiver);
    assert_eq!(&t.calls[..2], &[1, 1]);
    assert_eq!(&t.accepted[..2], &[100, 100]);
    let trapped = execute(
        &f.pic,
        sender,
        &request(receiver, 2, false, TargetReply::Trap, AfterCall::Raw),
    );
    assert_eq!(trapped.rejection_code, Some(5));
    assert_eq!(trapped.refunded, Some(400));
    assert_eq!(target(&f.pic, receiver).calls[2], 0);
}

#[test]
fn dropped_calls_and_trapping_callbacks_cleanup_without_repeating_remote_effects() {
    let f = install_lifecycle_boundary_fixture();
    let sender = f.install_runtime_probe_canister();
    let receiver = f.install_runtime_probe_canister();
    let dropped = execute(
        &f.pic,
        sender,
        &request(
            receiver,
            0,
            true,
            TargetReply::Wire,
            AfterCall::DropSubmitted,
        ),
    );
    assert!(dropped.result.is_none());
    until_held(&f.pic, receiver, &[0]);
    counts(&f.pic, sender, (1, 0, 0, 1));
    release(&f.pic, receiver, 0);
    tick(&f.pic);
    counts(&f.pic, sender, (1, 0, 0, 1));
    assert_eq!(target(&f.pic, receiver).calls[0], 1);
    execute(
        &f.pic,
        sender,
        &request(receiver, 1, false, TargetReply::Wire, AfterCall::Raw),
    );
    counts(&f.pic, sender, (2, 1, 0, 2));
    let ingress = submit(
        &f.pic,
        sender,
        &request(
            receiver,
            2,
            false,
            TargetReply::Wire,
            AfterCall::TrapCallback,
        ),
    );
    let error = wait(&f.pic, ingress).unwrap_err();
    assert_eq!(error.error_code, ErrorCode::CanisterCalledTrap);
    tick(&f.pic);
    counts(&f.pic, sender, (3, 1, 0, 3));
    let t = target(&f.pic, receiver);
    assert_eq!(t.calls[2], 1);
    assert_eq!(t.accepted[2], 100);
    execute(
        &f.pic,
        sender,
        &request(receiver, 3, false, TargetReply::Wire, AfterCall::Raw),
    );
    counts(&f.pic, sender, (4, 2, 0, 4));
}

#[test]
fn bounded_unknown_reply_preserves_effect_and_does_not_resubmit() {
    let f = install_lifecycle_boundary_fixture();
    let sender = f.install_runtime_probe_canister();
    let receiver = f.install_runtime_probe_canister();
    let mut input = request(receiver, 0, true, TargetReply::Wire, AfterCall::Raw);
    input.timeout_seconds = Some(1);
    let ingress = submit(&f.pic, sender, &input);
    until_held(&f.pic, receiver, &[0]);
    let mut reply = None;
    for _ in 0..100 {
        f.pic.advance_time(Duration::from_secs(1));
        f.pic.tick();
        if let Some(outcome) = f.pic.ingress_status(ingress.clone()) {
            reply = Some(outcome);
            break;
        }
    }
    let bytes = reply
        .expect("bounded call must return without releasing the receiver")
        .unwrap();
    let observed =
        ic_wire::native::decode_response::<Result<CallObservation, Error>>(EXECUTE, &bytes, LIMITS)
            .unwrap()
            .unwrap();
    assert_eq!(
        observed.result,
        Some(Err(Error::from_registered(codes::PLATFORM_UNAVAILABLE)))
    );
    assert_eq!(observed.rejection_code, Some(6));
    assert_eq!(observed.refunded, Some(0));
    counts(&f.pic, sender, (1, 0, 1, 1));
    assert_eq!(target(&f.pic, receiver).calls[0], 1);
    assert_eq!(target(&f.pic, receiver).accepted[0], 100);
    release(&f.pic, receiver, 0);
    tick(&f.pic);
    assert_eq!(target(&f.pic, receiver).calls[0], 1);
    counts(&f.pic, sender, (1, 0, 1, 1));
    execute(
        &f.pic,
        sender,
        &request(receiver, 1, false, TargetReply::Wire, AfterCall::Raw),
    );
    counts(&f.pic, sender, (2, 1, 1, 2));
}

fn install_reply(pic: &PocketIc, name: &str, reply: &[u8]) -> Principal {
    let id = pic.create_canister();
    pic.add_cycles(id, 1_000_000_000_000);
    let wasm = raw_reply_canister(
        &[RawReplyMethod {
            name,
            mode: ReplyMode::Update,
            reply,
        }],
        RawReplyLimits {
            methods: 1,
            reply_bytes: 8192,
            method_name_bytes: 128,
        },
    )
    .unwrap();
    pic.install_canister(id, wasm, vec![], None);
    id
}

#[test]
fn external_management_and_ledger_codecs_and_native_cascade_use_platform_execution() {
    let f = install_lifecycle_boundary_fixture();
    let sender = f.install_runtime_probe_canister();
    let receiver = f.install_runtime_probe_canister();
    f.pic
        .set_controllers(receiver, None, vec![sender, Principal::anonymous()])
        .unwrap();
    let fee = ic_platform::candid::contract::encode_args((
        ic_platform::candid::contract::Nat::from(42_u64),
    ))
    .unwrap();
    let ledger = install_reply(&f.pic, "icrc1_fee", &fee);
    let reply: Result<(bool, u64), Error> = update_wire(
        &f.pic,
        sender,
        Principal::anonymous(),
        spec("platform_call_external", EXTERNAL),
        &ExternalRequest {
            target: receiver.into(),
            ledger: ledger.into(),
        },
    )
    .unwrap();
    assert_eq!(reply, Ok((true, 42)));
    let malformed = install_reply(&f.pic, "icrc1_fee", b"bad");
    let reply: Result<(bool, u64), Error> = update_wire(
        &f.pic,
        sender,
        Principal::anonymous(),
        spec("platform_call_external", EXTERNAL),
        &ExternalRequest {
            target: receiver.into(),
            ledger: malformed.into(),
        },
    )
    .unwrap();
    assert_eq!(reply, Err(Error::from_registered(codes::CODEC_INVALID)));
    let protocol = canic::protocol::cascade::topology::METHOD_ID;
    let bytes = ic_wire::native::encode_response(
        protocol,
        &Ok::<(), Error>(()),
        canic::protocol::cascade::RESPONSE_LIMITS,
    )
    .unwrap();
    let target = install_reply(&f.pic, canic::protocol::cascade::topology::METHOD, &bytes);
    let good: Result<(), Error> = update_wire(
        &f.pic,
        sender,
        Principal::anonymous(),
        spec("platform_call_cascade", CASCADE),
        &ic_platform::Principal::from(target),
    )
    .unwrap();
    assert_eq!(good, Ok(()));
    let malformed = install_reply(&f.pic, canic::protocol::cascade::topology::METHOD, b"bad");
    let bad: Result<(), Error> = update_wire(
        &f.pic,
        sender,
        Principal::anonymous(),
        spec("platform_call_cascade", CASCADE),
        &ic_platform::Principal::from(malformed),
    )
    .unwrap();
    assert_eq!(bad, Err(Error::from_registered(codes::CODEC_INVALID)));
    let m = metrics(&f.pic, sender);
    assert_eq!((m.started, m.completed, m.failed), (2, 2, 0));
}
