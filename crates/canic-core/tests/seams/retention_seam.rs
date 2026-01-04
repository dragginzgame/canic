use canic_core::{
    cdk::types::Cycles,
    domain::policy,
    ops::storage::cycles::CycleTrackerOps,
};

#[test]
fn retention_uses_policy_cutoff_for_cycles() {
    let _guard = crate::lock();

    let now = 1_000_000;
    let cutoff = policy::cycles::retention_cutoff(now);

    CycleTrackerOps::record(cutoff - 1, Cycles::new(1));
    CycleTrackerOps::record(cutoff, Cycles::new(2));
    CycleTrackerOps::record(cutoff + 1, Cycles::new(3));

    let purged = CycleTrackerOps::purge_before(cutoff);
    assert_eq!(purged, 1);

    let timestamps: Vec<u64> = CycleTrackerOps::snapshot()
        .entries
        .into_iter()
        .map(|(ts, _)| ts)
        .collect();

    assert!(timestamps.contains(&cutoff));
    assert!(timestamps.contains(&(cutoff + 1)));
    assert!(!timestamps.contains(&(cutoff - 1)));
}
