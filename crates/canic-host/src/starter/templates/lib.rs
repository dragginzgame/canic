//! Application-owned greeting. Root management code lives in the Canic dependency.
#[cfg(target_family = "wasm")]
mod endpoint;

/// Edit this greeting and rebuild the application artifact.
pub const GREETING: &str = "Hello from Canic";

/// Produce a greeting after the endpoint admits a bounded name.
#[must_use]
pub fn greeting(name: &str) -> canic::Greeting {
    canic::Greeting {
        message: format!("{GREETING}, {name}!"),
    }
}
