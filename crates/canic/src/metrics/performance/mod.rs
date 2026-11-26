//! Controller observations of completed synchronous application updates.

mod recorder;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

///
/// Measurement
/// Instructions consumed inside a named handler since installation or upgrade.
///

#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
#[cfg_attr(feature = "inspection", wire(fixed))]
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

#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
#[cfg_attr(feature = "inspection", wire(fixed))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Page {
    /// Up to sixteen observed update handlers.
    pub measurements: Vec<Measurement>,
    /// Exclusive name cursor for the next page.
    pub next: Option<String>,
}

#[doc(hidden)]
pub use recorder::measure;
pub(super) use recorder::page;

/// Time the handler after admission and decoding.
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
