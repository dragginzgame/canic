use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static HTTP_METRICS: RefCell<HashMap<HttpMetricKey, u64>> = RefCell::new(HashMap::new());
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
/// HttpMetrics
/// Volatile counters for HTTP outcalls keyed by method + URL.
/// The label is a url override
///

pub struct HttpMetrics;

impl HttpMetrics {
    pub fn increment(method: &str, url: &str) {
        Self::increment_with_label(method, url, None);
    }

    pub fn increment_with_label(method: &str, url: &str, label: Option<&str>) {
        let label = Self::label_for(url, label);

        HTTP_METRICS.with_borrow_mut(|counts| {
            let key = HttpMetricKey {
                method: method.to_string(),
                url: label,
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

    fn label_for(url: &str, label: Option<&str>) -> String {
        if let Some(label) = label {
            return label.to_string();
        }

        Self::normalize(url)
    }

    fn normalize(url: &str) -> String {
        let without_fragment = url.split('#').next().unwrap_or(url);
        let without_query = without_fragment
            .split('?')
            .next()
            .unwrap_or(without_fragment);

        let candidate = without_query.trim();
        if candidate.is_empty() {
            url.to_string()
        } else {
            candidate.to_string()
        }
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

    #[test]
    fn http_metrics_track_method_and_url_normalized() {
        HttpMetrics::reset();

        HttpMetrics::increment("GET", "https://example.com/a?query=1#frag");
        HttpMetrics::increment("GET", "https://example.com/a?query=2");
        HttpMetrics::increment("POST", "https://example.com/a?query=3");
        HttpMetrics::increment("GET", "https://example.com/b#x");

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

    #[test]
    fn http_metrics_allow_custom_labels() {
        HttpMetrics::reset();

        HttpMetrics::increment_with_label(
            "GET",
            "https://example.com/search?q=abc",
            Some("search"),
        );
        HttpMetrics::increment_with_label(
            "GET",
            "https://example.com/search?q=def",
            Some("search"),
        );

        let snapshot = HttpMetrics::snapshot();
        let mut map: HashMap<(String, String), u64> = snapshot
            .into_iter()
            .map(|entry| ((entry.method, entry.url), entry.count))
            .collect();

        assert_eq!(
            map.remove(&("GET".to_string(), "search".to_string())),
            Some(2)
        );
        assert!(map.is_empty());
    }
}
