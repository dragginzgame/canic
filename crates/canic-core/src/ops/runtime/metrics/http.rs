use crate::{
    cdk::mgmt::HttpMethod,
    storage::metrics::http::{HttpMethodKind, HttpMetricKey, HttpMetrics},
};

#[derive(Clone, Debug)]
pub struct HttpMetricsSnapshot {
    pub entries: Vec<(HttpMetricKey, u64)>,
}

#[must_use]
pub fn snapshot() -> HttpMetricsSnapshot {
    let entries = HttpMetrics::export_raw().into_iter().collect();
    HttpMetricsSnapshot { entries }
}

/// Record an HTTP outcall with label normalization.
pub fn record_http_request(method: HttpMethod, url: &str, label: Option<&str>) {
    let kind = http_method_to_kind(method);
    let label = label.map_or_else(|| normalize_http_label(url, label), str::to_string);

    HttpMetrics::increment(kind, &label);
}

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

const fn http_method_to_kind(method: HttpMethod) -> HttpMethodKind {
    match method {
        HttpMethod::GET => HttpMethodKind::Get,
        HttpMethod::POST => HttpMethodKind::Post,
        HttpMethod::HEAD => HttpMethodKind::Head,
    }
}
