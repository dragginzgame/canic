use super::*;
use canic_management_protocol::backup::{Kind, Operation};

fn runtime(
    fresh: bool,
    memory: &ic_storage::DefaultMemoryImpl,
) -> ic_storage::MemoryRuntime<ic_storage::DefaultMemoryImpl> {
    canic::storage::bootstrap(memory.clone(), fresh, "manager", &[KEY], 512)
}

fn binding() -> Binding {
    Binding {
        operation: Operation {
            id: [1; 16],
            kind: Kind::Capture,
            approval: [2; 32],
            network: [3; 32],
            manager: "manager".into(),
            caller: "operator".into(),
            generation: 1,
            target: "child".into(),
        },
        step: Step::Take,
        request_hash: [4; 32],
        method: "take_canister_snapshot".into(),
        arguments_hash: [5; 32],
        arguments_bytes: 42,
        expires_at_ns: 100,
        cycles: 0,
        cycle_limit: canic_management_protocol::backup::MAX_CYCLES,
        reply_bytes: 1024,
    }
}

#[test]
fn original_slots_survive_reopen_and_conflicts_never_replace() {
    let b = binding();
    let memory = DefaultMemoryImpl::default();
    let mut l = Ledger::open(&runtime(true, &memory), true);
    l.reserve(b.clone()).unwrap();
    assert!(l.occupied());
    drop(l);
    l = Ledger::open(&runtime(false, &memory), false);
    assert_eq!(l.lookup(&b).unwrap().outcome, Outcome::Pending);
    for changed in [0, 1, 2, 3] {
        let mut other = b.clone();
        match changed {
            0 => other.operation.generation += 1,
            1 => other.operation.approval = [9; 32],
            2 => other.arguments_hash = [9; 32],
            _ => other.operation.target = "other".into(),
        }
        assert_eq!(l.lookup(&other), Err(Error::Conflict));
        assert_eq!(l.reserve(other), Err(Error::Conflict));
    }
    let mut next = b.clone();
    next.step = Step::Metadata;
    assert_eq!(l.reserve(next.clone()), Err(Error::Busy));
    l.complete(&b, Outcome::Reply(vec![8, 9]));
    drop(l);
    l = Ledger::open(&runtime(false, &memory), false);
    assert_eq!(l.lookup(&b).unwrap().outcome, Outcome::Reply(vec![8, 9]));
    l.reserve(next).unwrap();
}

#[test]
fn oversized_results_stay_pending_and_chunk_history_reopens() {
    let b = binding();
    let memory = DefaultMemoryImpl::default();
    let mut l = Ledger::open(&runtime(true, &memory), true);
    l.reserve(b.clone()).unwrap();
    l.complete(&b, Outcome::Reply(vec![0; 1025]));
    assert_eq!(l.lookup(&b).unwrap().outcome, Outcome::Pending);
    let mut new = b.clone();
    new.operation.id = [8; 16];
    assert_eq!(l.reserve(new), Err(Error::Busy));
    let memory = DefaultMemoryImpl::default();
    let mut l = Ledger::open(&runtime(true, &memory), true);
    for index in 0..300 {
        let mut b = b.clone();
        b.step = Step::Read(index);
        b.reply_bytes = 1_048_576;
        l.reserve(b.clone()).unwrap();
        l.complete(&b, Outcome::Reply(vec![]));
    }
    drop(l);
    let mut l = Ledger::open(&runtime(false, &memory), false);
    let mut last = b.clone();
    last.step = Step::Read(299);
    last.reply_bytes = 1_048_576;
    assert_eq!(l.lookup(&last).unwrap().outcome, Outcome::Reply(vec![]));
    let mut b = b;
    b.step = Step::FinalStatus;
    l.reserve(b).unwrap();
}

#[test]
fn rewind_admission_uses_the_surviving_capture_and_current_phase() {
    use canic_management_protocol::backup::{Effect, Request, management::CanisterArgs};
    let mut capture = binding();
    capture.step = Step::Fence;
    let memory = DefaultMemoryImpl::default();
    let mut ledger = Ledger::open(&runtime(true, &memory), true);
    let mut remember = |b: &Binding| {
        ledger.reserve(b.clone()).unwrap();
        ledger.complete(b, Outcome::Reply(vec![]));
    };
    remember(&capture);
    for step in [
        Step::Stop,
        Step::Take,
        Step::FinalStatus,
        Step::Start,
        Step::Open,
    ] {
        remember(&Binding {
            step,
            ..capture.clone()
        });
    }
    let mut restore = capture.operation.clone();
    restore.kind = Kind::Restore;
    restore.id = [8; 16];
    restore.approval = [9; 32];
    let request = Request {
        operation: restore,
        step: Step::Fence,
        effect: Effect::Recovery {
            child: CanisterArgs {
                canister_id: "aaaaa-aa".parse().unwrap(),
            },
            source: capture.operation.approval,
        },
        arguments_hash: [0; 32],
        arguments_bytes: 0,
        expires_at_ns: 50,
        cycles: 0,
        cycle_limit: 100,
        reply_bytes: 128,
    };
    drop(ledger);
    let ledger = Ledger::open(&runtime(false, &memory), false);
    assert_eq!(ledger.admit_recovery(&request), Ok(()));
    let mut changed = request.clone();
    changed.operation.generation += 1;
    assert_eq!(ledger.admit_recovery(&changed), Err(Error::Conflict));
    let mut changed = request.clone();
    if let Effect::Recovery { source, .. } = &mut changed.effect {
        *source = [0; 32];
    }
    assert_eq!(ledger.admit_recovery(&changed), Err(Error::Conflict));
    for step in [
        Step::Allocate,
        Step::Load,
        Step::Start,
        Step::Accept,
        Step::Open,
    ] {
        assert_eq!(
            ledger.admit_recovery(&Request {
                step,
                ..request.clone()
            }),
            Err(Error::Busy)
        );
    }
}

#[test]
fn target_fences_survive_upgrade_and_original_results_outlive_the_fence() {
    let a = binding();
    let mut b = a.clone();
    b.operation.target = "sibling".into();
    b.operation.id = [7; 16];
    let memory = DefaultMemoryImpl::default();
    let mut ledger = Ledger::open(&runtime(true, &memory), true);
    ledger.reserve(a.clone()).unwrap();
    ledger.reserve(b.clone()).unwrap();
    ledger.complete(&b, Outcome::Reply(vec![1]));
    let mut finish = b.clone();
    finish.step = Step::FinalStatus;
    ledger.reserve(finish.clone()).unwrap();
    ledger.complete(&finish, Outcome::Reply(vec![]));
    drop(ledger);
    let mut ledger = Ledger::open(&runtime(false, &memory), false);
    assert!(ledger.active("child").is_some());
    assert!(ledger.active("sibling").is_none());
    let mut next = b.clone();
    next.operation.id = [8; 16];
    ledger.reserve(next).unwrap();
    assert_eq!(ledger.lookup(&b).unwrap().outcome, Outcome::Reply(vec![1]));
    assert_eq!(ledger.lookup(&a).unwrap().outcome, Outcome::Pending);
    let mut stale = b;
    stale.step = Step::Metadata;
    assert!(ledger.reserve(stale).is_err());
}
