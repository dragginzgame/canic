//! Small contracts and explicitly selected actor adapters.
//! No host dependencies or management surface enters a Wasm consumer.

/// Optional caller-bound Auth endpoints; lifecycle remains explicit.
#[cfg(feature = "auth")]
pub mod auth;

/// Optional Blob endpoint composition; no implicit lifecycle or service activation.
#[cfg(feature = "blob")]
pub mod blob;

/// Optional application-owned typed database endpoints.
#[cfg(any(feature = "db", feature = "db-store"))]
pub mod db;

/// One bounded management entropy request, with no local generator or retry.
#[cfg(all(feature = "entropy", target_arch = "wasm32"))]
pub mod entropy;

/// Explicit lifecycle hooks independent of storage and Auth.
#[cfg(feature = "lifecycle")]
pub mod lifecycle;

#[cfg(feature = "method")]
pub mod method;

/// Passive operational reports; the optional `metrics` feature enables endpoint mounting.
pub mod metrics;

/// Application-owned readiness; the optional feature enables an explicit replicated mount.
pub mod readiness;

#[cfg(feature = "recovery")]
pub mod recovery;

#[cfg(feature = "storage")]
pub mod storage;

/// Optional runtime shard assignment and lookup for application hubs.
#[cfg(feature = "sharding")]
pub mod sharding;

/// Fleet-approved shard installation context.
#[cfg(feature = "topology")]
pub mod topology;

#[cfg(feature = "method")]
pub use canic_macros::{query, update};
