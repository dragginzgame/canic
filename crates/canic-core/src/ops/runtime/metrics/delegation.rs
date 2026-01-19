use crate::cdk::types::Principal;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static DELEGATION_METRICS: RefCell<HashMap<Principal, u64>> = RefCell::new(HashMap::new());
}

///
/// DelegationMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct DelegationMetricsSnapshot {
    pub entries: Vec<(Principal, u64)>,
}

///
/// DelegationMetrics
/// Records verified delegation authorities by signer principal.
///

pub struct DelegationMetrics;

impl DelegationMetrics {
    pub fn record_authority(authority: Principal) {
        DELEGATION_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(authority).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    #[must_use]
    pub fn snapshot() -> DelegationMetricsSnapshot {
        let entries = DELEGATION_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect();

        DelegationMetricsSnapshot { entries }
    }

    #[cfg(test)]
    pub fn reset() {
        DELEGATION_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn snapshot_map() -> HashMap<Principal, u64> {
        DelegationMetrics::snapshot().entries.into_iter().collect()
    }

    #[test]
    fn record_authority_increments() {
        DelegationMetrics::reset();

        let pid = p(1);
        DelegationMetrics::record_authority(pid);
        DelegationMetrics::record_authority(pid);

        let map = snapshot_map();
        assert_eq!(map.get(&pid), Some(&2));
    }
}
