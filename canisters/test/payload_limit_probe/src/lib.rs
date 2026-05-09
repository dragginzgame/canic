#![expect(clippy::unused_async)]
use canic::{Error, ids::CanisterRole, prelude::*};

const TEST_ROLE: CanisterRole = CanisterRole::new("test");

canic::start!(TEST_ROLE);
canic::cdk::export_candid_debug!();

// Provide an empty setup hook so `start!` can schedule user lifecycle work.
async fn canic_setup() {}

// Provide an empty install hook; payload tests only exercise update ingress.
async fn canic_install(_: Option<Vec<u8>>) {}

// Provide an empty upgrade hook for the required Canic lifecycle surface.
async fn canic_upgrade() {}

/// Echo payload length under the default update ingress limit.
#[canic_update]
fn default_echo(payload: String) -> Result<usize, Error> {
    Ok(payload.len())
}

/// Echo payload length under an explicit larger update ingress limit.
#[canic_update(payload(max_bytes = 32 * 1024))]
fn explicit_echo(payload: String) -> Result<usize, Error> {
    Ok(payload.len())
}

/// Echo payload length under an explicit limit and exported method name.
#[canic_update(name = "wire_named_echo", payload(max_bytes = 24 * 1024))]
fn named_echo(payload: String) -> Result<usize, Error> {
    Ok(payload.len())
}
