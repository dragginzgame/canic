use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static HTTP_METRICS: RefCell<HashMap<HttpMetricKey, u64>> = RefCell::new(HashMap::new());
}

///
/// HttpMetricKey
/// Uniquely identifies an HTTP outcall by method + label/url
///

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HttpMetricKey {
    pub method: HttpMethodKind,
    pub label: String,
}

///
/// HttpMethodKind
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HttpMethodKind {
    Get,
    Post,
    Head,
}

impl HttpMethodKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Head => "HEAD",
        }
    }
}

///
/// HttpMetrics
/// Volatile counters for HTTP outcalls keyed by method + URL.
/// The label is a url override
///

pub struct HttpMetrics;

impl HttpMetrics {
    pub fn increment(method: HttpMethodKind, label: &str) {
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
    pub fn export_raw() -> HashMap<HttpMetricKey, u64> {
        HTTP_METRICS.with_borrow(std::clone::Clone::clone)
    }
}
