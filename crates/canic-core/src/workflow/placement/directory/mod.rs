pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::{DirectoryConfig, DirectoryPool},
    dto::placement::directory::{DirectoryEntryStatusResponse, DirectoryRecoveryResponse},
    ids::CanisterRole,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        rpc::request::{CreateCanisterParent, RequestOps},
        storage::{
            children::CanisterChildrenOps,
            placement::directory::{
                DirectoryClaimResult, DirectoryEntryState, DirectoryPendingClaim,
                DirectoryRegistryOps, DirectoryReleaseResult,
            },
            registry::subnet::SubnetRegistryOps,
        },
    },
};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
enum DirectoryWorkflowError {
    #[error("directory placement is not configured for the current canister")]
    DirectoryDisabled,

    #[error("unknown directory pool '{requested}': configured pools: {available}")]
    UnknownPool {
        requested: String,
        available: String,
    },

    #[error("instance {0} is not a direct child of the current canister")]
    InstanceNotDirectChild(Principal),

    #[error("directory instance {pid} has role '{actual}', expected '{expected}'")]
    InstanceRoleMismatch {
        pid: Principal,
        expected: CanisterRole,
        actual: CanisterRole,
    },

    #[error("directory instance {0} is not present in the subnet registry")]
    RegistryEntryMissing(Principal),
}

impl From<DirectoryWorkflowError> for InternalError {
    fn from(err: DirectoryWorkflowError) -> Self {
        Self::domain(InternalErrorOrigin::Workflow, err.to_string())
    }
}

#[derive(Debug, Eq, PartialEq)]
enum DirectoryEntryClassification {
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    PendingFresh {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Repairable {
        claim_id: u64,
        provisional_pid: Principal,
    },
    NeedsCleanup {
        claim_id: u64,
        provisional_pid: Option<Principal>,
    },
}

pub struct DirectoryWorkflow;

static DIRECTORY_CLAIM_NONCE: AtomicU64 = AtomicU64::new(1);

impl DirectoryWorkflow {
    /// Resolve a bound instance for one key or create and bind a new one.
    pub async fn resolve_or_create(
        pool: &str,
        key_value: &str,
    ) -> Result<DirectoryEntryStatusResponse, InternalError> {
        let pool_cfg = Self::get_directory_pool_cfg(pool)?;
        let owner_pid = IcOps::metadata_entropy_canister();

        loop {
            let now = IcOps::now_secs();

            match Self::classify_entry(pool, key_value, &pool_cfg, now) {
                Some(DirectoryEntryClassification::Bound {
                    instance_pid,
                    bound_at,
                }) => {
                    return Ok(DirectoryEntryStatusResponse::Bound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(DirectoryEntryClassification::PendingFresh {
                    owner_pid,
                    created_at,
                    provisional_pid,
                }) => {
                    return Ok(DirectoryEntryStatusResponse::Pending {
                        owner_pid,
                        created_at,
                        provisional_pid,
                    });
                }

                Some(DirectoryEntryClassification::Repairable {
                    claim_id,
                    provisional_pid,
                }) => {
                    return Self::repair_stale_entry(
                        pool,
                        key_value,
                        claim_id,
                        provisional_pid,
                        now,
                    );
                }

                Some(DirectoryEntryClassification::NeedsCleanup {
                    claim_id,
                    provisional_pid,
                }) => {
                    Self::cleanup_stale_entry(pool, key_value, claim_id, provisional_pid).await?;
                }

                None => {
                    if let Some(status) =
                        Self::claim_and_create_instance(pool, key_value, &pool_cfg, owner_pid)
                            .await?
                    {
                        return Ok(status);
                    }
                }
            }
        }
    }

    /// Recover one directory entry by repairing a valid stale provisional child or
    /// releasing a dead pending claim.
    pub async fn recover_entry(
        pool: &str,
        key_value: &str,
    ) -> Result<DirectoryRecoveryResponse, InternalError> {
        let pool_cfg = Self::get_directory_pool_cfg(pool)?;
        loop {
            let now = IcOps::now_secs();

            match Self::classify_entry(pool, key_value, &pool_cfg, now) {
                None => return Ok(DirectoryRecoveryResponse::Missing),

                Some(DirectoryEntryClassification::Bound {
                    instance_pid,
                    bound_at,
                }) => {
                    return Ok(DirectoryRecoveryResponse::Bound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(DirectoryEntryClassification::PendingFresh {
                    owner_pid,
                    created_at,
                    provisional_pid,
                }) => {
                    return Ok(DirectoryRecoveryResponse::FreshPending {
                        owner_pid,
                        created_at,
                        provisional_pid,
                    });
                }

                Some(DirectoryEntryClassification::Repairable {
                    claim_id,
                    provisional_pid,
                }) => {
                    let repaired =
                        Self::repair_stale_entry(pool, key_value, claim_id, provisional_pid, now)?;

                    let DirectoryEntryStatusResponse::Bound {
                        instance_pid,
                        bound_at,
                    } = repaired
                    else {
                        return Err(InternalError::invariant(
                            InternalErrorOrigin::Workflow,
                            "directory stale repair returned non-bound status",
                        ));
                    };

                    return Ok(DirectoryRecoveryResponse::RepairedToBound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(DirectoryEntryClassification::NeedsCleanup {
                    claim_id,
                    provisional_pid,
                }) => {
                    if let Some(response) = Self::recover_cleanup_stale_entry(
                        pool,
                        key_value,
                        claim_id,
                        provisional_pid,
                    )
                    .await?
                    {
                        return Ok(response);
                    }
                }
            }
        }
    }

    /// Bind one logical directory key to a validated direct child instance.
    pub fn bind_instance(pool: &str, key_value: &str, pid: Principal) -> Result<(), InternalError> {
        let pool_cfg = Self::get_directory_pool_cfg(pool)?;
        Self::validate_bind_target(pid, &pool_cfg.canister_role)?;
        DirectoryRegistryOps::bind(pool, key_value, pid, IcOps::now_secs())
    }

    // Finalize one freshly created child using claim-matching writes so late async completions
    // cannot overwrite a newer claim after the key has been reclaimed.
    async fn finalize_created_instance(
        pool: &str,
        key_value: &str,
        claim: DirectoryPendingClaim,
        pid: Principal,
    ) -> Result<Option<DirectoryEntryStatusResponse>, InternalError> {
        if !DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
            pool,
            key_value,
            claim.claim_id,
            pid,
        )? {
            Self::recycle_abandoned_child(pid).await?;
            return Ok(None);
        }

        let bound_at = IcOps::now_secs();
        if !DirectoryRegistryOps::bind_if_claim_matches(
            pool,
            key_value,
            claim.claim_id,
            pid,
            bound_at,
        )? {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "directory claim lost between provisional attach and final bind",
            ));
        }

        Ok(Some(DirectoryEntryStatusResponse::Bound {
            instance_pid: pid,
            bound_at,
        }))
    }

    // Claim one logical key and, if this caller wins the claim, create and bind a new child.
    async fn claim_and_create_instance(
        pool: &str,
        key_value: &str,
        pool_cfg: &DirectoryPool,
        owner_pid: Principal,
    ) -> Result<Option<DirectoryEntryStatusResponse>, InternalError> {
        let now = IcOps::now_secs();
        let claim_id = new_claim_id();

        let claim =
            match DirectoryRegistryOps::claim_pending(pool, key_value, owner_pid, claim_id, now)? {
                DirectoryClaimResult::Bound {
                    instance_pid,
                    bound_at,
                } => {
                    return Ok(Some(DirectoryEntryStatusResponse::Bound {
                        instance_pid,
                        bound_at,
                    }));
                }
                DirectoryClaimResult::PendingFresh {
                    claim_id: _,
                    owner_pid,
                    created_at,
                    provisional_pid,
                } => {
                    return Ok(Some(DirectoryEntryStatusResponse::Pending {
                        owner_pid,
                        created_at,
                        provisional_pid,
                    }));
                }
                DirectoryClaimResult::Claimed(claim) => claim,
            };

        let pid = RequestOps::create_canister::<()>(
            &pool_cfg.canister_role,
            CreateCanisterParent::ThisCanister,
            None,
        )
        .await?
        .new_canister_pid;

        Self::finalize_created_instance(pool, key_value, claim, pid).await
    }

    // Recycle any abandoned provisional child and release the stale claim so one caller can
    // re-claim the key in the same user-driven flow without background timers.
    async fn cleanup_stale_entry(
        pool: &str,
        key_value: &str,
        claim_id: u64,
        provisional_pid: Option<Principal>,
    ) -> Result<(), InternalError> {
        if let Some(pid) = provisional_pid {
            Self::recycle_abandoned_child(pid).await?;
        }

        let _ = DirectoryRegistryOps::release_stale_pending_if_claim_matches(
            pool,
            key_value,
            claim_id,
            IcOps::now_secs(),
        )?;
        Ok(())
    }

    // Delegate orphan disposition to the root pool lifecycle instead of encoding pool logic here.
    async fn recycle_abandoned_child(pid: Principal) -> Result<(), InternalError> {
        if !SubnetRegistryOps::is_registered(pid) {
            return Ok(());
        }

        let _ = RequestOps::recycle_canister(pid).await?;
        Ok(())
    }

    // Release one stale claim after recycling any abandoned child and map the result for
    // explicit recovery callers. If ownership changed during cleanup, the caller should retry.
    async fn recover_cleanup_stale_entry(
        pool: &str,
        key_value: &str,
        claim_id: u64,
        provisional_pid: Option<Principal>,
    ) -> Result<Option<DirectoryRecoveryResponse>, InternalError> {
        if let Some(pid) = provisional_pid {
            Self::recycle_abandoned_child(pid).await?;
        }

        let now = IcOps::now_secs();
        match DirectoryRegistryOps::release_stale_pending_if_claim_matches(
            pool, key_value, claim_id, now,
        )? {
            DirectoryReleaseResult::ReleasedStalePending {
                owner_pid,
                created_at,
                provisional_pid,
            } => Ok(Some(DirectoryRecoveryResponse::ReleasedStalePending {
                owner_pid,
                created_at,
                provisional_pid,
                released_at: now,
            })),
            DirectoryReleaseResult::Missing => Ok(Some(DirectoryRecoveryResponse::Missing)),
            DirectoryReleaseResult::Bound {
                instance_pid,
                bound_at,
            } => Ok(Some(DirectoryRecoveryResponse::Bound {
                instance_pid,
                bound_at,
            })),
            DirectoryReleaseResult::PendingCurrent { .. } => Ok(None),
        }
    }

    // Classify the current entry once so resolve and recovery follow the same stale/repair rules.
    fn classify_entry(
        pool: &str,
        key_value: &str,
        pool_cfg: &DirectoryPool,
        now: u64,
    ) -> Option<DirectoryEntryClassification> {
        let state = DirectoryRegistryOps::lookup_state(pool, key_value)?;

        let classification = match state {
            DirectoryEntryState::Bound {
                instance_pid,
                bound_at,
            } => DirectoryEntryClassification::Bound {
                instance_pid,
                bound_at,
            },

            DirectoryEntryState::Pending {
                claim_id: _,
                owner_pid,
                created_at,
                provisional_pid,
            } if !pending_is_stale(now, created_at) => DirectoryEntryClassification::PendingFresh {
                owner_pid,
                created_at,
                provisional_pid,
            },

            DirectoryEntryState::Pending {
                claim_id,
                provisional_pid: Some(pid),
                ..
            } if Self::validate_bind_target(pid, &pool_cfg.canister_role).is_ok() => {
                DirectoryEntryClassification::Repairable {
                    claim_id,
                    provisional_pid: pid,
                }
            }

            DirectoryEntryState::Pending {
                claim_id,
                provisional_pid,
                ..
            } => DirectoryEntryClassification::NeedsCleanup {
                claim_id,
                provisional_pid,
            },
        };

        Some(classification)
    }

    // Repair a stale valid provisional child only if its original claim is still current.
    fn repair_stale_entry(
        pool: &str,
        key_value: &str,
        claim_id: u64,
        provisional_pid: Principal,
        now: u64,
    ) -> Result<DirectoryEntryStatusResponse, InternalError> {
        if !DirectoryRegistryOps::bind_if_claim_matches(
            pool,
            key_value,
            claim_id,
            provisional_pid,
            now,
        )? {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "directory claim lost during stale repair without an await boundary",
            ));
        }

        Ok(DirectoryEntryStatusResponse::Bound {
            instance_pid: provisional_pid,
            bound_at: now,
        })
    }

    // Validate that the target instance is one of this canister's direct children and
    // that its registered role matches the pool's configured instance role.
    fn validate_bind_target(
        pid: Principal,
        expected_role: &CanisterRole,
    ) -> Result<(), InternalError> {
        if !CanisterChildrenOps::data()
            .entries
            .iter()
            .any(|(child_pid, _)| *child_pid == pid)
        {
            return Err(DirectoryWorkflowError::InstanceNotDirectChild(pid).into());
        }

        let record =
            SubnetRegistryOps::get(pid).ok_or(DirectoryWorkflowError::RegistryEntryMissing(pid))?;

        if record.role != *expected_role {
            return Err(DirectoryWorkflowError::InstanceRoleMismatch {
                pid,
                expected: expected_role.clone(),
                actual: record.role,
            }
            .into());
        }

        Ok(())
    }

    // Resolve the configured pool definition for the current directory-bearing parent.
    fn get_directory_pool_cfg(pool: &str) -> Result<DirectoryPool, InternalError> {
        let directory = ConfigOps::current_directory_config()?
            .ok_or(DirectoryWorkflowError::DirectoryDisabled)?;
        let available = available_pool_names(&directory);

        directory
            .pools
            .get(pool)
            .cloned()
            .ok_or_else(|| DirectoryWorkflowError::UnknownPool {
                requested: pool.to_string(),
                available,
            })
            .map_err(InternalError::from)
    }
}

fn available_pool_names(directory: &DirectoryConfig) -> String {
    if directory.pools.is_empty() {
        return "none".to_string();
    }

    let mut names: Vec<_> = directory.pools.keys().cloned().collect();
    names.sort();
    names.join(", ")
}

fn new_claim_id() -> u64 {
    let nonce = DIRECTORY_CLAIM_NONCE.fetch_add(1, Ordering::Relaxed);
    IcOps::now_millis().rotate_left(21) ^ nonce
}

const fn pending_is_stale(now: u64, created_at: u64) -> bool {
    now.saturating_sub(created_at) > DirectoryRegistryOps::PENDING_TTL_SECS
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Cycles,
        config::schema::{
            CanisterConfig, CanisterKind, DelegatedAuthCanisterConfig, DirectoryConfig,
            DirectoryPool, RandomnessConfig, StandardsCanisterConfig,
        },
        ids::SubnetRole,
        ops::{
            storage::children::CanisterChildrenOps,
            storage::placement::directory::{DirectoryClaimResult, DirectoryRegistryOps},
            storage::registry::subnet::SubnetRegistryOps,
        },
        test::{
            config::ConfigTestBuilder,
            seams::{lock, p},
            support::import_test_env,
        },
    };
    use futures::executor::block_on;

    fn claim_id(id: u64) -> u64 {
        id
    }

    fn directory_hub_config(instance_role: &CanisterRole) -> CanisterConfig {
        let mut directory = DirectoryConfig::default();
        directory.pools.insert(
            "projects".to_string(),
            DirectoryPool {
                canister_role: instance_role.clone(),
                key_name: "project".to_string(),
            },
        );

        CanisterConfig {
            kind: CanisterKind::Singleton,
            initial_cycles: Cycles::new(0),
            topup_policy: None,
            randomness: RandomnessConfig::default(),
            scaling: None,
            sharding: None,
            directory: Some(directory),
            delegated_auth: DelegatedAuthCanisterConfig::default(),
            standards: StandardsCanisterConfig::default(),
        }
    }

    fn clear_subnet_registry() {
        for (pid, _) in SubnetRegistryOps::data().entries {
            let _ = SubnetRegistryOps::remove(&pid);
        }
    }

    fn install_directory_test_context(child_role: &CanisterRole, child_pid: Principal) {
        let root_pid = p(1);
        let hub_pid = p(2);

        let _cfg = ConfigTestBuilder::new()
            .with_prime_canister("project_hub", directory_hub_config(child_role))
            .with_prime_canister(
                "project_instance",
                ConfigTestBuilder::canister_config(CanisterKind::Instance),
            )
            .install();

        import_test_env(
            CanisterRole::new("project_hub"),
            SubnetRole::PRIME,
            root_pid,
        );

        clear_subnet_registry();
        DirectoryRegistryOps::clear_for_test();
        CanisterChildrenOps::import_direct_children(hub_pid, vec![(child_pid, child_role.clone())]);

        let created_at = 0;
        SubnetRegistryOps::register_root(root_pid, created_at);
        SubnetRegistryOps::register_unchecked(
            hub_pid,
            &CanisterRole::new("project_hub"),
            root_pid,
            vec![],
            created_at,
        )
        .expect("register hub");
        SubnetRegistryOps::register_unchecked(child_pid, child_role, hub_pid, vec![], created_at)
            .expect("register child");
    }

    #[test]
    fn bind_instance_persists_assignment_for_matching_direct_child() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);

        DirectoryWorkflow::bind_instance("projects", "alpha", child_pid)
            .expect("bind should succeed");

        assert_eq!(
            query::DirectoryQuery::lookup_key("projects", "alpha"),
            Some(child_pid)
        );
    }

    #[test]
    fn bind_instance_rejects_non_child_pid() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);
        CanisterChildrenOps::import_direct_children(p(2), vec![]);

        let err = DirectoryWorkflow::bind_instance("projects", "alpha", child_pid)
            .expect_err("bind should reject non-child pid");

        assert!(err.to_string().contains("not a direct child"));
    }

    #[test]
    fn bind_instance_rejects_role_mismatch() {
        let _guard = lock();
        let configured_role = CanisterRole::new("project_instance");
        let actual_role = CanisterRole::new("wrong_instance_role");
        let child_pid = p(3);
        install_directory_test_context(&configured_role, child_pid);
        clear_subnet_registry();

        let root_pid = p(1);
        let hub_pid = p(2);
        let created_at = 0;
        SubnetRegistryOps::register_root(root_pid, created_at);
        SubnetRegistryOps::register_unchecked(
            hub_pid,
            &CanisterRole::new("project_hub"),
            root_pid,
            vec![],
            created_at,
        )
        .expect("register hub");
        SubnetRegistryOps::register_unchecked(child_pid, &actual_role, hub_pid, vec![], created_at)
            .expect("register mismatched child");

        let err = DirectoryWorkflow::bind_instance("projects", "alpha", child_pid)
            .expect_err("bind should reject mismatched child role");

        assert!(err.to_string().contains("expected"));
    }

    #[test]
    fn resolve_or_create_returns_existing_bound_entry_without_create() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);
        DirectoryRegistryOps::bind("projects", "alpha", child_pid, 10).expect("seed bound entry");

        let result = block_on(DirectoryWorkflow::resolve_or_create("projects", "alpha"))
            .expect("bound entry should resolve without create");

        assert_eq!(
            result,
            DirectoryEntryStatusResponse::Bound {
                instance_pid: child_pid,
                bound_at: 10,
            }
        );
    }

    #[test]
    fn resolve_or_create_returns_fresh_pending_entry_without_create() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);

        let owner_pid = p(7);
        let created_at = IcOps::now_secs();
        let claim = DirectoryRegistryOps::claim_pending(
            "projects",
            "alpha",
            owner_pid,
            claim_id(1),
            created_at,
        )
        .expect("seed pending entry");
        assert_eq!(
            claim,
            DirectoryClaimResult::Claimed(DirectoryPendingClaim {
                claim_id: claim_id(1),
                owner_pid,
                created_at,
            })
        );

        let result = block_on(DirectoryWorkflow::resolve_or_create("projects", "alpha"))
            .expect("fresh pending should be surfaced");

        assert_eq!(
            result,
            DirectoryEntryStatusResponse::Pending {
                owner_pid,
                created_at,
                provisional_pid: None,
            }
        );
    }

    #[test]
    fn resolve_or_create_repairs_stale_pending_with_valid_provisional_child() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);

        let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
            .expect("seed stale pending entry");
        let DirectoryClaimResult::Claimed(claim) = claim else {
            panic!("expected stale claim");
        };
        DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
            "projects",
            "alpha",
            claim.claim_id,
            child_pid,
        )
        .expect("seed provisional child");

        let result = block_on(DirectoryWorkflow::resolve_or_create("projects", "alpha"))
            .expect("stale pending should repair to bound");

        match result {
            DirectoryEntryStatusResponse::Bound { instance_pid, .. } => {
                assert_eq!(instance_pid, child_pid);
            }
            other @ DirectoryEntryStatusResponse::Pending { .. } => {
                panic!("expected bound result, got {other:?}")
            }
        }
    }

    #[test]
    fn classify_entry_returns_none_for_missing_key() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);

        let pool_cfg = DirectoryWorkflow::get_directory_pool_cfg("projects")
            .expect("pool config should exist");
        let classification =
            DirectoryWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

        assert_eq!(classification, None);
    }

    #[test]
    fn classify_entry_marks_stale_pending_without_provisional_for_cleanup() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);
        DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
            .expect("seed stale pending entry");

        let pool_cfg = DirectoryWorkflow::get_directory_pool_cfg("projects")
            .expect("pool config should exist");
        let classification =
            DirectoryWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

        assert_eq!(
            classification,
            Some(DirectoryEntryClassification::NeedsCleanup {
                claim_id: claim_id(1),
                provisional_pid: None
            })
        );
    }

    #[test]
    fn classify_entry_marks_invalid_provisional_child_for_cleanup() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);
        let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
            .expect("seed stale pending entry");
        let DirectoryClaimResult::Claimed(claim) = claim else {
            panic!("expected stale claim");
        };
        DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
            "projects",
            "alpha",
            claim.claim_id,
            p(8),
        )
        .expect("seed invalid provisional child");

        let pool_cfg = DirectoryWorkflow::get_directory_pool_cfg("projects")
            .expect("pool config should exist");
        let classification =
            DirectoryWorkflow::classify_entry("projects", "alpha", &pool_cfg, IcOps::now_secs());

        assert_eq!(
            classification,
            Some(DirectoryEntryClassification::NeedsCleanup {
                claim_id: claim_id(1),
                provisional_pid: Some(p(8))
            })
        );
    }

    #[test]
    fn recover_entry_releases_stale_pending_without_provisional_child() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);
        DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
            .expect("seed stale pending entry");

        let result = block_on(DirectoryWorkflow::recover_entry("projects", "alpha"))
            .expect("stale dead key should be released");

        assert_eq!(
            result,
            DirectoryRecoveryResponse::ReleasedStalePending {
                owner_pid: p(7),
                created_at: 1,
                provisional_pid: None,
                released_at: IcOps::now_secs(),
            }
        );
        assert_eq!(
            DirectoryRegistryOps::lookup_entry("projects", "alpha"),
            None
        );
    }

    #[test]
    fn recover_entry_repairs_valid_stale_provisional_child() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);
        let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
            .expect("seed stale pending entry");
        let DirectoryClaimResult::Claimed(claim) = claim else {
            panic!("expected stale claim");
        };
        DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
            "projects",
            "alpha",
            claim.claim_id,
            child_pid,
        )
        .expect("seed provisional child");

        let result = block_on(DirectoryWorkflow::recover_entry("projects", "alpha"))
            .expect("valid provisional child should be repaired");

        assert_eq!(
            result,
            DirectoryRecoveryResponse::RepairedToBound {
                instance_pid: child_pid,
                bound_at: IcOps::now_secs(),
            }
        );
        assert!(matches!(
            DirectoryRegistryOps::lookup_entry("projects", "alpha"),
            Some(DirectoryEntryStatusResponse::Bound { instance_pid, .. }) if instance_pid == child_pid
        ));
    }

    #[test]
    fn recover_entry_releases_stale_pending_when_provisional_child_is_missing() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);

        let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), claim_id(1), 1)
            .expect("seed stale pending entry");
        let DirectoryClaimResult::Claimed(claim) = claim else {
            panic!("expected stale claim");
        };
        DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
            "projects",
            "alpha",
            claim.claim_id,
            p(8),
        )
        .expect("seed missing provisional child");

        let result = block_on(DirectoryWorkflow::recover_entry("projects", "alpha"))
            .expect("missing provisional child should still release stale key");

        assert_eq!(
            result,
            DirectoryRecoveryResponse::ReleasedStalePending {
                owner_pid: p(7),
                created_at: 1,
                provisional_pid: Some(p(8)),
                released_at: IcOps::now_secs(),
            }
        );
        assert_eq!(
            DirectoryRegistryOps::lookup_entry("projects", "alpha"),
            None
        );
    }
}
