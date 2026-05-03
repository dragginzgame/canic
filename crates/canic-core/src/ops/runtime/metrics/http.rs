use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static HTTP_METRICS: RefCell<HashMap<HttpMetricKey, u64>> = RefCell::new(HashMap::new());
}

///
/// HttpMetricKey
///

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct HttpMetricKey {
    pub method: HttpMethod,
    pub label: String,
}

///
/// HttpMetricsSnapshot
///

#[derive(Clone)]
pub struct HttpMetricsSnapshot {
    pub entries: Vec<(HttpMetricKey, u64)>,
}

///
/// HttpMethod
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[remain::sorted]
pub enum HttpMethod {
    Get,
    Head,
    Post,
}

impl HttpMethod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
        }
    }
}

///
/// HttpMetrics
/// Volatile counters for HTTP outcalls keyed by method + low-cardinality label.
/// Explicit labels are preferred; URL-derived fallback labels strip query/fragment only.
///

pub struct HttpMetrics;

impl HttpMetrics {
    /// Record an HTTP outcall with label normalization.
    pub fn record_http_request(method: HttpMethod, url: &str, label: Option<&str>) {
        let label = label.map_or_else(|| normalize_http_label(url, label), str::to_string);

        Self::increment(method, &label);
    }

    fn increment(method: HttpMethod, label: &str) {
        HTTP_METRICS.with_borrow_mut(|counts| {
            let key = HttpMetricKey {
                method,
                label: label.to_string(),
            };

            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    #[must_use]
    pub fn snapshot() -> HttpMetricsSnapshot {
        let entries = HTTP_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect();

        HttpMetricsSnapshot { entries }
    }

    #[cfg(test)]
    pub fn reset() {
        HTTP_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// Normalize an HTTP label from a URL.
///
/// - Removes fragments (`#...`)
/// - Removes query strings (`?...`)
/// - Trims whitespace
/// - Falls back to the original URL if normalization yields an empty string
///
#[must_use]
pub fn normalize_http_label(url: &str, label: Option<&str>) -> String {
    if let Some(label) = label {
        return label.to_string();
    }

    let without_fragment = url.split('#').next().unwrap_or(url);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);

    let trimmed = without_query.trim();
    if trimmed.is_empty() {
        url.to_string()
    } else {
        trimmed.to_string()
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_http_label_prefers_explicit_label() {
        let label = normalize_http_label(
            "https://api.example.test/users/123?token=secret#frag",
            Some("example_api_users"),
        );

        assert_eq!(label, "example_api_users");
    }

    #[test]
    fn normalize_http_label_strips_query_and_fragment() {
        let label =
            normalize_http_label(" https://api.example.test/users?token=secret#frag ", None);

        assert_eq!(label, "https://api.example.test/users");
    }

    #[test]
    fn http_metrics_preserve_explicit_low_cardinality_label() {
        HttpMetrics::reset();

        HttpMetrics::record_http_request(
            HttpMethod::Get,
            "https://api.example.test/users/123?token=secret",
            Some("example_api_users"),
        );
        HttpMetrics::record_http_request(
            HttpMethod::Get,
            "https://api.example.test/users/456?token=secret",
            Some("example_api_users"),
        );

        let snapshot = HttpMetrics::snapshot();
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].0.label, "example_api_users");
        assert_eq!(snapshot.entries[0].1, 2);
    }
}
