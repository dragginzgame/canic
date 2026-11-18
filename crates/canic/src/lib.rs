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

/// Explicit mounts for the demo's fixed greeting and replicated probe contract.
#[cfg(all(feature = "greeting", target_arch = "wasm32"))]
pub mod greeting;

/// Explicit lifecycle hooks independent of storage and Auth.
#[cfg(all(feature = "lifecycle", target_arch = "wasm32"))]
pub mod lifecycle;

#[cfg(feature = "method")]
pub mod method;

#[cfg(all(feature = "method", target_arch = "wasm32"))]
pub use canic_macros::{query, update};

/// Passive operational reports; the optional `metrics` feature enables endpoint mounting.
pub mod metrics;

/// Application-owned readiness; the optional feature enables an explicit replicated mount.
pub mod readiness;

#[cfg(feature = "recovery")]
pub mod recovery;

use serde::{Deserialize, Serialize};

/// Identity of the greeting RPC, including its Wire request/response envelope.
pub const GREET: u64 = 1;
/// Shared admission policy for the tiny greeting request and response.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 16384,
    nodes: 256,
    depth: 8,
};
/// Maximum UTF-8 bytes in a caller's name.
pub const MAX_NAME_BYTES: usize = 128;

/// Read-only greeting result. Queries are not certified application state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Greeting {
    /// Human-readable greeting from the installed executable.
    pub message: String,
}

#[cfg(feature = "storage")]
pub mod storage;

/// Optional runtime shard assignment and lookup for application hubs.
#[cfg(feature = "sharding")]
pub mod sharding;

/// Fleet-approved shard installation context.
#[cfg(feature = "topology")]
pub mod topology;
