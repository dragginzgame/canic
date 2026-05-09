use crate::{
    InternalError,
    dto::placement::directory::{
        DirectoryEntryStatusResponse, DirectoryRegistryEntry, DirectoryRegistryResponse,
    },
    ops::{prelude::*, storage::StorageOpsError},
    storage::stable::directory::{DirectoryEntryRecord, DirectoryKey, DirectoryRegistry},
};
use thiserror::Error as ThisError;

///
/// DirectoryRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum DirectoryRegistryOpsError {
    #[error("invalid directory key: {0}")]
    InvalidKey(String),

    #[error("directory key '{key_value}' in pool '{pool}' already bound to instance {pid}")]
    KeyBound {
        pool: String,
        key_value: String,
        pid: Principal,
    },

    #[error(
        "directory key '{key_value}' in pool '{pool}' is pending for provisional child {expected}, not {actual}"
    )]
    ProvisionalPidMismatch {
        pool: String,
        key_value: String,
        expected: Principal,
        actual: Principal,
    },
}

impl From<DirectoryRegistryOpsError> for InternalError {
    fn from(err: DirectoryRegistryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

///
/// DirectoryRegistryOps
///

pub struct DirectoryRegistryOps;

///
/// DirectoryEntryState
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectoryEntryState {
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
/// DirectoryPendingClaim
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryPendingClaim {
    pub claim_id: u64,
    pub owner_pid: Principal,
    pub created_at: u64,
}

///
/// DirectoryClaimResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectoryClaimResult {
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    PendingFresh {
        claim_id: u64,
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Claimed(DirectoryPendingClaim),
}

///
/// DirectoryReleaseResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectoryReleaseResult {
    Missing,
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    PendingCurrent {
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

impl DirectoryRegistryOps {
    pub const PENDING_TTL_SECS: u64 = 300;

    // Claim one logical key for in-progress instance creation before async work begins.
    pub fn claim_pending(
        pool: &str,
        key_value: &str,
        owner_pid: Principal,
        claim_id: u64,
        created_at: u64,
    ) -> Result<DirectoryClaimResult, InternalError> {
        let key = DirectoryKey::try_new(pool, key_value)
            .map_err(DirectoryRegistryOpsError::InvalidKey)?;

        match DirectoryRegistry::get(&key) {
            Some(DirectoryEntryRecord::Bound {
                instance_pid,
                bound_at,
            }) => Ok(DirectoryClaimResult::Bound {
                instance_pid,
                bound_at,
            }),

            Some(DirectoryEntryRecord::Pending {
                claim_id,
                owner_pid: existing_owner_pid,
                created_at: existing_created_at,
                provisional_pid,
            }) if !is_pending_stale(created_at, existing_created_at) => {
                Ok(DirectoryClaimResult::PendingFresh {
                    claim_id,
                    owner_pid: existing_owner_pid,
                    created_at: existing_created_at,
                    provisional_pid,
                })
            }

            Some(DirectoryEntryRecord::Pending { .. }) | None => {
                DirectoryRegistry::insert(
                    key,
                    DirectoryEntryRecord::Pending {
                        claim_id,
                        owner_pid,
                        created_at,
                        provisional_pid: None,
                    },
                );

                Ok(DirectoryClaimResult::Claimed(DirectoryPendingClaim {
                    claim_id,
                    owner_pid,
                    created_at,
                }))
            }
        }
    }

    // Read one entry with its internal claim state for workflow classification.
    #[must_use]
    pub fn lookup_state(pool: &str, key_value: &str) -> Option<DirectoryEntryState> {
        let key = DirectoryKey::try_new(pool, key_value).ok()?;
        DirectoryRegistry::get(&key).map(entry_to_state)
    }

    // Attach the created child pid only if the caller still owns the current pending claim.
    pub fn set_provisional_pid_if_claim_matches(
        pool: &str,
        key_value: &str,
        expected_claim_id: u64,
        provisional_pid: Principal,
    ) -> Result<bool, InternalError> {
        let key = DirectoryKey::try_new(pool, key_value)
            .map_err(DirectoryRegistryOpsError::InvalidKey)?;
        let entry = DirectoryRegistry::get(&key);

        let Some(DirectoryEntryRecord::Pending {
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

        DirectoryRegistry::insert(
            key,
            DirectoryEntryRecord::Pending {
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
        let key = DirectoryKey::try_new(pool, key_value).ok()?;
        match DirectoryRegistry::get(&key) {
            Some(DirectoryEntryRecord::Bound { instance_pid, .. }) => Some(instance_pid),
            Some(DirectoryEntryRecord::Pending { .. }) | None => None,
        }
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<DirectoryEntryStatusResponse> {
        let key = DirectoryKey::try_new(pool, key_value).ok()?;
        DirectoryRegistry::get(&key).map(entry_to_response)
    }

    // Release one stale pending claim so recovery/admin paths can clear dead keys.
    pub fn release_stale_pending_if_claim_matches(
        pool: &str,
        key_value: &str,
        expected_claim_id: u64,
        now: u64,
    ) -> Result<DirectoryReleaseResult, InternalError> {
        let key = DirectoryKey::try_new(pool, key_value)
            .map_err(DirectoryRegistryOpsError::InvalidKey)?;

        let Some(entry) = DirectoryRegistry::get(&key) else {
            return Ok(DirectoryReleaseResult::Missing);
        };

        match entry {
            DirectoryEntryRecord::Bound {
                instance_pid,
                bound_at,
            } => Ok(DirectoryReleaseResult::Bound {
                instance_pid,
                bound_at,
            }),

            DirectoryEntryRecord::Pending {
                claim_id,
                owner_pid,
                created_at,
                provisional_pid,
            } if claim_id != expected_claim_id || !is_pending_stale(now, created_at) => {
                Ok(DirectoryReleaseResult::PendingCurrent {
                    owner_pid,
                    created_at,
                    provisional_pid,
                })
            }

            DirectoryEntryRecord::Pending {
                claim_id: _,
                owner_pid,
                created_at,
                provisional_pid,
            } => {
                let _ = DirectoryRegistry::remove(&key);

                Ok(DirectoryReleaseResult::ReleasedStalePending {
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
        let key = DirectoryKey::try_new(pool, key_value)
            .map_err(DirectoryRegistryOpsError::InvalidKey)?;

        match DirectoryRegistry::get(&key) {
            Some(DirectoryEntryRecord::Bound { instance_pid, .. }) if instance_pid == pid => Ok(()),

            Some(DirectoryEntryRecord::Bound { instance_pid, .. }) => {
                Err(DirectoryRegistryOpsError::KeyBound {
                    pool: pool.to_string(),
                    key_value: key_value.to_string(),
                    pid: instance_pid,
                }
                .into())
            }

            Some(DirectoryEntryRecord::Pending {
                provisional_pid: Some(expected_pid),
                ..
            }) if expected_pid != pid => Err(DirectoryRegistryOpsError::ProvisionalPidMismatch {
                pool: pool.to_string(),
                key_value: key_value.to_string(),
                expected: expected_pid,
                actual: pid,
            }
            .into()),

            Some(DirectoryEntryRecord::Pending { .. }) | None => {
                DirectoryRegistry::insert(
                    key,
                    DirectoryEntryRecord::Bound {
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
        let key = DirectoryKey::try_new(pool, key_value)
            .map_err(DirectoryRegistryOpsError::InvalidKey)?;

        match DirectoryRegistry::get(&key) {
            Some(DirectoryEntryRecord::Pending {
                claim_id,
                provisional_pid: Some(expected_pid),
                ..
            }) if claim_id == expected_claim_id && expected_pid != pid => {
                Err(DirectoryRegistryOpsError::ProvisionalPidMismatch {
                    pool: pool.to_string(),
                    key_value: key_value.to_string(),
                    expected: expected_pid,
                    actual: pid,
                }
                .into())
            }

            Some(DirectoryEntryRecord::Pending { claim_id, .. })
                if claim_id != expected_claim_id =>
            {
                Ok(false)
            }

            Some(DirectoryEntryRecord::Pending { .. }) => {
                DirectoryRegistry::insert(
                    key,
                    DirectoryEntryRecord::Bound {
                        instance_pid: pid,
                        bound_at,
                    },
                );
                Ok(true)
            }

            Some(DirectoryEntryRecord::Bound { .. }) | None => Ok(false),
        }
    }

    #[must_use]
    pub fn entries_response() -> DirectoryRegistryResponse {
        let entries = DirectoryRegistry::export()
            .entries
            .into_iter()
            .map(|(key, entry)| DirectoryRegistryEntry {
                pool: key.pool.to_string(),
                key_value: key.key_value.to_string(),
                status: entry_to_response(entry),
            })
            .collect();

        DirectoryRegistryResponse(entries)
    }

    #[cfg(test)]
    pub(crate) fn clear_for_test() {
        DirectoryRegistry::clear();
    }
}

// Decide whether an in-progress claim can be reclaimed by a later caller.
const fn is_pending_stale(now: u64, created_at: u64) -> bool {
    now.saturating_sub(created_at) > DirectoryRegistryOps::PENDING_TTL_SECS
}

// Convert the storage-owned entry state into the public placement DTO shape.
const fn entry_to_response(entry: DirectoryEntryRecord) -> DirectoryEntryStatusResponse {
    match entry {
        DirectoryEntryRecord::Pending {
            claim_id: _,
            owner_pid,
            created_at,
            provisional_pid,
        } => DirectoryEntryStatusResponse::Pending {
            owner_pid,
            created_at,
            provisional_pid,
        },
        DirectoryEntryRecord::Bound {
            instance_pid,
            bound_at,
        } => DirectoryEntryStatusResponse::Bound {
            instance_pid,
            bound_at,
        },
    }
}

const fn entry_to_state(entry: DirectoryEntryRecord) -> DirectoryEntryState {
    match entry {
        DirectoryEntryRecord::Pending {
            claim_id,
            owner_pid,
            created_at,
            provisional_pid,
        } => DirectoryEntryState::Pending {
            claim_id,
            owner_pid,
            created_at,
            provisional_pid,
        },
        DirectoryEntryRecord::Bound {
            instance_pid,
            bound_at,
        } => DirectoryEntryState::Bound {
            instance_pid,
            bound_at,
        },
    }
}

#[cfg(test)]
mod tests;
