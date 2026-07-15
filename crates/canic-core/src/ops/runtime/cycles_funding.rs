//! Module: ops::runtime::cycles_funding
//!
//! Responsibility: expose child cycles funding ledger snapshots to runtime workflows.
//! Does not own: cycles funding policy, grant execution, or stable storage schema.
//! Boundary: routes runtime grant totals through stable storage accounting.

use crate::{
    cdk::types::Principal, model::cycles_funding::FundingLedgerSnapshot,
    ops::storage::cycles::CyclesFundingLedgerStoreOps,
};

///
/// CyclesFundingLedgerOps
///
/// Operations-layer facade for runtime child funding ledger snapshots.
///

pub struct CyclesFundingLedgerOps;

impl CyclesFundingLedgerOps {
    /// Return the current grant ledger state for a child.
    #[must_use]
    pub fn snapshot(child: Principal) -> FundingLedgerSnapshot {
        CyclesFundingLedgerStoreOps::snapshot(child).unwrap_or_default()
    }

    /// Record a successful grant for cooldown and child-budget accounting.
    pub fn record_child_grant(child: Principal, granted_cycles: u128, now_secs: u64) {
        CyclesFundingLedgerStoreOps::record_child_grant(child, granted_cycles, now_secs);
    }

    /// Restore a previously observed ledger snapshot after a failed external grant.
    pub fn restore_child_snapshot(child: Principal, snapshot: FundingLedgerSnapshot) {
        CyclesFundingLedgerStoreOps::restore_child_snapshot(child, snapshot);
    }

    #[cfg(test)]
    pub fn reset_for_tests() {
        CyclesFundingLedgerStoreOps::reset_for_tests();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_child_grant_accumulates_in_stable_ledger() {
        let child = Principal::from_slice(&[42; 29]);

        CyclesFundingLedgerOps::reset_for_tests();
        CyclesFundingLedgerOps::record_child_grant(child, 100, 10);
        CyclesFundingLedgerOps::record_child_grant(child, 25, 20);

        assert_eq!(
            CyclesFundingLedgerOps::snapshot(child),
            FundingLedgerSnapshot {
                granted_total: 125,
                last_granted_at: 20,
            }
        );
    }

    #[test]
    fn restore_child_snapshot_reverts_budget_state() {
        let child = Principal::from_slice(&[43; 29]);
        let previous = FundingLedgerSnapshot {
            granted_total: 80,
            last_granted_at: 7,
        };

        CyclesFundingLedgerOps::reset_for_tests();
        CyclesFundingLedgerOps::restore_child_snapshot(child, previous);
        CyclesFundingLedgerOps::record_child_grant(child, 20, 10);
        CyclesFundingLedgerOps::restore_child_snapshot(child, previous);

        assert_eq!(CyclesFundingLedgerOps::snapshot(child), previous);
    }
}
