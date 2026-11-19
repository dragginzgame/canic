//! Stateless CLI qualification actor for calls, metrics and compatible upgrades.

use serde::{Deserialize, Serialize};

// Report readiness for the installed CLI fixture.
canic::mount_readiness_endpoint!(crate::readiness);

// Sample the installed canister clock.
canic::mount_metrics_endpoint!(crate::snapshot);

// greet
// Greet a visitor with a name of up to 128 UTF-8 bytes.
// limits : LIMITS
#[canic::query(contract = Greet)]
fn greet(name: String) -> Greeting {
    greeting(&name)
}

// probe
// Return the replicated greeting for the same bounded name.
// limits : LIMITS
#[canic::update(contract = Probe)]
fn probe(name: String) -> Greeting {
    greeting(&name)
}

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

fn readiness() -> canic::readiness::State {
    // This actor has no persistent state or lifecycle to open.
    canic::readiness::State::Ready
}

fn snapshot() -> canic::metrics::Snapshot {
    canic::metrics::Snapshot {
        observed_at_ns: ic_platform_runtime::system::time(),
        timers: None,
    }
}

/// Wire contract for the `greet` endpoint.
struct Greet;
impl canic::method::Contract for Greet {
    const NAME: &'static str = "greet";
    const ID: u64 = GREET;
    const REQUEST: canic::method::wire::Limits = LIMITS;
    type Input = String;
    type Output = Greeting;
}

/// Wire contract for the `probe` endpoint.
struct Probe;
impl canic::method::Contract for Probe {
    const NAME: &'static str = "probe";
    const ID: u64 = GREET;
    const REQUEST: canic::method::wire::Limits = LIMITS;
    type Input = String;
    type Output = Greeting;
}
