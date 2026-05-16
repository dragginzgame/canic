use crate::{cdk::types::Principal, domain::policy::cycles_funding::FundingLedgerSnapshot};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static FUNDING_LEDGER: RefCell<HashMap<Principal, FundingLedgerSnapshot>> =
        RefCell::new(HashMap::new());
}

///
/// CyclesFundingLedgerOps
///

pub struct CyclesFundingLedgerOps;

impl CyclesFundingLedgerOps {
    /// Return the current grant ledger state for a child.
    #[must_use]
    pub fn snapshot(child: Principal) -> FundingLedgerSnapshot {
        FUNDING_LEDGER.with_borrow(|ledger| ledger.get(&child).copied().unwrap_or_default())
    }

    /// Record a successful grant for cooldown and child-budget accounting.
    pub fn record_child_grant(child: Principal, granted_cycles: u128, now_secs: u64) {
        FUNDING_LEDGER.with_borrow_mut(|ledger| {
            let entry = ledger.entry(child).or_default();
            entry.granted_total = entry.granted_total.saturating_add(granted_cycles);
            entry.last_granted_at = now_secs;
        });
    }

    #[cfg(test)]
    pub fn reset_for_tests() {
        FUNDING_LEDGER.with_borrow_mut(HashMap::clear);
    }
}
