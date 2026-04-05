use crate::dto::rpc::Response;
use canic_memory::serialize;

use self::{guard::ReplayPending, slot as replay_slot};

pub mod guard;
pub mod key;
pub mod slot;
pub mod ttl;

///
/// ReplayReserveError
/// Mechanical replay-reservation failures surfaced by ops replay reservation APIs.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayReserveError {
    CapacityReached { max_entries: usize },
}

///
/// ReplayCommitError
/// Mechanical replay-commit failures surfaced by ops replay commit APIs.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayCommitError {
    EncodeFailed(String),
}

/// reserve_root_replay
///
/// Persist a pending replay reservation marker before capability execution.
pub fn reserve_root_replay(
    pending: ReplayPending,
    max_entries: usize,
) -> Result<(), ReplayReserveError> {
    if !replay_slot::has_root_slot(pending.slot_key) && replay_slot::root_slot_len() >= max_entries
    {
        return Err(ReplayReserveError::CapacityReached { max_entries });
    }

    replay_slot::reserve_root_slot(pending);
    Ok(())
}

/// commit_root_replay
///
/// Persist canonical response bytes for an existing root replay reservation.
pub fn commit_root_replay(
    pending: ReplayPending,
    response: &Response,
) -> Result<(), ReplayCommitError> {
    let response_bytes = serialize::serialize(response)
        .map_err(|err| ReplayCommitError::EncodeFailed(err.to_string()))?;
    replay_slot::commit_root_slot(pending, response_bytes);
    Ok(())
}

/// abort_root_replay
///
/// Remove an in-flight replay reservation after failed capability execution.
pub fn abort_root_replay(pending: ReplayPending) {
    let _ = replay_slot::remove_root_slot(pending.slot_key);
}
