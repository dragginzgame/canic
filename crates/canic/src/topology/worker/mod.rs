//! Worker allocation instructions retained inside the approved topology install frame.
use ic_wire::{Direction, Encoding, Limits};
use std::num::NonZeroU64;

const FRAME: Encoding = Encoding::Frame {
    method: ic_wire::method_id("canic.worker_install.v2"),
    direction: Direction::Request,
};
const LIMITS: Limits = Limits {
    bytes: 128,
    nodes: 16,
    allocation: 256,
    depth: 4,
};

/// Encode the minimum observed cycle balance for reclaiming this pool's workers.
/// # Errors
/// Returns the Wire error if the grant cannot be encoded.
pub fn reuse_policy(minimum: NonZeroU64) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(&minimum.get(), FRAME, LIMITS)
}

/// Recover the approved floor. Unrecognized and historical grants confer no reuse permission.
#[must_use]
pub fn minimum_reuse_cycles(bytes: &[u8]) -> Option<NonZeroU64> {
    NonZeroU64::new(ic_wire::decode(bytes, FRAME, LIMITS).ok()?)
}

#[cfg(test)]
mod tests;
