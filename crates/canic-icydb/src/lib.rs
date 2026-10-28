//! Native IcyDB canister exports and host lifecycle composition.

mod endpoints;
mod entropy;
mod lifecycle;
mod transport;
mod watchdog;
pub mod wire;

pub use entropy::request_boot_entropy;
pub use transport::{read_arguments, serve_raw, serve_raw_async};

#[doc(hidden)]
pub mod __private {
    pub use crate::watchdog::{StartupProgress, reconcile_startup_watchdog};
    pub use canic_macros::__canic_managed_component_entry as managed_entry;
    pub use canic_macros::__canic_native_entry as native_entry;
    pub use ic_platform::cdk;
}
