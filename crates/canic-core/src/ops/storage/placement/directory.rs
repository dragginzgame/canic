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

    #[error("directory key '{key_value}' in pool '{pool}' is not currently pending")]
    NotPending { pool: String, key_value: String },

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
/// DirectoryClaimResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectoryClaimResult {
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    PendingFresh {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Claimed {
        owner_pid: Principal,
        created_at: u64,
    },
}

impl DirectoryRegistryOps {
    pub const PENDING_TTL_SECS: u64 = 300;

    // Claim one logical key for in-progress instance creation before async work begins.
    pub fn claim_pending(
        pool: &str,
        key_value: &str,
        owner_pid: Principal,
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
                owner_pid: existing_owner_pid,
                created_at: existing_created_at,
                provisional_pid,
            }) if !is_pending_stale(created_at, existing_created_at) => {
                Ok(DirectoryClaimResult::PendingFresh {
                    owner_pid: existing_owner_pid,
                    created_at: existing_created_at,
                    provisional_pid,
                })
            }

            Some(DirectoryEntryRecord::Pending { .. }) | None => {
                DirectoryRegistry::insert(
                    key,
                    DirectoryEntryRecord::Pending {
                        owner_pid,
                        created_at,
                        provisional_pid: None,
                    },
                );

                Ok(DirectoryClaimResult::Claimed {
                    owner_pid,
                    created_at,
                })
            }
        }
    }

    // Attach the created child pid to an existing pending claim for later repair or finalize.
    pub fn set_provisional_pid(
        pool: &str,
        key_value: &str,
        provisional_pid: Principal,
    ) -> Result<(), InternalError> {
        let key = DirectoryKey::try_new(pool, key_value)
            .map_err(DirectoryRegistryOpsError::InvalidKey)?;
        let entry = DirectoryRegistry::get(&key);

        let Some(DirectoryEntryRecord::Pending {
            owner_pid,
            created_at,
            ..
        }) = entry
        else {
            return Err(DirectoryRegistryOpsError::NotPending {
                pool: pool.to_string(),
                key_value: key_value.to_string(),
            }
            .into());
        };

        DirectoryRegistry::insert(
            key,
            DirectoryEntryRecord::Pending {
                owner_pid,
                created_at,
                provisional_pid: Some(provisional_pid),
            },
        );

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn claim_pending_returns_bound_when_key_is_already_bound() {
        DirectoryRegistryOps::clear_for_test();

        let pid = p(1);
        DirectoryRegistryOps::bind("projects", "alpha", pid, 10).expect("initial bind");

        let result =
            DirectoryRegistryOps::claim_pending("projects", "alpha", p(9), 20).expect("claim");

        assert_eq!(
            result,
            DirectoryClaimResult::Bound {
                instance_pid: pid,
                bound_at: 10,
            }
        );
    }

    #[test]
    fn claim_pending_reclaims_stale_pending_entries() {
        DirectoryRegistryOps::clear_for_test();

        let owner_pid = p(1);
        let new_owner_pid = p(2);

        let first = DirectoryRegistryOps::claim_pending("projects", "alpha", owner_pid, 10)
            .expect("initial claim");
        assert_eq!(
            first,
            DirectoryClaimResult::Claimed {
                owner_pid,
                created_at: 10,
            }
        );

        let reclaimed = DirectoryRegistryOps::claim_pending(
            "projects",
            "alpha",
            new_owner_pid,
            10 + DirectoryRegistryOps::PENDING_TTL_SECS + 1,
        )
        .expect("stale claim should be reclaimed");

        assert_eq!(
            reclaimed,
            DirectoryClaimResult::Claimed {
                owner_pid: new_owner_pid,
                created_at: 10 + DirectoryRegistryOps::PENDING_TTL_SECS + 1,
            }
        );
    }

    #[test]
    fn bind_promotes_matching_pending_provisional_child() {
        DirectoryRegistryOps::clear_for_test();

        let owner_pid = p(1);
        let child_pid = p(2);

        DirectoryRegistryOps::claim_pending("projects", "alpha", owner_pid, 10)
            .expect("initial claim");
        DirectoryRegistryOps::set_provisional_pid("projects", "alpha", child_pid)
            .expect("attach provisional child");
        DirectoryRegistryOps::bind("projects", "alpha", child_pid, 20)
            .expect("bind should promote matching provisional child");

        assert_eq!(
            DirectoryRegistryOps::lookup_key("projects", "alpha"),
            Some(child_pid)
        );
    }

    #[test]
    fn lookup_entry_reports_pending_status() {
        DirectoryRegistryOps::clear_for_test();

        let owner_pid = p(1);
        DirectoryRegistryOps::claim_pending("projects", "alpha", owner_pid, 10)
            .expect("initial claim");

        assert_eq!(
            DirectoryRegistryOps::lookup_entry("projects", "alpha"),
            Some(DirectoryEntryStatusResponse::Pending {
                owner_pid,
                created_at: 10,
                provisional_pid: None,
            })
        );
    }

    #[test]
    fn bind_rejects_conflicting_provisional_child() {
        DirectoryRegistryOps::clear_for_test();

        DirectoryRegistryOps::claim_pending("projects", "alpha", p(1), 10).expect("initial claim");
        DirectoryRegistryOps::set_provisional_pid("projects", "alpha", p(2))
            .expect("attach provisional child");

        let err = DirectoryRegistryOps::bind("projects", "alpha", p(3), 20)
            .expect_err("conflicting provisional child should fail");

        assert!(err.to_string().contains("pending for provisional child"));
    }
}
