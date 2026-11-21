//! Controller observations of completed synchronous application updates.

#[cfg(feature = "endpoint-metrics")]
mod recorder;
#[cfg(all(test, feature = "endpoint-metrics"))]
mod tests;

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

#[cfg(feature = "endpoint-metrics")]
#[doc(hidden)]
pub use recorder::measure;
#[cfg(feature = "endpoint-metrics")]
pub(super) use recorder::page;

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
