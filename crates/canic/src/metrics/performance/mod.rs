//! Controller observations of completed synchronous application updates.

#[cfg(feature = "endpoint-metrics")]
mod recorder;
#[cfg(all(test, feature = "endpoint-metrics"))]
mod tests;

use crate::method::{Contract, wire};
use serde::{Deserialize, Serialize};

///
/// Measurement
/// Instructions consumed inside a named handler since installation or upgrade.
///

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Measurement {
    /// IC export name.
    pub endpoint: String,
    /// Committed handler returns, saturating on overflow.
    pub calls: u64,
    /// Most recent sample.
    pub latest: u64,
    /// Rounded down, absent if the sample count or total overflowed.
    pub average: Option<u64>,
    /// Largest sample.
    pub maximum: u64,
    /// The sample count or total saturated.
    pub overflowed: bool,
}

///
/// Page
/// Bounded observations ordered by exported name.
///

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Page {
    /// Up to sixteen observed update handlers.
    pub measurements: Vec<Measurement>,
    /// Exclusive name cursor for the next page.
    pub next: Option<String>,
}

///
/// EndpointMetrics
/// Controller-only child query.
///

pub struct EndpointMetrics;

impl Contract for EndpointMetrics {
    const NAME: &'static str = "endpoint_metrics";
    const ID: u64 = wire::method_id(Self::NAME);
    const REQUEST: wire::Limits = wire::Limits {
        nodes: 256,
        ..crate::method::LIMITS
    };
    const REPLY: wire::Limits = wire::Limits {
        bytes: 4096,
        allocation: 65_536,
        nodes: 4096,
        depth: 8,
    };
    type Input = Option<String>;
    type Output = Page;
}

///
/// ChildMetrics
/// Root controller reads one page from a ready managed child slot.
///

pub struct ChildMetrics;

impl Contract for ChildMetrics {
    const NAME: &'static str = "child_metrics";
    const ID: u64 = wire::method_id(Self::NAME);
    const REQUEST: wire::Limits = EndpointMetrics::REQUEST;
    const REPLY: wire::Limits = EndpointMetrics::REPLY;
    type Input = (u8, Option<String>);
    type Output = Page;
}

#[cfg(feature = "endpoint-metrics")]
#[doc(hidden)]
pub use recorder::measure;

/// Expand directly to the handler when measurement is disabled.
#[cfg(not(feature = "endpoint-metrics"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_measure_update {
    ($name:expr, $call:expr) => {
        $call
    };
}

/// Time the handler after admission and decoding.
#[cfg(feature = "endpoint-metrics")]
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_measure_update {
    ($name:expr, $call:expr) => {{
        const _: () = assert!(
            $name.len() <= 128,
            "measured endpoint names must fit the metrics page"
        );
        $crate::metrics::performance::measure($name, || $call)
    }};
}
