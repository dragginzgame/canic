use crate::dto::rpc::Response;
use candid::encode_one;

use self::{guard::ReplayPending, slot as replay_slot};

pub mod guard;
pub mod key;
pub mod slot;
pub mod ttl;

///
/// ReplayCommitError
/// Mechanical replay-commit failures surfaced by ops replay commit APIs.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayCommitError {
    CapacityReached { max_entries: usize },
    EncodeFailed(String),
}

/// commit_root_replay
///
/// Persist a fresh root replay reservation and canonical response payload bytes.
pub fn commit_root_replay(
    pending: ReplayPending,
    response: &Response,
    max_entries: usize,
) -> Result<(), ReplayCommitError> {
    if replay_slot::root_slot_len() >= max_entries {
        return Err(ReplayCommitError::CapacityReached { max_entries });
    }

    let response_candid =
        encode_one(response).map_err(|err| ReplayCommitError::EncodeFailed(err.to_string()))?;
    replay_slot::commit_root_slot(pending, response_candid);
    Ok(())
}
