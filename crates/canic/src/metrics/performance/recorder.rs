//! Only compile-time export names can allocate a recorder entry.

use super::{EndpointMetrics, Measurement, Page};
use crate::metrics::store;
use ic_metrics::MeasurementSummary;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    ops::Bound::{Excluded, Unbounded},
};

crate::__canic_report_mount!("canic:module:endpoint-metrics");

///
/// UpdateInstructions
/// Stores the instruction samples for completed application updates.
///

struct UpdateInstructions;

impl store::Source for UpdateInstructions {
    type Storage = RefCell<BTreeMap<&'static str, MeasurementSummary>>;
}

fn instructions() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        ic_platform_runtime::raw::instruction_counter()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ic_platform_runtime::system::trap("endpoint measurement requires a canister")
    }
}

/// Record one returned synchronous handler. IC message rollback also rolls back the sample.
pub fn measure<T>(name: &'static str, handler: impl FnOnce() -> T) -> T {
    let start = instructions();
    let result = handler();
    let elapsed = instructions() - start;
    store::open::<UpdateInstructions>()
        .borrow_mut()
        .entry(name)
        .or_default()
        .record(elapsed);

    result
}

/// Admit only actual IC controllers.
/// # Errors
/// Refuses every other caller.
fn controller() -> Result<(), &'static str> {
    use ic_platform_runtime::system;
    system::is_controller(&system::caller())
        .then_some(())
        .ok_or("controller required")
}

/// Project one page from observed static export names in lexical order.
/// # Panics
/// A retained entry must contain at least one completed sample.
#[must_use]
#[crate::query(
    contract = EndpointMetrics,
    guard = controller,
)]
fn endpoint_metrics(after: Option<String>) -> Page {
    let Some(storage) = store::get::<UpdateInstructions>() else {
        return Page {
            measurements: Vec::new(),
            next: None,
        };
    };
    let entries = storage.borrow();
    let mut rows =
        entries.range::<str, _>((after.as_deref().map_or(Unbounded, Excluded), Unbounded));
    let measurements: Vec<_> = rows
        .by_ref()
        .take(16)
        .map(|(&name, summary)| {
            let aggregate = summary.aggregate();
            let overflowed = aggregate.samples_overflowed() || aggregate.total_overflowed();

            Measurement {
                endpoint: name.to_owned(),
                calls: aggregate.samples(),
                latest: summary.latest().expect("recorded sample"),
                average: (!overflowed).then(|| {
                    u64::try_from(aggregate.total() / u128::from(aggregate.samples()))
                        .expect("average of u64 samples")
                }),
                maximum: aggregate.maximum().expect("recorded sample"),
                overflowed,
            }
        })
        .collect();
    let next = rows
        .next()
        .and_then(|_| measurements.last().map(|row| row.endpoint.clone()));

    Page { measurements, next }
}
