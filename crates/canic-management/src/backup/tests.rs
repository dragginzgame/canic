use super::*;
use canic_management_protocol::backup::{Kind, Operation};
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
    let mut l = Ledger::default();
    l.reserve(b.clone()).unwrap();
    assert!(l.occupied());
    l = Ledger::open(&l.encode());
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
    l = Ledger::open(&l.encode());
    assert_eq!(l.lookup(&b).unwrap().outcome, Outcome::Reply(vec![8, 9]));
    l.reserve(next).unwrap();
}
#[test]
fn oversized_results_remain_pending_and_liabilities_never_refund() {
    let b = binding();
    let mut l = Ledger::default();
    l.reserve(b.clone()).unwrap();
    l.complete(&b, Outcome::Reply(vec![0; 1025]));
    assert_eq!(l.lookup(&b).unwrap().outcome, Outcome::Pending);
    let mut new = b.clone();
    new.operation.id = [8; 16];
    assert_eq!(l.reserve(new), Err(Error::Busy));
    let mut l = Ledger::default();
    for index in 0..MAX_STEPS {
        let mut b = b.clone();
        b.step = Step::Read(index as u32);
        l.reserve(b.clone()).unwrap();
        l.complete(&b, Outcome::Reply(vec![]));
    }
    let mut b = b.clone();
    b.step = Step::FinalStatus;
    assert_eq!(l.reserve(b), Err(Error::Capacity));
}
