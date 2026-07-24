//! Module: ops::storage::placement::binding
//!
//! Responsibility: provide deterministic binding registry claim and binding operations.
//! Does not own: placement policy, provisioning workflow, or endpoint DTOs.
//! Boundary: storage ops facade over stable binding registry records.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    dto::placement::binding::{
        PlacementBindingRegistryEntry, PlacementBindingRegistryResponse,
        PlacementBindingStatusResponse,
    },
    ops::{prelude::*, storage::StorageOpsError},
    storage::stable::placement_binding::{
        PlacementBindingEntryRecord, PlacementBindingKey, PlacementBindingRegistry,
    },
};
use thiserror::Error as ThisError;

///
/// PlacementBindingRegistryOpsError
///
/// Typed storage failure for binding registry claim and binding operations.
///

#[derive(Debug, ThisError)]
pub enum PlacementBindingRegistryOpsError {
    #[error("invalid binding key: {0}")]
    InvalidKey(String),

    #[error("binding key '{key_value}' in pool '{pool}' already bound to instance {pid}")]
    KeyBound {
        pool: String,
        key_value: String,
        pid: Principal,
    },

    #[error(
        "binding key '{key_value}' in pool '{pool}' is pending for provisional child {expected}, not {actual}"
    )]
    ProvisionalPidMismatch {
        pool: String,
        key_value: String,
        expected: Principal,
        actual: Principal,
    },
}

impl From<PlacementBindingRegistryOpsError> for InternalError {
    fn from(err: PlacementBindingRegistryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

///
/// PlacementBindingRegistryOps
///
/// Storage-ops facade for binding registry claim and binding operations.
///

pub struct PlacementBindingRegistryOps;

///
/// PlacementBindingEntryState
///
/// Internal binding registry state view used by placement workflows.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementBindingEntryState {
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
/// PlacementBindingPendingClaim
///
/// Pending binding claim returned when a caller owns a logical key reservation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacementBindingPendingClaim {
    pub claim_id: u64,
    pub owner_pid: Principal,
    pub created_at: u64,
}

///
/// PlacementBindingClaimResult
///
/// Result of attempting to claim one logical binding key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementBindingClaimResult {
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
    Claimed(PlacementBindingPendingClaim),
}

///
/// PlacementBindingReleaseResult
///
/// Result of attempting to release a stale pending binding claim.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementBindingReleaseResult {
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

impl PlacementBindingRegistryOps {
    pub const PENDING_TTL_SECS: u64 = 300;

    // Claim one logical key for in-progress instance creation before async work begins.
    pub fn claim_pending(
        pool: &str,
        key_value: &str,
        owner_pid: Principal,
        claim_id: u64,
        created_at: u64,
    ) -> Result<PlacementBindingClaimResult, InternalError> {
        let key = PlacementBindingKey::try_new(pool, key_value)
            .map_err(PlacementBindingRegistryOpsError::InvalidKey)?;

        match PlacementBindingRegistry::get(&key) {
            Some(PlacementBindingEntryRecord::Bound {
                instance_pid,
                bound_at,
            }) => Ok(PlacementBindingClaimResult::Bound {
                instance_pid,
                bound_at,
            }),

            Some(PlacementBindingEntryRecord::Pending {
                claim_id,
                owner_pid: existing_owner_pid,
                created_at: existing_created_at,
                provisional_pid,
            }) => Ok(PlacementBindingClaimResult::PendingExisting {
                claim_id,
                owner_pid: existing_owner_pid,
                created_at: existing_created_at,
                provisional_pid,
            }),

            None => {
                PlacementBindingRegistry::insert(
                    key,
                    PlacementBindingEntryRecord::Pending {
                        claim_id,
                        owner_pid,
                        created_at,
                        provisional_pid: None,
                    },
                );

                Ok(PlacementBindingClaimResult::Claimed(
                    PlacementBindingPendingClaim {
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
    pub fn lookup_state(pool: &str, key_value: &str) -> Option<PlacementBindingEntryState> {
        let key = PlacementBindingKey::try_new(pool, key_value).ok()?;
        PlacementBindingRegistry::get(&key).map(entry_to_state)
    }

    // Attach the created child pid only if the caller still owns the current pending claim.
    pub fn set_provisional_pid_if_claim_matches(
        pool: &str,
        key_value: &str,
        expected_claim_id: u64,
        provisional_pid: Principal,
    ) -> Result<bool, InternalError> {
        let key = PlacementBindingKey::try_new(pool, key_value)
            .map_err(PlacementBindingRegistryOpsError::InvalidKey)?;
        let entry = PlacementBindingRegistry::get(&key);

        let Some(PlacementBindingEntryRecord::Pending {
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

        PlacementBindingRegistry::insert(
            key,
            PlacementBindingEntryRecord::Pending {
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
        let key = PlacementBindingKey::try_new(pool, key_value).ok()?;
        match PlacementBindingRegistry::get(&key) {
            Some(PlacementBindingEntryRecord::Bound { instance_pid, .. }) => Some(instance_pid),
            Some(PlacementBindingEntryRecord::Pending { .. }) | None => None,
        }
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<PlacementBindingStatusResponse> {
        let key = PlacementBindingKey::try_new(pool, key_value).ok()?;
        PlacementBindingRegistry::get(&key).map(entry_to_response)
    }

    // Release one stale pending claim so recovery/admin paths can clear dead keys.
    pub fn release_stale_pending_if_claim_matches(
        pool: &str,
        key_value: &str,
        expected_claim_id: u64,
        now: u64,
    ) -> Result<PlacementBindingReleaseResult, InternalError> {
        let key = PlacementBindingKey::try_new(pool, key_value)
            .map_err(PlacementBindingRegistryOpsError::InvalidKey)?;

        let Some(entry) = PlacementBindingRegistry::get(&key) else {
            return Ok(PlacementBindingReleaseResult::Missing);
        };

        match entry {
            PlacementBindingEntryRecord::Bound {
                instance_pid,
                bound_at,
            } => Ok(PlacementBindingReleaseResult::Bound {
                instance_pid,
                bound_at,
            }),

            PlacementBindingEntryRecord::Pending {
                claim_id,
                owner_pid,
                created_at,
                provisional_pid,
            } if claim_id != expected_claim_id
                || !is_pending_stale(now, created_at)
                || provisional_pid.is_none() =>
            {
                Ok(PlacementBindingReleaseResult::PendingRetained {
                    owner_pid,
                    created_at,
                    provisional_pid,
                })
            }

            PlacementBindingEntryRecord::Pending {
                claim_id: _,
                owner_pid,
                created_at,
                provisional_pid,
            } => {
                let _ = PlacementBindingRegistry::remove(&key);

                Ok(PlacementBindingReleaseResult::ReleasedStalePending {
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
        let key = PlacementBindingKey::try_new(pool, key_value)
            .map_err(PlacementBindingRegistryOpsError::InvalidKey)?;

        match PlacementBindingRegistry::get(&key) {
            Some(PlacementBindingEntryRecord::Bound { instance_pid, .. })
                if instance_pid == pid =>
            {
                Ok(())
            }

            Some(PlacementBindingEntryRecord::Bound { instance_pid, .. }) => {
                Err(PlacementBindingRegistryOpsError::KeyBound {
                    pool: pool.to_string(),
                    key_value: key_value.to_string(),
                    pid: instance_pid,
                }
                .into())
            }

            Some(PlacementBindingEntryRecord::Pending {
                provisional_pid: Some(expected_pid),
                ..
            }) if expected_pid != pid => {
                Err(PlacementBindingRegistryOpsError::ProvisionalPidMismatch {
                    pool: pool.to_string(),
                    key_value: key_value.to_string(),
                    expected: expected_pid,
                    actual: pid,
                }
                .into())
            }

            Some(PlacementBindingEntryRecord::Pending { .. }) | None => {
                PlacementBindingRegistry::insert(
                    key,
                    PlacementBindingEntryRecord::Bound {
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
        let key = PlacementBindingKey::try_new(pool, key_value)
            .map_err(PlacementBindingRegistryOpsError::InvalidKey)?;

        match PlacementBindingRegistry::get(&key) {
            Some(PlacementBindingEntryRecord::Pending {
                claim_id,
                provisional_pid: Some(expected_pid),
                ..
            }) if claim_id == expected_claim_id && expected_pid != pid => {
                Err(PlacementBindingRegistryOpsError::ProvisionalPidMismatch {
                    pool: pool.to_string(),
                    key_value: key_value.to_string(),
                    expected: expected_pid,
                    actual: pid,
                }
                .into())
            }

            Some(PlacementBindingEntryRecord::Pending { claim_id, .. })
                if claim_id != expected_claim_id =>
            {
                Ok(false)
            }

            Some(PlacementBindingEntryRecord::Pending { .. }) => {
                PlacementBindingRegistry::insert(
                    key,
                    PlacementBindingEntryRecord::Bound {
                        instance_pid: pid,
                        bound_at,
                    },
                );
                Ok(true)
            }

            Some(PlacementBindingEntryRecord::Bound { .. }) | None => Ok(false),
        }
    }

    #[must_use]
    pub fn entries_response() -> PlacementBindingRegistryResponse {
        let entries = PlacementBindingRegistry::export()
            .entries
            .into_iter()
            .map(|record| PlacementBindingRegistryEntry {
                pool: record.key.pool.to_string(),
                key_value: record.key.key_value.to_string(),
                status: entry_to_response(record.entry),
            })
            .collect();

        PlacementBindingRegistryResponse(entries)
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        PlacementBindingRegistry::clear();
    }
}

// Decide whether an in-progress claim can be reclaimed by a later caller.
const fn is_pending_stale(now: u64, created_at: u64) -> bool {
    now.saturating_sub(created_at) > PlacementBindingRegistryOps::PENDING_TTL_SECS
}

// Convert the storage-owned entry state into the public placement DTO shape.
const fn entry_to_response(entry: PlacementBindingEntryRecord) -> PlacementBindingStatusResponse {
    match entry {
        PlacementBindingEntryRecord::Pending {
            claim_id: _,
            owner_pid,
            created_at,
            provisional_pid,
        } => PlacementBindingStatusResponse::Pending {
            owner_pid,
            created_at,
            provisional_pid,
        },
        PlacementBindingEntryRecord::Bound {
            instance_pid,
            bound_at,
        } => PlacementBindingStatusResponse::Bound {
            instance_pid,
            bound_at,
        },
    }
}

const fn entry_to_state(entry: PlacementBindingEntryRecord) -> PlacementBindingEntryState {
    match entry {
        PlacementBindingEntryRecord::Pending {
            claim_id,
            owner_pid,
            created_at,
            provisional_pid,
        } => PlacementBindingEntryState::Pending {
            claim_id,
            owner_pid,
            created_at,
            provisional_pid,
        },
        PlacementBindingEntryRecord::Bound {
            instance_pid,
            bound_at,
        } => PlacementBindingEntryState::Bound {
            instance_pid,
            bound_at,
        },
    }
}
