//! Bounded blob metrics for the owning application's single sampler.
//!
//! Combine these rows with application rows and register that callback after
//! both installation and restoration. Sampling creates no timer or registration.

pub use crate::ops::metrics::sample;
