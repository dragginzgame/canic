//! Small contracts and explicitly selected actor adapters.
//! No host dependencies or management surface enters a Wasm consumer.

/// Optional caller-bound Auth endpoints; lifecycle remains explicit.
#[cfg(feature = "auth")]
pub mod auth;

/// Optional Blob endpoint composition; no implicit lifecycle or service activation.
#[cfg(feature = "blob")]
pub mod blob;

/// Optional application-owned typed database endpoints.
#[cfg(feature = "db")]
pub mod db;

/// Passive operational reports; the optional `metrics` feature enables endpoint mounting.
pub mod metrics;

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
