use crate::cdk::types::Principal;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static DELEGATED_AUTH_METRICS: RefCell<HashMap<Principal, u64>> = RefCell::new(HashMap::new());
}

///
/// DelegatedAuthMetrics
/// Records verified delegation authorities by signer principal.
///

pub struct DelegatedAuthMetrics;

impl DelegatedAuthMetrics {
    pub fn record_authority(authority: Principal) {
        DELEGATED_AUTH_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(authority).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    #[must_use]
    pub fn snapshot() -> Vec<(Principal, u64)> {
        DELEGATED_AUTH_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    #[cfg(test)]
    pub fn reset() {
        DELEGATED_AUTH_METRICS.with_borrow_mut(HashMap::clear);
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
        DelegatedAuthMetrics::snapshot().into_iter().collect()
    }

    #[test]
    fn record_authority_increments() {
        DelegatedAuthMetrics::reset();

        let pid = p(1);
        DelegatedAuthMetrics::record_authority(pid);
        DelegatedAuthMetrics::record_authority(pid);

        let map = snapshot_map();
        assert_eq!(map.get(&pid), Some(&2));
    }
}
