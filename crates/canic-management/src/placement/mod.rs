//! Pure placement decisions shared by the installed owner and its regression tests.

#[cfg(test)]
mod tests;

use canic::sharding::{Allocation, Placement};
use canic_management_protocol::{Failure, Phase, Status, children::ChildId};
use sha2::{Digest, Sha256};

pub(crate) fn resolve(
    allocation: Allocation,
    principal: Option<&str>,
    observed_generation: Option<[u8; 32]>,
    status: &Status,
) -> Placement {
    if observed_generation != Some(allocation.generation)
        || principal.is_some_and(|id| status.worker.as_deref() != Some(id))
        || status
            .failure
            .as_ref()
            .is_some_and(|failure| *failure != Failure::Uncertain)
        || matches!(
            status.phase,
            Phase::Empty | Phase::Expired | Phase::ReserveBlocked
        )
    {
        return Placement::Unavailable(allocation, principal.map(str::to_owned));
    }
    if status.failure == Some(Failure::Uncertain) {
        return Placement::Pending(allocation);
    }
    match (status.phase, principal) {
        (Phase::Ready, Some(id)) => Placement::Assigned(allocation, id.to_owned()),
        (Phase::Ready, None) => Placement::Unavailable(allocation, None),
        _ => Placement::Pending(allocation),
    }
}

// Legacy HRW takes the first eight digest bytes, big endian. A strict comparison
// retains the first candidate when scores tie.
pub(crate) fn score(parts: &[&[u8]]) -> u64 {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update(part);
    }
    u64::from_be_bytes(hash.finalize()[..8].try_into().unwrap())
}

pub(crate) fn select(best: &mut Option<(u64, ChildId)>, score: u64, slot: ChildId) {
    if best.is_none_or(|(previous, _)| score > previous) {
        *best = Some((score, slot));
    }
}
