//! Module: ops::storage::placement::index
//!
//! Responsibility: provide deterministic index registry claim and index operations.
//! Does not own: placement policy, provisioning workflow, or endpoint DTOs.
//! Boundary: storage ops facade over stable index registry records.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    dto::placement::index::{
        PlacementIndexRegistryEntry, PlacementIndexRegistryResponse, PlacementIndexStatusResponse,
    },
    ops::prelude::*,
    storage::stable::placement_index::{
        PlacementIndexEntryRecord, PlacementIndexKey, PlacementIndexRegistry,
    },
};
use thiserror::Error as ThisError;

///
/// PlacementIndexRegistryOpsError
///
/// Typed storage failure for index registry claim and index operations.
///

#[derive(Debug, ThisError)]
pub enum PlacementIndexRegistryOpsError {
    #[error("invalid index key: {0}")]
    InvalidKey(String),

    #[error("index key '{key_value}' in pool '{pool}' already bound to instance {pid}")]
    KeyBound {
        pool: String,
        key_value: String,
        pid: Principal,
    },

    #[error(
        "index key '{key_value}' in pool '{pool}' is pending for provisional child {expected}, not {actual}"
    )]
    ProvisionalPidMismatch {
        pool: String,
        key_value: String,
        expected: Principal,
        actual: Principal,
    },
}

impl From<PlacementIndexRegistryOpsError> for InternalError {
    fn from(err: PlacementIndexRegistryOpsError) -> Self {
        let code = match err {
            PlacementIndexRegistryOpsError::InvalidKey(_) => {
                crate::diagnostics::codes::SECURITY_INVALID
            }
            PlacementIndexRegistryOpsError::KeyBound { .. } => {
                crate::diagnostics::codes::SECURITY_INVALID_STATE
            }
            PlacementIndexRegistryOpsError::ProvisionalPidMismatch { .. } => {
                crate::diagnostics::codes::AUTHORITY_CONFLICT
            }
        };
        Self::public(code)
    }
}

///
/// PlacementIndexRegistryOps
///
/// Storage-ops facade for index registry claim and index operations.
///

pub struct PlacementIndexRegistryOps;

///
/// PlacementIndexEntryState
///
/// Internal index registry state view used by placement workflows.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementIndexEntryState {
    Pending {
        claim_id: u64,
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
}

///
/// PlacementIndexPendingClaim
///
/// Pending index claim returned when a caller owns a logical key reservation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacementIndexPendingClaim {
    pub claim_id: u64,
    pub owner_pid: Principal,
    pub created_at: u64,
}

///
/// PlacementIndexClaimResult
///
/// Result of attempting to claim one logical index key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementIndexClaimResult {
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    PendingExisting {
        claim_id: u64,
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Claimed(PlacementIndexPendingClaim),
}

///
/// PlacementIndexReleaseResult
///
/// Result of attempting to release a stale pending index claim.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementIndexReleaseResult {
    Missing,
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    PendingRetained {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    ReleasedStalePending {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
}

impl PlacementIndexRegistryOps {
    pub const PENDING_TTL_SECS: u64 = 300;

    // Claim one logical key for in-progress instance creation before async work begins.
    pub fn claim_pending(
        pool: &str,
        key_value: &str,
        owner_pid: Principal,
        claim_id: u64,
        created_at: u64,
    ) -> Result<PlacementIndexClaimResult, InternalError> {
        let key = PlacementIndexKey::try_new(pool, key_value)
            .map_err(PlacementIndexRegistryOpsError::InvalidKey)?;

        match PlacementIndexRegistry::get(&key) {
            Some(PlacementIndexEntryRecord::Bound {
                instance_pid,
                bound_at,
            }) => Ok(PlacementIndexClaimResult::Bound {
                instance_pid,
                bound_at,
            }),

            Some(PlacementIndexEntryRecord::Pending {
                claim_id,
                owner_pid: existing_owner_pid,
                created_at: existing_created_at,
                provisional_pid,
            }) => Ok(PlacementIndexClaimResult::PendingExisting {
                claim_id,
                owner_pid: existing_owner_pid,
                created_at: existing_created_at,
                provisional_pid,
            }),

            None => {
                PlacementIndexRegistry::insert(
                    key,
                    PlacementIndexEntryRecord::Pending {
                        claim_id,
                        owner_pid,
                        created_at,
                        provisional_pid: None,
                    },
                );

                Ok(PlacementIndexClaimResult::Claimed(
                    PlacementIndexPendingClaim {
                        claim_id,
                        owner_pid,
                        created_at,
                    },
                ))
            }
        }
    }

    // Read one entry with its internal claim state for workflow classification.
    #[must_use]
    pub fn lookup_state(pool: &str, key_value: &str) -> Option<PlacementIndexEntryState> {
        let key = PlacementIndexKey::try_new(pool, key_value).ok()?;
        PlacementIndexRegistry::get(&key).map(entry_to_state)
    }

    // Attach the created child pid only if the caller still owns the current pending claim.
    pub fn set_provisional_pid_if_claim_matches(
        pool: &str,
        key_value: &str,
        expected_claim_id: u64,
        provisional_pid: Principal,
    ) -> Result<bool, InternalError> {
        let key = PlacementIndexKey::try_new(pool, key_value)
            .map_err(PlacementIndexRegistryOpsError::InvalidKey)?;
        let entry = PlacementIndexRegistry::get(&key);

        let Some(PlacementIndexEntryRecord::Pending {
            claim_id,
            owner_pid,
            created_at,
            ..
        }) = entry
        else {
            return Ok(false);
        };

        if claim_id != expected_claim_id {
            return Ok(false);
        }

        PlacementIndexRegistry::insert(
            key,
            PlacementIndexEntryRecord::Pending {
                claim_id,
                owner_pid,
                created_at,
                provisional_pid: Some(provisional_pid),
            },
        );

        Ok(true)
    }

    #[must_use]
    pub fn lookup_key(pool: &str, key_value: &str) -> Option<Principal> {
        let key = PlacementIndexKey::try_new(pool, key_value).ok()?;
        match PlacementIndexRegistry::get(&key) {
            Some(PlacementIndexEntryRecord::Bound { instance_pid, .. }) => Some(instance_pid),
            Some(PlacementIndexEntryRecord::Pending { .. }) | None => None,
        }
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<PlacementIndexStatusResponse> {
        let key = PlacementIndexKey::try_new(pool, key_value).ok()?;
        PlacementIndexRegistry::get(&key).map(entry_to_response)
    }

    // Release one stale pending claim so recovery/admin paths can clear dead keys.
    pub fn release_stale_pending_if_claim_matches(
        pool: &str,
        key_value: &str,
        expected_claim_id: u64,
        now: u64,
    ) -> Result<PlacementIndexReleaseResult, InternalError> {
        let key = PlacementIndexKey::try_new(pool, key_value)
            .map_err(PlacementIndexRegistryOpsError::InvalidKey)?;

        let Some(entry) = PlacementIndexRegistry::get(&key) else {
            return Ok(PlacementIndexReleaseResult::Missing);
        };

        match entry {
            PlacementIndexEntryRecord::Bound {
                instance_pid,
                bound_at,
            } => Ok(PlacementIndexReleaseResult::Bound {
                instance_pid,
                bound_at,
            }),

            PlacementIndexEntryRecord::Pending {
                claim_id,
                owner_pid,
                created_at,
                provisional_pid,
            } if claim_id != expected_claim_id
                || !is_pending_stale(now, created_at)
                || provisional_pid.is_none() =>
            {
                Ok(PlacementIndexReleaseResult::PendingRetained {
                    owner_pid,
                    created_at,
                    provisional_pid,
                })
            }

            PlacementIndexEntryRecord::Pending {
                claim_id: _,
                owner_pid,
                created_at,
                provisional_pid,
            } => {
                let _ = PlacementIndexRegistry::remove(&key);

                Ok(PlacementIndexReleaseResult::ReleasedStalePending {
                    owner_pid,
                    created_at,
                    provisional_pid,
                })
            }
        }
    }

    // Finalize a resolved child into the canonical bound state.
    pub fn bind(
        pool: &str,
        key_value: &str,
        pid: Principal,
        bound_at: u64,
    ) -> Result<(), InternalError> {
        let key = PlacementIndexKey::try_new(pool, key_value)
            .map_err(PlacementIndexRegistryOpsError::InvalidKey)?;

        match PlacementIndexRegistry::get(&key) {
            Some(PlacementIndexEntryRecord::Bound { instance_pid, .. }) if instance_pid == pid => {
                Ok(())
            }

            Some(PlacementIndexEntryRecord::Bound { instance_pid, .. }) => {
                Err(PlacementIndexRegistryOpsError::KeyBound {
                    pool: pool.to_string(),
                    key_value: key_value.to_string(),
                    pid: instance_pid,
                }
                .into())
            }

            Some(PlacementIndexEntryRecord::Pending {
                provisional_pid: Some(expected_pid),
                ..
            }) if expected_pid != pid => {
                Err(PlacementIndexRegistryOpsError::ProvisionalPidMismatch {
                    pool: pool.to_string(),
                    key_value: key_value.to_string(),
                    expected: expected_pid,
                    actual: pid,
                }
                .into())
            }

            Some(PlacementIndexEntryRecord::Pending { .. }) | None => {
                PlacementIndexRegistry::insert(
                    key,
                    PlacementIndexEntryRecord::Bound {
                        instance_pid: pid,
                        bound_at,
                    },
                );
                Ok(())
            }
        }
    }

    // Finalize a created child only if the caller still owns the current pending claim.
    pub fn bind_if_claim_matches(
        pool: &str,
        key_value: &str,
        expected_claim_id: u64,
        pid: Principal,
        bound_at: u64,
    ) -> Result<bool, InternalError> {
        let key = PlacementIndexKey::try_new(pool, key_value)
            .map_err(PlacementIndexRegistryOpsError::InvalidKey)?;

        match PlacementIndexRegistry::get(&key) {
            Some(PlacementIndexEntryRecord::Pending {
                claim_id,
                provisional_pid: Some(expected_pid),
                ..
            }) if claim_id == expected_claim_id && expected_pid != pid => {
                Err(PlacementIndexRegistryOpsError::ProvisionalPidMismatch {
                    pool: pool.to_string(),
                    key_value: key_value.to_string(),
                    expected: expected_pid,
                    actual: pid,
                }
                .into())
            }

            Some(PlacementIndexEntryRecord::Pending { claim_id, .. })
                if claim_id != expected_claim_id =>
            {
                Ok(false)
            }

            Some(PlacementIndexEntryRecord::Pending { .. }) => {
                PlacementIndexRegistry::insert(
                    key,
                    PlacementIndexEntryRecord::Bound {
                        instance_pid: pid,
                        bound_at,
                    },
                );
                Ok(true)
            }

            Some(PlacementIndexEntryRecord::Bound { .. }) | None => Ok(false),
        }
    }

    #[must_use]
    pub fn entries_response() -> PlacementIndexRegistryResponse {
        let entries = PlacementIndexRegistry::export()
            .entries
            .into_iter()
            .map(|record| PlacementIndexRegistryEntry {
                pool: record.key.pool.to_string(),
                key_value: record.key.key_value.to_string(),
                status: entry_to_response(record.entry),
            })
            .collect();

        PlacementIndexRegistryResponse(entries)
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        PlacementIndexRegistry::clear();
    }
}

// Decide whether an in-progress claim can be reclaimed by a later caller.
const fn is_pending_stale(now: u64, created_at: u64) -> bool {
    now.saturating_sub(created_at) > PlacementIndexRegistryOps::PENDING_TTL_SECS
}

// Convert the storage-owned entry state into the public placement DTO shape.
const fn entry_to_response(entry: PlacementIndexEntryRecord) -> PlacementIndexStatusResponse {
    match entry {
        PlacementIndexEntryRecord::Pending {
            claim_id: _,
            owner_pid,
            created_at,
            provisional_pid,
        } => PlacementIndexStatusResponse::Pending {
            owner_pid,
            created_at,
            provisional_pid,
        },
        PlacementIndexEntryRecord::Bound {
            instance_pid,
            bound_at,
        } => PlacementIndexStatusResponse::Bound {
            instance_pid,
            bound_at,
        },
    }
}

const fn entry_to_state(entry: PlacementIndexEntryRecord) -> PlacementIndexEntryState {
    match entry {
        PlacementIndexEntryRecord::Pending {
            claim_id,
            owner_pid,
            created_at,
            provisional_pid,
        } => PlacementIndexEntryState::Pending {
            claim_id,
            owner_pid,
            created_at,
            provisional_pid,
        },
        PlacementIndexEntryRecord::Bound {
            instance_pid,
            bound_at,
        } => PlacementIndexEntryState::Bound {
            instance_pid,
            bound_at,
        },
    }
}
