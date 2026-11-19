//! Mount the public greeting and report readiness for this stateless app.

use crate::{Greet, Greeting, MAX_NAME_BYTES, Probe};

/// Greet a visitor in a read-only query.
#[canic::query(contract = Greet)]
fn greet(name: String) -> Greeting {
    greeting(&name)
}

/// Return the same greeting through a replicated update.
#[canic::update(contract = Probe)]
fn probe(name: String) -> Greeting {
    greeting(&name)
}

canic::mount_readiness_endpoint!(crate::endpoints::readiness);

fn greeting(name: &str) -> Greeting {
    assert!(name.len() <= MAX_NAME_BYTES, "name exceeds 128 UTF-8 bytes");
    Greeting {
        message: format!("Hello from Canic, {name}!"),
    }
}

fn readiness() -> canic::readiness::State {
    canic::readiness::State::Ready
}
