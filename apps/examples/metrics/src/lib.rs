//! Ordinary updates measured by Canic and observed by their controller.

use canic::{method::Contract, readiness::State};

// A freshly installed or upgraded metrics canister can accept traffic immediately.
canic::mount_readiness_endpoint!(readiness);

/// UTF-8 payload accepted by a measured ping.
pub type Payload = canic::method::wire::Text<256>;

// ping
// Return the length in bytes of up to 256 UTF-8 bytes.
// limits : default
#[canic::update(contract = Ping)]
fn ping(payload: Payload) -> u32 {
    payload.as_str().len() as u32
}

// checksum
// Compute a rolling checksum over up to 256 UTF-8 bytes.
// limits : default
#[canic::update(contract = Checksum)]
fn checksum(payload: Payload) -> u32 {
    payload.as_str().bytes().fold(0u32, |sum, byte| {
        sum.wrapping_mul(31).wrapping_add(u32::from(byte))
    })
}

///
/// Ping
/// Wire method for one measured request.
///

pub struct Ping;

impl Contract for Ping {
    const NAME: &'static str = "ping";
    const ID: u64 = canic::method::wire::method_id(Self::NAME);
    type Input = Payload;
    type Output = u32;
}

///
/// Checksum
/// Wire method for computing a payload checksum.
///

pub struct Checksum;

impl Contract for Checksum {
    const NAME: &'static str = "checksum";
    const ID: u64 = canic::method::wire::method_id(Self::NAME);
    type Input = Payload;
    type Output = u32;
}

fn readiness() -> State {
    State::Ready
}
