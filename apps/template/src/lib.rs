//! Stateless starter canister.

mod endpoints;

use serde::{Deserialize, Serialize};

/// Identity of the greeting RPC, including its Wire request/response envelope.
pub const GREET: u64 = 1;
/// Shared admission policy for the tiny greeting request and response.
pub const LIMITS: canic::method::wire::Limits = canic::method::wire::Limits {
    bytes: 4096,
    allocation: 16384,
    nodes: 256,
    depth: 8,
};
/// Maximum UTF-8 bytes in a caller's name.
pub(crate) const MAX_NAME_BYTES: usize = 128;

/// Read-only greeting result. Queries are not certified application state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Greeting {
    /// Human-readable greeting from the installed executable.
    pub message: String,
}
