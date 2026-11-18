//! The starter's public greeting and readiness endpoints.
#![cfg(target_arch = "wasm32")]

canic::mount_greeting_endpoints!(crate::greeting);
canic::mount_readiness_endpoint!(crate::readiness);

fn greeting(name: &str) -> canic::Greeting {
    canic::Greeting {
        message: format!("Hello from Canic, {name}!"),
    }
}

fn readiness() -> canic::readiness::State {
    canic::readiness::State::Ready
}
