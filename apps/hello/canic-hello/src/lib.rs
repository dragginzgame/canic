//! First hands-on greeting application. Upgrades replace code; no persistent state is claimed.

#[cfg(target_family = "wasm")]
mod endpoint;

/// Greeting embedded in the executable; edit this and rebuild to rehearse an upgrade.
#[cfg(not(feature = "greeting-v2"))]
pub const GREETING: &str = "Hello from Canic";
/// Second executable used to rehearse an upgrade without changing the canister ID.
#[cfg(feature = "greeting-v2")]
pub const GREETING: &str = "Hello again from Canic";

/// Compute the demo response after the endpoint has admitted its bounded name.
#[must_use]
pub fn greeting(name: &str) -> canic::Greeting {
    canic::Greeting {
        message: format!("{GREETING}, {name}!"),
    }
}
