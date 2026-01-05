use canic_core::{
    cdk::types::Cycles,
    ops::storage::pool::PoolOps,
    storage::stable::pool::{PoolStatus, PoolStore},
    workflow::pool::PoolWorkflow,
};

#[test]
fn pool_selection_uses_workflow_ordering() {
    let _guard = crate::lock();

    for entry in PoolOps::snapshot().entries {
        PoolOps::remove(&entry.pid);
    }

    let pid_a = crate::p(20);
    let pid_b = crate::p(21);
    let pid_c = crate::p(22);

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
    assert_eq!(selected.pid, pid_a);
    assert!(!PoolOps::contains(&pid_a));
}
