//! Compose the independent blob service in a consumer-owned Canic canister.
//!
//! The consumer owns App configuration and the canister artifact. This library
//! owns only endpoint/lifecycle integration and aggregate metrics; the upstream
//! service owns storage behavior. Use [`mount!`] in an existing application or
//! [`canister!`] for a dedicated service canister.

#[cfg(all(feature = "wire-schema", target_arch = "wasm32"))]
compile_error!("Blob wire-schema is a native producer feature; exclude it from deployed artifacts");

mod endpoints;
mod ops;
#[cfg(all(feature = "wire-schema", not(target_arch = "wasm32")))]
mod schema;
#[doc(hidden)]
pub mod transport;
mod workflow;

pub mod funding;
pub mod lifecycle;
pub mod metrics;
pub use ops::memory::requests as memory_requests;

/// Host namespace owner for the service’s permanent memory requests.
pub const MEMORY_AUTHORITY: &str = "blob-service";

/// Permanent namespace granted by the composing artifact’s allocation pool.
pub const MEMORY_KEY_PREFIX: &str = "blob.";

/// Upstream typed service contracts, shared without a second DTO schema.
pub use ic_blob_storage_contracts::dto;

/// Dependencies used by the canister composition macro.
#[doc(hidden)]
pub mod __private {
    pub mod operation_metrics {
        pub use crate::workflow::{admission, reference};
    }
    pub use crate::ops::{Workflow, memory};
    #[cfg(all(feature = "wire-schema", not(target_arch = "wasm32")))]
    pub use crate::schema::methods as wire_methods;
    pub use crate::transport;
    pub use ic_blob_storage_contracts;
    pub use ic_memory;
    pub use ic_platform;
}

/// Register the supplier projections only in its explicitly selected native producer.
#[doc(hidden)]
#[cfg(all(feature = "wire-schema", not(target_arch = "wasm32")))]
#[macro_export]
macro_rules! __canic_blob_schema {
    () => {
        canic::__canic_register_wire_schema_provider!(
            $crate::__private::wire_methods,
            Some($crate::__private::ic_blob_storage_contracts::wire::schema::runtime_adapter)
        );
    };
}

/// Ordinary mounted artifacts retain only runtime codec declarations.
#[doc(hidden)]
#[cfg(not(all(feature = "wire-schema", not(target_arch = "wasm32"))))]
#[macro_export]
macro_rules! __canic_blob_schema {
    () => {};
}
