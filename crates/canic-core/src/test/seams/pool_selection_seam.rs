// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    cdk::types::Cycles,
    ops::storage::pool::PoolOps,
    storage::stable::pool::{PoolStatus, PoolStore},
    test::seams::{lock, p},
    workflow::pool::PoolWorkflow,
};

#[test]
fn pool_selection_uses_workflow_ordering() {
    let _guard = lock();

    for entry in PoolOps::data().entries {
        PoolOps::remove(&entry.pid);
    }

    let pid_a = p(20);
    let pid_b = p(21);
    let pid_c = p(22);

    PoolStore::register(
        pid_a,
        Cycles::new(1),
        PoolStatus::Ready,
        None,
        None,
        None,
        5,
    );
    PoolStore::register(
        pid_b,
        Cycles::new(1),
        PoolStatus::Ready,
        None,
        None,
        None,
        5,
    );
    PoolStore::register(
        pid_c,
        Cycles::new(1),
        PoolStatus::Ready,
        None,
        None,
        None,
        9,
    );

    let selected = PoolWorkflow::pop_oldest_ready().expect("expected a ready entry");
    assert_eq!(selected, pid_a);
    assert!(!PoolOps::contains(&pid_a));

    let next = PoolWorkflow::pop_oldest_ready().expect("expected a second ready entry");
    assert_eq!(next, pid_b);
    assert!(!PoolOps::contains(&pid_b));
    assert!(PoolOps::contains(&pid_c));
}

#[test]
fn pending_reset_selection_is_ordered_bounded_and_non_destructive() {
    let _guard = lock();

    for entry in PoolOps::data().entries {
        PoolOps::remove(&entry.pid);
    }

    let pid_a = p(23);
    let pid_b = p(24);
    let pid_c = p(25);

    PoolStore::register(
        pid_a,
        Cycles::default(),
        PoolStatus::PendingReset,
        None,
        None,
        None,
        5,
    );
    PoolStore::register(
        pid_b,
        Cycles::default(),
        PoolStatus::PendingReset,
        None,
        None,
        None,
        5,
    );
    PoolStore::register(
        pid_c,
        Cycles::default(),
        PoolStatus::PendingReset,
        None,
        None,
        None,
        9,
    );

    let first = PoolOps::pending_reset_page(None, 1);
    assert_eq!(first.pids, vec![pid_a]);

    let second = PoolOps::pending_reset_page(first.next_cursor.as_ref(), 1);
    assert_eq!(second.pids, vec![pid_b]);

    let third = PoolOps::pending_reset_page(second.next_cursor.as_ref(), 1);
    assert_eq!(third.pids, vec![pid_c]);
    assert!(third.next_cursor.is_none());

    let empty = PoolOps::pending_reset_page(None, 0);
    assert!(empty.pids.is_empty());
    assert!(empty.next_cursor.is_none());
    assert!(PoolOps::contains(&pid_a));
    assert!(PoolOps::contains(&pid_b));
    assert!(PoolOps::contains(&pid_c));
}
