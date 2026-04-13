pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::{DirectoryConfig, DirectoryPool},
    dto::placement::directory::DirectoryEntryStatusResponse,
    ids::CanisterRole,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        rpc::request::{CreateCanisterParent, RequestOps},
        storage::{
            children::CanisterChildrenOps,
            placement::directory::{DirectoryClaimResult, DirectoryRegistryOps},
            registry::subnet::SubnetRegistryOps,
        },
    },
};
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
enum DirectoryResolvePlan {
    Return(DirectoryEntryStatusResponse),
    CreateNew,
}

pub struct DirectoryWorkflow;

impl DirectoryWorkflow {
    /// Resolve a bound instance for one key or create and bind a new one.
    pub async fn resolve_or_create(
        pool: &str,
        key_value: &str,
    ) -> Result<DirectoryEntryStatusResponse, InternalError> {
        let pool_cfg = Self::get_directory_pool_cfg(pool)?;
        let now = IcOps::now_secs();
        let owner_pid = IcOps::metadata_entropy_canister();

        match Self::plan_resolve_or_create(pool, key_value, &pool_cfg, now, owner_pid)? {
            DirectoryResolvePlan::Return(status) => return Ok(status),
            DirectoryResolvePlan::CreateNew => {}
        }

        let pid = RequestOps::create_canister::<()>(
            &pool_cfg.canister_role,
            CreateCanisterParent::ThisCanister,
            None,
        )
        .await?
        .new_canister_pid;

        DirectoryRegistryOps::set_provisional_pid(pool, key_value, pid)?;
        Self::validate_bind_target(pid, &pool_cfg.canister_role)?;

        let bound_at = IcOps::now_secs();
        DirectoryRegistryOps::bind(pool, key_value, pid, bound_at)?;

        Ok(DirectoryEntryStatusResponse::Bound {
            instance_pid: pid,
            bound_at,
        })
    }

    // Decide whether the current request can return an existing status or must create.
    fn plan_resolve_or_create(
        pool: &str,
        key_value: &str,
        pool_cfg: &DirectoryPool,
        now: u64,
        owner_pid: Principal,
    ) -> Result<DirectoryResolvePlan, InternalError> {
        if let Some(status) = Self::resolve_existing_entry(pool, key_value, pool_cfg, now)? {
            return Ok(DirectoryResolvePlan::Return(status));
        }

        match DirectoryRegistryOps::claim_pending(pool, key_value, owner_pid, now)? {
            DirectoryClaimResult::Bound {
                instance_pid,
                bound_at,
            } => Ok(DirectoryResolvePlan::Return(
                DirectoryEntryStatusResponse::Bound {
                    instance_pid,
                    bound_at,
                },
            )),
            DirectoryClaimResult::PendingFresh {
                owner_pid,
                created_at,
                provisional_pid,
            } => Ok(DirectoryResolvePlan::Return(
                DirectoryEntryStatusResponse::Pending {
                    owner_pid,
                    created_at,
                    provisional_pid,
                },
            )),
            DirectoryClaimResult::Claimed { .. } => Ok(DirectoryResolvePlan::CreateNew),
        }
    }

    /// Bind one logical directory key to a validated direct child instance.
    pub fn bind_instance(pool: &str, key_value: &str, pid: Principal) -> Result<(), InternalError> {
        let pool_cfg = Self::get_directory_pool_cfg(pool)?;
        Self::validate_bind_target(pid, &pool_cfg.canister_role)?;
        DirectoryRegistryOps::bind(pool, key_value, pid, IcOps::now_secs())
    }

    // Resolve entry states that can be satisfied without issuing a new create request.
    fn resolve_existing_entry(
        pool: &str,
        key_value: &str,
        pool_cfg: &DirectoryPool,
        now: u64,
    ) -> Result<Option<DirectoryEntryStatusResponse>, InternalError> {
        let Some(status) = DirectoryRegistryOps::lookup_entry(pool, key_value) else {
            return Ok(None);
        };

        match status {
            DirectoryEntryStatusResponse::Bound { .. } => Ok(Some(status)),

            DirectoryEntryStatusResponse::Pending {
                owner_pid,
                created_at,
                provisional_pid,
            } if !pending_is_stale(now, created_at) => {
                Ok(Some(DirectoryEntryStatusResponse::Pending {
                    owner_pid,
                    created_at,
                    provisional_pid,
                }))
            }

            DirectoryEntryStatusResponse::Pending {
                provisional_pid: Some(pid),
                ..
            } => {
                if Self::validate_bind_target(pid, &pool_cfg.canister_role).is_ok() {
                    DirectoryRegistryOps::bind(pool, key_value, pid, now)?;
                    return Ok(Some(DirectoryEntryStatusResponse::Bound {
                        instance_pid: pid,
                        bound_at: now,
                    }));
                }

                Ok(None)
            }

            DirectoryEntryStatusResponse::Pending { .. } => Ok(None),
        }
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

const fn pending_is_stale(now: u64, created_at: u64) -> bool {
    now.saturating_sub(created_at) > DirectoryRegistryOps::PENDING_TTL_SECS
}

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
            runtime::env::EnvOps,
            storage::children::CanisterChildrenOps,
            storage::placement::directory::{DirectoryClaimResult, DirectoryRegistryOps},
            storage::registry::subnet::SubnetRegistryOps,
        },
        storage::stable::env::EnvRecord,
        test::{
            config::ConfigTestBuilder,
            seams::{lock, p},
        },
    };
    use futures::executor::block_on;

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

        let env = EnvRecord {
            canister_role: Some(CanisterRole::new("project_hub")),
            subnet_role: Some(SubnetRole::PRIME),
            root_pid: Some(root_pid),
            prime_root_pid: Some(root_pid),
            subnet_pid: Some(root_pid),
            parent_pid: Some(root_pid),
        };
        EnvOps::import(env).expect("import directory test env");

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
        let claim = DirectoryRegistryOps::claim_pending("projects", "alpha", owner_pid, created_at)
            .expect("seed pending entry");
        assert_eq!(
            claim,
            DirectoryClaimResult::Claimed {
                owner_pid,
                created_at,
            }
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

        DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), 1)
            .expect("seed stale pending entry");
        DirectoryRegistryOps::set_provisional_pid("projects", "alpha", child_pid)
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
    fn plan_resolve_or_create_returns_create_new_for_missing_key() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);

        let pool_cfg = DirectoryWorkflow::get_directory_pool_cfg("projects")
            .expect("pool config should exist");
        let plan = DirectoryWorkflow::plan_resolve_or_create(
            "projects",
            "alpha",
            &pool_cfg,
            IcOps::now_secs(),
            p(9),
        )
        .expect("missing key should require create");

        assert_eq!(plan, DirectoryResolvePlan::CreateNew);
    }

    #[test]
    fn plan_resolve_or_create_returns_create_new_for_stale_pending_without_provisional() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);
        DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), 1)
            .expect("seed stale pending entry");

        let pool_cfg = DirectoryWorkflow::get_directory_pool_cfg("projects")
            .expect("pool config should exist");
        let plan = DirectoryWorkflow::plan_resolve_or_create(
            "projects",
            "alpha",
            &pool_cfg,
            IcOps::now_secs(),
            p(9),
        )
        .expect("stale pending without provisional child should require create");

        assert_eq!(plan, DirectoryResolvePlan::CreateNew);
    }

    #[test]
    fn plan_resolve_or_create_returns_create_new_for_invalid_provisional_child() {
        let _guard = lock();
        let child_role = CanisterRole::new("project_instance");
        let child_pid = p(3);
        install_directory_test_context(&child_role, child_pid);
        DirectoryRegistryOps::claim_pending("projects", "alpha", p(7), 1)
            .expect("seed stale pending entry");
        DirectoryRegistryOps::set_provisional_pid("projects", "alpha", p(8))
            .expect("seed invalid provisional child");

        let pool_cfg = DirectoryWorkflow::get_directory_pool_cfg("projects")
            .expect("pool config should exist");
        let plan = DirectoryWorkflow::plan_resolve_or_create(
            "projects",
            "alpha",
            &pool_cfg,
            IcOps::now_secs(),
            p(9),
        )
        .expect("invalid provisional child should require create");

        assert_eq!(plan, DirectoryResolvePlan::CreateNew);
    }
}
