use super::*;
use crate::fleet_ensure::{
    policy::startup_funding::hub_config, view::startup_funding::StartupChildAccounting,
};

const T: u128 = 1_000_000_000_000;

pub(in crate::fleet_ensure) fn qualify(desired: &DesiredFleet, root: &StartupRootFunding) {
    let config = hub_config();
    let mut root = root.clone();
    root.balance = StartupNativeBalance::Observed(0);
    for entry in &mut root.child_usage {
        entry.observed_balance_cycles = Some(4 * T);
        entry.usage = Ok(StartupChildAccounting {
            observed_at_ns: 100_000_000_000,
            accounted_cycles: 0,
            last_accounted_at_secs: 0,
            pending_operations: 0,
            reserved_cycles: Some(0),
        });
    }
    let demand = project(&config, desired, &root).unwrap();
    assert_eq!(demand.root_grants_cycles, 30 * T);
    assert_eq!(demand.uncovered_cycles, 0);
    assert_eq!(demand.shortfall_cycles, demand.minimum_native_cycles);
    let hub = demand
        .children
        .iter()
        .find(|child| child.outgoing_cycles > 0)
        .unwrap();
    assert_eq!(hub.outgoing_cycles, 20 * T);
    assert_eq!(hub.parent_grants_cycles, 30 * T);
    root.child_usage.reverse();
    assert_eq!(project(&config, desired, &root), Ok(demand));
    root.child_usage[0].observed_balance_cycles = None;
    assert_eq!(
        project(&config, desired, &root),
        Err(StartupDemandUnavailable::BalanceNotObserved)
    );
    root.child_usage[0].observed_balance_cycles = Some(4 * T);
    root.child_usage[0]
        .usage
        .as_mut()
        .unwrap()
        .pending_operations = 1;
    assert_eq!(
        project(&config, desired, &root),
        Err(StartupDemandUnavailable::PendingGrant)
    );
    root.child_usage[0]
        .usage
        .as_mut()
        .unwrap()
        .pending_operations = 0;
    for child in &mut root.child_usage {
        child.usage.as_mut().unwrap().accounted_cycles = u128::MAX;
    }
    let exhausted = project(&config, desired, &root).unwrap();
    assert_eq!(exhausted.root_grants_cycles, 0);
    assert_eq!(exhausted.uncovered_cycles, 18 * T + 3);
}
