//! First hands-on greeting application. Upgrades replace code; no persistent state is claimed.

mod actor;

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
pub const MAX_NAME_BYTES: usize = 128;

/// Read-only greeting result. Queries are not certified application state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Greeting {
    /// Human-readable greeting from the installed executable.
    pub message: String,
}

/// Greeting embedded in the executable; edit this and rebuild to rehearse an upgrade.
#[cfg(not(feature = "greeting-v2"))]
pub const GREETING: &str = "Hello from Canic";
/// Second executable used to rehearse an upgrade without changing the canister ID.
#[cfg(feature = "greeting-v2")]
pub const GREETING: &str = "Hello again from Canic";

/// Compute the reply for a bounded UTF-8 name.
/// # Panics
/// Refuses names over 128 UTF-8 bytes.
#[must_use]
pub fn greeting(name: &str) -> Greeting {
    assert!(name.len() <= MAX_NAME_BYTES, "name exceeds 128 UTF-8 bytes");
    Greeting {
        message: format!("{GREETING}, {name}!"),
    }
}
