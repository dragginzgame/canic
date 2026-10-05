//! Compose the independent blob service in a consumer-owned Canic canister.
//!
//! The consumer owns App configuration and the canister artifact. This library
//! owns only endpoint/lifecycle integration and aggregate metrics; the upstream
//! service owns storage behavior. Use [`mount!`] in an existing application or
//! [`canister!`] for a dedicated service canister.

mod endpoints;
mod ops;

pub mod lifecycle;
pub mod metrics;
#[doc(hidden)]
pub mod workflow;

/// Upstream typed service contracts, shared without a second DTO schema.
pub use ic_blob_storage::dto;

/// Dependencies used by the canister composition macro.
#[doc(hidden)]
pub mod __private {
    pub use crate::ops::memory;
    pub use ic_blob_storage;
    pub use ic_cdk;
    pub use ic_memory;
}
