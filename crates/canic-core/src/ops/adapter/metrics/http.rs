use crate::{
    cdk::mgmt::HttpMethod,
    dto::metrics::http::HttpMetricEntry,
    model::metrics::http::{HttpMethodKind, HttpMetricKey, HttpMetrics},
};

/// Increment HTTP metric using mgmt API types.
/// Performs method mapping and label normalization.
pub fn increment_http_metric(method: HttpMethod, url: &str, label: Option<&str>) {
    let kind = map_method(method);
    let label = label
        .map(str::to_string)
        .unwrap_or_else(|| normalize_url(url));

    HttpMetrics::increment(kind, &label);
}

/// Convert raw HTTP metrics into DTO view.
#[must_use]
pub fn http_metrics_to_view(
    raw: impl IntoIterator<Item = (HttpMetricKey, u64)>,
) -> Vec<HttpMetricEntry> {
    raw.into_iter()
        .map(|(key, count)| HttpMetricEntry {
            method: method_kind_to_string(key.method),
            label: key.label,
            count,
        })
        .collect()
}

fn map_method(method: HttpMethod) -> HttpMethodKind {
    match method {
        HttpMethod::GET => HttpMethodKind::Get,
        HttpMethod::POST => HttpMethodKind::Post,
        HttpMethod::HEAD => HttpMethodKind::Head,
    }
}
