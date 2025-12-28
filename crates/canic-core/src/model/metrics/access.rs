use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ACCESS_METRICS: RefCell<HashMap<AccessMetricKey, u64>> = RefCell::new(HashMap::new());
}

///
/// AccessMetricKind
/// Enumerates the access-control stage that rejected the call.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[remain::sorted]
pub enum AccessMetricKind {
    Auth,
    Guard,
    Policy,
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
    pub fn export_raw() -> HashMap<AccessMetricKey, u64> {
        ACCESS_METRICS.with_borrow(std::clone::Clone::clone)
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
    use std::collections::HashMap;

    #[test]
    fn access_metrics_track_endpoint_and_stage() {
        AccessMetrics::reset();

        AccessMetrics::increment("foo", AccessMetricKind::Guard);
        AccessMetrics::increment("foo", AccessMetricKind::Guard);
        AccessMetrics::increment("foo", AccessMetricKind::Auth);
        AccessMetrics::increment("bar", AccessMetricKind::Policy);

        let raw = AccessMetrics::export_raw();
        let mut map: HashMap<(String, AccessMetricKind), u64> = raw
            .into_iter()
            .map(|(key, count)| ((key.endpoint, key.kind), count))
            .collect();

        assert_eq!(
            map.remove(&("foo".to_string(), AccessMetricKind::Guard)),
            Some(2)
        );
        assert_eq!(
            map.remove(&("foo".to_string(), AccessMetricKind::Auth)),
            Some(1)
        );
        assert_eq!(
            map.remove(&("bar".to_string(), AccessMetricKind::Policy)),
            Some(1)
        );
        assert!(map.is_empty());
    }
}
