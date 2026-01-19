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

    for (pid, _) in PoolOps::data().entries {
        PoolOps::remove(&pid);
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
