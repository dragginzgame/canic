use crate::{
    cdk::mgmt::HttpMethod,
    dto::metrics::HttpMetricEntry,
    ops::runtime::metrics::normalize_http_label,
    storage::metrics::http::{HttpMethodKind, HttpMetricKey, HttpMetrics},
};

/// Increment HTTP metric using mgmt API types.
/// Performs method mapping and label normalization.
pub fn record_http_request(method: HttpMethod, url: &str, label: Option<&str>) {
    let kind = http_method_to_kind(method);
    let label = label.map_or_else(|| normalize_http_label(url, label), str::to_string);

    HttpMetrics::increment(kind, &label);
}

/// Convert raw HTTP metrics into DTO view.
#[must_use]
pub fn http_metrics_to_view(
    raw: impl IntoIterator<Item = (HttpMetricKey, u64)>,
) -> Vec<HttpMetricEntry> {
    raw.into_iter()
        .map(|(key, count)| HttpMetricEntry {
            method: key.method.as_str().to_string(),
            label: key.label,
            count,
        })
        .collect()
}

const fn http_method_to_kind(method: HttpMethod) -> HttpMethodKind {
    match method {
        HttpMethod::GET => HttpMethodKind::Get,
        HttpMethod::POST => HttpMethodKind::Post,
        HttpMethod::HEAD => HttpMethodKind::Head,
    }
}
