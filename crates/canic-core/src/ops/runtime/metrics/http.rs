use crate::infra::ic::http::HttpMethod as InfraHttpMethod;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static HTTP_METRICS: RefCell<HashMap<HttpMetricKey, u64>> = RefCell::new(HashMap::new());
}

///
/// HttpMetricKey
///

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HttpMetricKey {
    pub method: HttpMethod,
    pub label: String,
}

///
/// HttpMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct HttpMetricsSnapshot {
    pub entries: Vec<(HttpMetricKey, u64)>,
}

///
/// HttpMethod
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Head,
}

impl HttpMethod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Head => "HEAD",
        }
    }
}

impl From<InfraHttpMethod> for HttpMethod {
    fn from(method: crate::infra::ic::http::HttpMethod) -> Self {
        match method {
            InfraHttpMethod::GET => Self::Get,
            InfraHttpMethod::POST => Self::Post,
            InfraHttpMethod::HEAD => Self::Head,
        }
    }
}

///
/// HttpMetrics
/// Volatile counters for HTTP outcalls keyed by method + label.
/// The label is a URL override or normalized URL.
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
