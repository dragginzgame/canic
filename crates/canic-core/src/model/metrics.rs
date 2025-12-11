use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

use crate::types::Principal;

thread_local! {
    static SYSTEM_METRICS: RefCell<HashMap<SystemMetricKind, u64>> = RefCell::new(HashMap::new());
    static ICC_METRICS: RefCell<HashMap<IccMetricKey, u64>> = RefCell::new(HashMap::new());
    static HTTP_METRICS: RefCell<HashMap<HttpMetricKey, u64>> = RefCell::new(HashMap::new());
}

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

///
/// SystemMetricsSnapshot
///

pub type SystemMetricsSnapshot = Vec<SystemMetricEntry>;

///
/// SystemMetricKind
/// Enumerates the resource-heavy actions we track.
///
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[remain::sorted]
pub enum SystemMetricKind {
    CanisterCall,
    CanisterStatus,
    CreateCanister,
    DeleteCanister,
    DepositCycles,
    HttpOutcall,
    InstallCode,
    ReinstallCode,
    UninstallCode,
    UpgradeCode,
}

///
/// SystemMetricEntry
/// Snapshot entry pairing a metric kind with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemMetricEntry {
    pub kind: SystemMetricKind,
    pub count: u64,
}

///
/// IccMetricKey
/// Uniquely identifies an inter-canister call by target + method.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct IccMetricKey {
    pub target: Principal,
    pub method: String,
}

///
/// IccMetricEntry
/// Snapshot entry pairing a target/method with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct IccMetricEntry {
    pub target: Principal,
    pub method: String,
    pub count: u64,
}

///
/// HttpMetricKey
/// Uniquely identifies an HTTP outcall by method + URL.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct HttpMetricKey {
    pub method: String,
    pub url: String,
}

///
/// HttpMetricEntry
/// Snapshot entry pairing a method/url with its count.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpMetricEntry {
    pub method: String,
    pub url: String,
    pub count: u64,
}

///
/// HttpMetricsSnapshot
///

pub type HttpMetricsSnapshot = Vec<HttpMetricEntry>;

///
/// IccMetricsSnapshot
///

pub type IccMetricsSnapshot = Vec<IccMetricEntry>;

///
/// MetricsReport
/// Composite metrics view bundling action, ICC, and HTTP counters.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MetricsReport {
    pub system: SystemMetricsSnapshot,
    pub icc: IccMetricsSnapshot,
    pub http: HttpMetricsSnapshot,
}

// -----------------------------------------------------------------------------
// State
// -----------------------------------------------------------------------------

///
/// SystemMetrics
/// Thin facade over the action metrics counters.
///

pub struct SystemMetrics;

impl SystemMetrics {
    /// Increment a counter and return the new value.
    pub fn increment(kind: SystemMetricKind) {
        SYSTEM_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(kind).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Return a snapshot of all counters.
    #[must_use]
    pub fn snapshot() -> Vec<SystemMetricEntry> {
        SYSTEM_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(kind, count)| SystemMetricEntry {
                    kind: *kind,
                    count: *count,
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        SYSTEM_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// IccMetrics
/// Volatile counters for inter-canister calls keyed by target + method.
///
pub struct IccMetrics;

impl IccMetrics {
    /// Increment the ICC counter for a target/method pair.
    pub fn increment(target: Principal, method: &str) {
        ICC_METRICS.with_borrow_mut(|counts| {
            let key = IccMetricKey {
                target,
                method: method.to_string(),
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot all ICC counters.
    #[must_use]
    pub fn snapshot() -> IccMetricsSnapshot {
        ICC_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(key, count)| IccMetricEntry {
                    target: key.target,
                    method: key.method.clone(),
                    count: *count,
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        ICC_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// HttpMetrics
/// Volatile counters for HTTP outcalls keyed by method + URL.
///
pub struct HttpMetrics;

impl HttpMetrics {
    pub fn increment(method: &str, url: &str) {
        HTTP_METRICS.with_borrow_mut(|counts| {
            let key = HttpMetricKey {
                method: method.to_string(),
                url: url.to_string(),
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    #[must_use]
    pub fn snapshot() -> HttpMetricsSnapshot {
        HTTP_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(key, count)| HttpMetricEntry {
                    method: key.method.clone(),
                    url: key.url.clone(),
                    count: *count,
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        HTTP_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn increments_and_snapshots() {
        SystemMetrics::reset();

        SystemMetrics::increment(SystemMetricKind::CreateCanister);
        SystemMetrics::increment(SystemMetricKind::CreateCanister);
        SystemMetrics::increment(SystemMetricKind::InstallCode);

        let snapshot = SystemMetrics::snapshot();
        let as_map: HashMap<SystemMetricKind, u64> = snapshot
            .into_iter()
            .map(|entry| (entry.kind, entry.count))
            .collect();

        assert_eq!(as_map.get(&SystemMetricKind::CreateCanister), Some(&2));
        assert_eq!(as_map.get(&SystemMetricKind::InstallCode), Some(&1));
        assert!(!as_map.contains_key(&SystemMetricKind::CanisterCall));
    }

    #[test]
    fn icc_metrics_track_target_and_method() {
        IccMetrics::reset();

        let t1 = Principal::from_slice(&[1; 29]);
        let t2 = Principal::from_slice(&[2; 29]);

        IccMetrics::increment(t1, "foo");
        IccMetrics::increment(t1, "foo");
        IccMetrics::increment(t1, "bar");
        IccMetrics::increment(t2, "foo");

        let snapshot = IccMetrics::snapshot();
        let mut map: HashMap<(Principal, String), u64> = snapshot
            .into_iter()
            .map(|entry| ((entry.target, entry.method), entry.count))
            .collect();

        assert_eq!(map.remove(&(t1, "foo".to_string())), Some(2));
        assert_eq!(map.remove(&(t1, "bar".to_string())), Some(1));
        assert_eq!(map.remove(&(t2, "foo".to_string())), Some(1));
        assert!(map.is_empty());
    }

    #[test]
    fn http_metrics_track_method_and_url() {
        HttpMetrics::reset();

        HttpMetrics::increment("GET", "https://example.com/a");
        HttpMetrics::increment("GET", "https://example.com/a");
        HttpMetrics::increment("POST", "https://example.com/a");
        HttpMetrics::increment("GET", "https://example.com/b");

        let snapshot = HttpMetrics::snapshot();
        let mut map: HashMap<(String, String), u64> = snapshot
            .into_iter()
            .map(|entry| ((entry.method, entry.url), entry.count))
            .collect();

        assert_eq!(
            map.remove(&("GET".to_string(), "https://example.com/a".to_string())),
            Some(2)
        );
        assert_eq!(
            map.remove(&("POST".to_string(), "https://example.com/a".to_string())),
            Some(1)
        );
        assert_eq!(
            map.remove(&("GET".to_string(), "https://example.com/b".to_string())),
            Some(1)
        );
        assert!(map.is_empty());
    }
}
