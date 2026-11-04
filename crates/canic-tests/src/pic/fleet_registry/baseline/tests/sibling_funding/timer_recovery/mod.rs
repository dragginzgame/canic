//! Qualify native callback recovery against actual caller-owned funding and held IC replies.

use super::*;
use crate::pic::application_probe::{self, CallbackFault, CallbackProbe};
use canic::dto::runtime::{CanisterTimerStatus, TimerExecutionOutcome, TimerRegistrationStatus};
use ic_testkit::pocket_ic::common::rest::RawMessageId;

#[test]
pub(in super::super) fn native_topup_recovery_preserves_callbacks_and_live_paid_attempts() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let fixture = install_sibling_funding_fixture();
    qualify_trap(
        &fixture,
        fixture.children[0],
        CallbackFault::BeforeAwait,
        false,
    );
    qualify_trap(
        &fixture,
        fixture.children[1],
        CallbackFault::AfterAwait,
        true,
    );
    qualify_held_reply(&fixture, fixture.children[0]);
}

#[test]
pub(in super::super) fn root_snapshot_keeps_pool_demand_coherent_across_history_and_rearm() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let fixture = install_sibling_funding_fixture();
    for _ in 0..32 {
        fixture.pic.tick().expect("PocketIC tick");
    }
    let request = canic::dto::authority_restore::AuthoritySnapshotRequest {
        operation_id: [0xe5; 32],
    };
    let prepare = snapshot_spec(true);
    let resume = snapshot_spec(false);
    let initial = submit_snapshot(&fixture, prepare, request);
    observe_snapshot_coherence(&fixture, std::slice::from_ref(&initial));
    snapshot_reply(&fixture, initial, prepare).unwrap();
    let sealed_timers = root_runtime(&fixture.pic, fixture.root).timers;
    crate::pic::protected_state::prepare_snapshot(
        &fixture.pic,
        fixture.root,
        Principal::anonymous(),
        request,
    )
    .unwrap();
    assert_eq!(
        root_runtime(&fixture.pic, fixture.root).timers,
        sealed_timers
    );
    crate::pic::protected_state::resume_snapshot(
        &fixture.pic,
        fixture.root,
        Principal::anonymous(),
        request,
    )
    .unwrap();
    for _ in 0..16 {
        fixture.pic.tick().expect("PocketIC tick");
    }
    let resumed_timers = root_runtime(&fixture.pic, fixture.root).timers;
    crate::pic::protected_state::resume_snapshot(
        &fixture.pic,
        fixture.root,
        Principal::anonymous(),
        request,
    )
    .unwrap();
    assert_eq!(
        root_runtime(&fixture.pic, fixture.root).timers,
        resumed_timers
    );
    for resume_first in [true, false] {
        let specs = if resume_first {
            [resume, prepare]
        } else {
            [prepare, resume]
        };
        let calls = specs.map(|spec| submit_snapshot(&fixture, spec, request));
        observe_snapshot_coherence(&fixture, &calls);
        for (call, spec) in calls.into_iter().zip(specs) {
            if let Err(error) = snapshot_reply(&fixture, call, spec) {
                assert_eq!(
                    error.code(),
                    canic_contracts::diagnostics::codes::STATE_CONFLICT.raw_code()
                );
            }
        }
        let status = crate::pic::protected_state::authority_restore(
            &fixture.pic,
            fixture.root,
            Principal::anonymous(),
        )
        .unwrap();
        if status.phase == canic::dto::authority_restore_status::Phase::Sealed {
            crate::pic::protected_state::resume_snapshot(
                &fixture.pic,
                fixture.root,
                Principal::anonymous(),
                request,
            )
            .unwrap();
        }
        for _ in 0..16 {
            fixture.pic.tick().expect("PocketIC tick");
        }
    }
}

fn snapshot_spec(prepare: bool) -> ic_testkit::pic::WireCallSpec {
    use canic::protocol::protected_state;
    if prepare {
        ic_testkit::pic::WireCallSpec {
            method: protected_state::snapshot_prepare::METHOD,
            method_id: protected_state::snapshot_prepare::METHOD_ID,
            request_limits: protected_state::snapshot_prepare::REQUEST_LIMITS,
            response_limits: protected_state::RESPONSE_LIMITS,
        }
    } else {
        ic_testkit::pic::WireCallSpec {
            method: protected_state::snapshot_resume::METHOD,
            method_id: protected_state::snapshot_resume::METHOD_ID,
            request_limits: protected_state::snapshot_resume::REQUEST_LIMITS,
            response_limits: protected_state::RESPONSE_LIMITS,
        }
    }
}

fn submit_snapshot(
    fixture: &SiblingFundingFixture,
    spec: ic_testkit::pic::WireCallSpec,
    request: canic::dto::authority_restore::AuthoritySnapshotRequest,
) -> RawMessageId {
    let bytes =
        ic_wire::native::encode_request(spec.method_id, &request, spec.request_limits).unwrap();
    fixture
        .pic
        .submit_call(fixture.root, Principal::anonymous(), spec.method, bytes)
        .unwrap()
}

fn snapshot_reply(
    fixture: &SiblingFundingFixture,
    id: RawMessageId,
    spec: ic_testkit::pic::WireCallSpec,
) -> Result<canic::dto::authority_restore_status::Status, Error> {
    let bytes = fixture.pic.await_call(id).unwrap();
    ic_wire::native::decode_response(spec.method_id, &bytes, spec.response_limits).unwrap()
}

fn observe_snapshot_coherence(fixture: &SiblingFundingFixture, calls: &[RawMessageId]) {
    let mut observed_open = false;
    for _ in 0..64 {
        fixture.pic.tick().expect("PocketIC tick");
        let fence = crate::pic::protected_state::authority_restore(
            &fixture.pic,
            fixture.root,
            Principal::anonymous(),
        )
        .unwrap();
        let runtime = root_runtime(&fixture.pic, fixture.root);
        for (subsystem, name) in [
            ("canister_pool", "maintain"),
            ("async_job_recovery", "watchdog"),
        ] {
            let timer = runtime
                .timers
                .iter()
                .find(|timer| {
                    timer.owner == "canic" && timer.subsystem == subsystem && timer.name == name
                })
                .unwrap();
            match fence.phase {
                canic::dto::authority_restore_status::Phase::Open => {
                    observed_open = true;
                    assert_ne!(
                        timer.registration,
                        TimerRegistrationStatus::Unregistered,
                        "an open authority must retain its native producer demand"
                    );
                }
                canic::dto::authority_restore_status::Phase::ReleaseSealed { .. } => {
                    panic!("a snapshot must not create a release seal")
                }
                canic::dto::authority_restore_status::Phase::Sealed => {
                    assert_eq!(
                        timer.registration,
                        TimerRegistrationStatus::Unregistered,
                        "a sealed authority must have no armed producer"
                    );
                    assert_eq!(timer.next_due_at_ns, None);
                }
            }
        }
        if calls.iter().all(|id| {
            fixture
                .pic
                .ingress_status(id.clone())
                .expect("PocketIC ingress_status")
                .is_some()
        }) {
            assert!(
                observed_open,
                "observe the real history-await window before seal"
            );
            return;
        }
    }
    panic!("snapshot commands did not complete within their bounded history observation");
}

fn qualify_trap(
    fixture: &SiblingFundingFixture,
    child: Principal,
    fault: CallbackFault,
    transferred_before_trap: bool,
) {
    configure(fixture, child, fault);
    demand(fixture, child);
    await_state(fixture, || {
        timer(fixture, child).last_outcome == Some(TimerExecutionOutcome::Unacknowledged)
    });
    let trapped = probe(fixture, child);
    assert_eq!(trapped.lease_deadline_ns.is_some(), transferred_before_trap);
    assert_eq!(
        usage(&fixture.pic, fixture.root, fixture.hub, child)
            .accounted_cycles
            .to_u128(),
        if transferred_before_trap { GRANT } else { 0 }
    );
    configure(fixture, child, CallbackFault::None);
    fixture
        .pic
        .advance_time(Duration::from_secs(301))
        .expect("PocketIC advance_time");
    await_state(fixture, || {
        let observed = probe(fixture, child);
        observed.lease_deadline_ns.is_none()
            && observed.entered > trapped.entered
            && usage(&fixture.pic, fixture.root, fixture.hub, child)
                .accounted_cycles
                .to_u128()
                == GRANT
    });
    let recovered = probe(fixture, child);
    if transferred_before_trap {
        assert_eq!(recovered.operation_id, trapped.operation_id);
    }
    assert_eq!(
        timer(fixture, child).registration,
        TimerRegistrationStatus::Scheduled
    );
    assert!(timer(fixture, child).next_due_at_ns.is_some());
    assert_eq!(
        usage(&fixture.pic, fixture.root, fixture.hub, child).pending_operations,
        0
    );
}

fn qualify_held_reply(fixture: &SiblingFundingFixture, child: Principal) {
    configure(fixture, child, CallbackFault::HoldReply);
    demand(fixture, child);
    await_state(fixture, || probe(fixture, child).held);
    let held = probe(fixture, child);
    assert!(held.operation_id.is_some());
    assert!(held.lease_deadline_ns.is_some());
    assert_eq!(
        timer(fixture, child).registration,
        TimerRegistrationStatus::Running
    );
    let paid = usage(&fixture.pic, fixture.root, fixture.hub, child).accounted_cycles;
    assert_eq!(paid.to_u128(), 2 * GRANT);
    fixture
        .pic
        .advance_time(Duration::from_secs(601))
        .expect("PocketIC advance_time");
    for _ in 0..30 {
        fixture.pic.tick().expect("PocketIC tick");
    }
    let still_held = probe(fixture, child);
    assert!(still_held.held);
    assert_eq!(still_held.entered, held.entered);
    assert_eq!(still_held.operation_id, held.operation_id);
    assert_eq!(still_held.lease_deadline_ns, held.lease_deadline_ns);
    assert_eq!(
        timer(fixture, child).registration,
        TimerRegistrationStatus::Running
    );
    assert_eq!(
        usage(&fixture.pic, fixture.root, fixture.hub, child).accounted_cycles,
        paid
    );
    configure(fixture, child, CallbackFault::None);
    await_state(fixture, || {
        probe(fixture, child).lease_deadline_ns.is_none()
    });
    assert_eq!(probe(fixture, child).entered, held.entered);
    assert_eq!(
        usage(&fixture.pic, fixture.root, fixture.hub, child).accounted_cycles,
        paid
    );
    assert_eq!(
        timer(fixture, child).registration,
        TimerRegistrationStatus::Scheduled
    );
}

fn configure(fixture: &SiblingFundingFixture, child: Principal, fault: CallbackFault) {
    ic_testkit::pic::update_wire::<_, Result<(), Error>>(
        &fixture.pic,
        child,
        fixture.root,
        application_probe::TOPUP_CALLBACK_FAULT,
        &fault,
    )
    .unwrap()
    .unwrap();
}

fn demand(fixture: &SiblingFundingFixture, child: Principal) {
    ic_testkit::pic::update_wire::<_, Result<(), Error>>(
        &fixture.pic,
        child,
        fixture.root,
        application_probe::TOPUP_DEMAND,
        &1_900_000_000_000_u128,
    )
    .unwrap()
    .unwrap();
}

fn probe(fixture: &SiblingFundingFixture, child: Principal) -> CallbackProbe {
    ic_testkit::pic::query_wire::<_, Result<CallbackProbe, Error>>(
        &fixture.pic,
        child,
        fixture.root,
        application_probe::TOPUP_CALLBACK_PROBE,
        &(),
    )
    .unwrap()
    .unwrap()
}

fn timer(fixture: &SiblingFundingFixture, child: Principal) -> CanisterTimerStatus {
    let observed: Result<canic::dto::runtime_observation::CanicRuntimeStatus, Error> =
        ic_testkit::pic::query_wire(
            &fixture.pic,
            child,
            fixture.root,
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::protected_runtime::METHOD,
                method_id: canic::protocol::protected_runtime::METHOD_ID,
                request_limits: canic::protocol::protected_runtime::REQUEST_LIMITS,
                response_limits: canic::protocol::protected_runtime::RESPONSE_LIMITS,
            },
            &(),
        )
        .unwrap();
    let observed: canic::dto::runtime::CanicRuntimeStatus = observed.unwrap().into();
    observed
        .timers
        .into_iter()
        .find(|timer| {
            timer.owner == "canic" && timer.subsystem == "cycles" && timer.name == "topup"
        })
        .expect("the sole native top-up registration")
}

fn await_state(fixture: &SiblingFundingFixture, ready: impl Fn() -> bool) {
    for _ in 0..120 {
        if ready() {
            return;
        }
        fixture
            .pic
            .advance_time(Duration::from_secs(1))
            .expect("PocketIC advance_time");
        fixture.pic.tick().expect("PocketIC tick");
    }
    panic!("native funding callback did not reach its required state");
}
