use crate::{ids::AccessMetricKind, ops::prelude::*};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ACCESS_METRICS: RefCell<HashMap<AccessMetricKey, u64>> = RefCell::new(HashMap::new());
}

///
/// AccessMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct AccessMetricsSnapshot {
    pub entries: Vec<(AccessMetricKey, u64)>,
}

///
/// AccessMetricKey
/// Uniquely identifies a rejected access attempt by endpoint + stage.
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct AccessMetricKey {
    pub endpoint: String,
    pub kind: AccessMetricKind,
}

///
/// AccessMetrics
/// Volatile counters for unsuccessful access attempts by endpoint + stage.
///

pub struct AccessMetrics;

impl AccessMetrics {
    /// Increment the access-rejection counter for an endpoint/stage pair.
    pub fn increment(endpoint: &str, kind: AccessMetricKind) {
        ACCESS_METRICS.with_borrow_mut(|counts| {
            let key = AccessMetricKey {
                endpoint: endpoint.to_string(),
                kind,
            };

            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    #[must_use]
    pub fn snapshot() -> AccessMetricsSnapshot {
        let entries = ACCESS_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect();

        AccessMetricsSnapshot { entries }
    }

    #[cfg(test)]
    pub fn reset() {
        ACCESS_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_map() -> HashMap<(String, AccessMetricKind), u64> {
        AccessMetrics::snapshot()
            .entries
            .into_iter()
            .map(|(key, count)| ((key.endpoint, key.kind), count))
            .collect()
    }

    #[test]
    fn access_metrics_track_endpoint_and_stage() {
        AccessMetrics::reset();

        AccessMetrics::increment("foo", AccessMetricKind::Guard);
        AccessMetrics::increment("foo", AccessMetricKind::Guard);
        AccessMetrics::increment("foo", AccessMetricKind::Auth);
        AccessMetrics::increment("bar", AccessMetricKind::Rule);

        let mut map = snapshot_map();

        assert_eq!(
            map.remove(&("foo".to_string(), AccessMetricKind::Guard)),
            Some(2)
        );
        assert_eq!(
            map.remove(&("foo".to_string(), AccessMetricKind::Auth)),
            Some(1)
        );
        assert_eq!(
            map.remove(&("bar".to_string(), AccessMetricKind::Rule)),
            Some(1)
        );
        assert!(map.is_empty());
    }
}
