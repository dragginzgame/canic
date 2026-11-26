//! Stateless starter canister.

use serde::{Deserialize, Serialize};

// greet
// Greet a visitor whose name is at most 128 UTF-8 bytes.
// limits : Greet
#[canic::query(contract = Greet)]
fn greet(name: String) -> Greeting {
    greeting(&name)
}

// probe
// Return the greeting for a name of up to 128 UTF-8 bytes through a replicated update.
// limits : Probe
#[canic::update(contract = Probe)]
fn probe(name: String) -> Greeting {
    greeting(&name)
}

canic::mount_readiness_endpoint!(readiness);

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
#[cfg_attr(feature = "inspection", derive(canic::method::wire::typescript::Wire))]
#[cfg_attr(feature = "inspection", wire(crate = "canic::method::wire", fixed))]
pub struct Greeting {
    /// Human-readable greeting from the installed executable.
    pub message: String,
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

fn greeting(name: &str) -> Greeting {
    assert!(name.len() <= MAX_NAME_BYTES, "name exceeds 128 UTF-8 bytes");
    Greeting {
        message: format!("Hello from Canic, {name}!"),
    }
}

fn readiness() -> canic::readiness::State {
    canic::readiness::State::Ready
}
