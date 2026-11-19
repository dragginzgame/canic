//! Application-owned fenced recovery. A successful call returns the exact accepted command.
use serde::{Deserialize, Serialize};

/// Replicated application recovery contract.
pub const METHOD: u64 = 83;
/// Fixed command and acknowledgement ceiling.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 1024,
    nodes: 24,
    depth: 4,
};

/// Commands require the application's retained recovery authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Command {
    /// Close public writes at this fresh generation.
    Fence(u64),
    /// Replace the restored checkpoint fence while keeping public writes closed.
    Accept {
        /// Generation present in the original checkpoint.
        checkpoint: u64,
        /// Fresh generation assigned outside the rollback set.
        generation: u64,
    },
    /// Open writes for this exact generation.
    Open(u64),
}

#[cfg(feature = "method")]
impl crate::method::Contract for Command {
    const NAME: &'static str = "recovery";
    const ID: u64 = METHOD;
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = Self;
    type Output = Self;
}

/// Encode an original invocation or its matching acknowledgement.
/// # Errors
/// Returns a Wire error if the fixed command exceeds its bound.
pub fn encode(command: Command, direction: ic_wire::Direction) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &command,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction,
        },
        LIMITS,
    )
}
