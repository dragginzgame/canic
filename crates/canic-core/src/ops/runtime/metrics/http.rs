use crate::storage::metrics::http::{
    HttpMethodKind, HttpMetricKey as ModelHttpMetricKey, HttpMetrics,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HttpMetricKey {
    pub method: HttpMethod,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct HttpMetricsSnapshot {
    pub entries: Vec<(HttpMetricKey, u64)>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Head,
}

#[must_use]
pub fn snapshot() -> HttpMetricsSnapshot {
    let entries = HttpMetrics::export_raw()
        .into_iter()
        .map(|(key, count)| (key.into(), count))
        .collect();
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
        HttpMethod::Get => HttpMethodKind::Get,
        HttpMethod::Post => HttpMethodKind::Post,
        HttpMethod::Head => HttpMethodKind::Head,
    }
}

const fn http_method_from_kind(method: HttpMethodKind) -> HttpMethod {
    match method {
        HttpMethodKind::Get => HttpMethod::Get,
        HttpMethodKind::Post => HttpMethod::Post,
        HttpMethodKind::Head => HttpMethod::Head,
    }
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

impl From<ModelHttpMetricKey> for HttpMetricKey {
    fn from(key: ModelHttpMetricKey) -> Self {
        Self {
            method: http_method_from_kind(key.method),
            label: key.label,
        }
    }
}
