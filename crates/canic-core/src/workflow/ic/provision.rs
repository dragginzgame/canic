// =============================================================================
// PROVISIONING (ROOT ORCHESTRATOR HELPERS)
// =============================================================================

//! Provisioning helpers for creating, installing, and tearing down canisters.
//!
//! These routines bundle the multi-phase orchestration that root performs when
//! scaling out the topology: reserving cycles, recording registry state,
//! installing WASM modules, and cascading state updates to descendants.

use crate::{
    InternalError, InternalErrorOrigin,
    api::runtime::install::{ApprovedModuleSource, ModuleSourceRuntimeApi},
    config::Config,
    config::schema::CanisterKind,
    domain::policy,
    dto::{abi::v1::CanisterInitPayload, env::EnvBootstrapArgs},
    ops::{
        config::ConfigOps,
        ic::{
            IcOps,
            mgmt::{CanisterInstallMode, MgmtOps},
        },
        runtime::env::EnvOps,
        runtime::metrics::canister_ops::{
            CanisterOpsMetricOperation, CanisterOpsMetricOutcome, CanisterOpsMetricReason,
            CanisterOpsMetrics,
        },
        runtime::metrics::provisioning::{
            ProvisioningMetricOperation, ProvisioningMetricOutcome, ProvisioningMetricReason,
            ProvisioningMetrics,
        },
        storage::{
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            registry::subnet::SubnetRegistryOps,
        },
        topology::index::builder::{RootAppIndexBuilder, RootSubnetIndexBuilder},
    },
    workflow::{
        cascade::snapshot::StateSnapshotBuilder, pool::PoolWorkflow, prelude::*,
        runtime::install::ModuleInstallWorkflow,
    },
};

///
/// ProvisionWorkflow
///

pub struct ProvisionWorkflow;

impl ProvisionWorkflow {
    pub fn build_nonroot_init_payload(
        role: &CanisterRole,
        parent_pid: Principal,
    ) -> Result<CanisterInitPayload, InternalError> {
        let env = EnvBootstrapArgs {
            prime_root_pid: Some(EnvOps::prime_root_pid()?),
            subnet_role: Some(EnvOps::subnet_role()?),
            subnet_pid: Some(EnvOps::subnet_pid()?),
            root_pid: Some(EnvOps::root_pid()?),
            canister_role: Some(role.clone()),
            parent_pid: Some(parent_pid),
        };

        let app_index = AppIndexOps::snapshot_args();
        let subnet_index = SubnetIndexOps::snapshot_args();

        Ok(CanisterInitPayload {
            env,
            app_index,
            subnet_index,
        })
    }

    //
    // ===========================================================================
    // INDEX SYNC
    // ===========================================================================
    //

    /// Rebuild AppIndex and SubnetIndex from the registry,
    /// import them directly, and return a builder containing the sections to sync.
    ///
    /// When `updated_role` is provided, only include the sections that list that role.
    pub fn rebuild_indexes_from_registry(
        updated_role: Option<&CanisterRole>,
    ) -> Result<StateSnapshotBuilder, InternalError> {
        let cfg = ConfigOps::get()?;
        let subnet_cfg = ConfigOps::current_subnet()?;
        let registry = SubnetRegistryOps::data();
        let allow_incomplete = updated_role.is_some();

        let include_app = updated_role.is_none_or(|role| cfg.app_index.contains(role));
        let include_subnet = updated_role.is_none_or(|role| subnet_cfg.subnet_index.contains(role));

        let mut builder = StateSnapshotBuilder::new()?;

        if include_app {
            let app_data = RootAppIndexBuilder::build(&registry, &cfg.app_index)?;

            if allow_incomplete {
                AppIndexOps::import_allow_incomplete(app_data)?;
            } else {
                AppIndexOps::import(app_data)?;
            }
            builder = builder.with_app_index()?;
        }

        if include_subnet {
            let subnet_data = RootSubnetIndexBuilder::build(&registry, &subnet_cfg.subnet_index)?;

            if allow_incomplete {
                SubnetIndexOps::import_allow_incomplete(subnet_data)?;
            } else {
                SubnetIndexOps::import(subnet_data)?;
            }
            builder = builder.with_subnet_index()?;
        }

        Ok(builder)
    }

    //
    // ===========================================================================
    // HIGH-LEVEL FLOW
    // ===========================================================================
    //

    /// Create and install a new canister of the requested type beneath `parent`.
    ///
    /// PHASES:
    /// 1. Allocate a canister ID and cycles (preferring the pool)
    /// 2. Install WASM + bootstrap initial state
    /// 3. Register canister in SubnetRegistry
    /// 4. Cascade topology + sync directories
    pub async fn create_and_install_canister(
        role: &CanisterRole,
        parent_pid: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        // Resolve the approved install source before allocation begins.
        record_provisioning(
            role,
            ProvisioningMetricOperation::ResolveModule,
            ProvisioningMetricOutcome::Started,
            ProvisioningMetricReason::Ok,
        );
        let module_source = match ModuleSourceRuntimeApi::approved_module_source(role).await {
            Ok(module_source) => {
                record_provisioning(
                    role,
                    ProvisioningMetricOperation::ResolveModule,
                    ProvisioningMetricOutcome::Completed,
                    ProvisioningMetricReason::Ok,
                );
                module_source
            }
            Err(err) => {
                record_canister_op(
                    role,
                    CanisterOpsMetricOperation::Install,
                    CanisterOpsMetricOutcome::Failed,
                    CanisterOpsMetricReason::MissingWasm,
                );
                record_provisioning(
                    role,
                    ProvisioningMetricOperation::ResolveModule,
                    ProvisioningMetricOutcome::Failed,
                    ProvisioningMetricReason::MissingWasm,
                );
                return Err(err);
            }
        };

        // Phase 1: allocation
        let (pid, source) = allocate_canister(role).await?;

        // Phase 2: installation
        if let Err(err) = install_canister(pid, role, parent_pid, &module_source, extra_arg).await {
            log!(
                Topic::CanisterLifecycle,
                Error,
                "install failed for {pid} ({role}): {err}"
            );
            if source == AllocationSource::Pool {
                if let Err(recycle_err) = PoolWorkflow::pool_import_canister(pid).await {
                    log!(
                        Topic::CanisterPool,
                        Warn,
                        "failed to recycle pool canister after install failure: {pid} ({recycle_err})"
                    );
                }
            } else if let Err(delete_err) = Self::uninstall_and_delete_canister(pid).await {
                log!(
                    Topic::CanisterLifecycle,
                    Warn,
                    "failed to delete canister after install failure: {pid} ({delete_err})"
                );
            }

            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("failed to install canister {pid}: {err}"),
            ));
        }

        Ok(pid)
    }

    //
    // ===========================================================================
    // DELETION
    // ===========================================================================
    //

    /// Delete an existing canister.
    ///
    /// PHASES:
    /// 0. Uninstall code
    /// 1. Delete via management canister
    /// 2. Remove from SubnetRegistry
    /// 3. Cascade topology
    /// 4. Sync directories
    pub async fn uninstall_and_delete_canister(pid: Principal) -> Result<(), InternalError> {
        if let Err(err) = EnvOps::require_root() {
            CanisterOpsMetrics::record_unscoped(
                CanisterOpsMetricOperation::Delete,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::from_error(&err),
            );
            return Err(err);
        }

        let role = SubnetRegistryOps::get(pid).map(|record| record.role);
        record_delete_metric(
            role.as_ref(),
            CanisterOpsMetricOutcome::Started,
            CanisterOpsMetricReason::Ok,
        );

        // Phase 0: uninstall code
        if let Err(err) = MgmtOps::uninstall_code(pid).await {
            record_delete_metric(
                role.as_ref(),
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::from_error(&err),
            );
            return Err(err);
        }

        // Phase 1: stop the canister before deletion.
        if let Err(err) = MgmtOps::stop_canister(pid).await {
            record_delete_metric(
                role.as_ref(),
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::from_error(&err),
            );
            return Err(err);
        }

        // Phase 2: delete the canister
        if let Err(err) = MgmtOps::delete_canister(pid).await {
            record_delete_metric(
                role.as_ref(),
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::from_error(&err),
            );
            return Err(err);
        }

        // Phase 3: remove registry record
        let removed_entry = SubnetRegistryOps::remove(&pid);
        match &removed_entry {
            Some(c) => log!(
                Topic::CanisterLifecycle,
                Ok,
                "🗑️ delete_canister: {} ({})",
                pid,
                c.role
            ),
            None => log!(
                Topic::CanisterLifecycle,
                Warn,
                "🗑️ delete_canister: {pid} not in registry"
            ),
        }

        record_delete_metric(
            role.as_ref(),
            CanisterOpsMetricOutcome::Completed,
            CanisterOpsMetricReason::Ok,
        );

        Ok(())
    }
}

//
// ===========================================================================
// PHASE 1 — ALLOCATION (Pool → Create)
// ===========================================================================
//

///
/// AllocationSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AllocationSource {
    Pool,
    New,
}

/// Allocate a canister ID and ensure it meets the initial cycle target.
///
/// Reuses a canister from the pool if available; otherwise creates a new one.
async fn allocate_canister(
    role: &CanisterRole,
) -> Result<(Principal, AllocationSource), InternalError> {
    // use ConfigOps for a clean, ops-layer config lookup
    record_provisioning(
        role,
        ProvisioningMetricOperation::Allocate,
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    let cfg = match ConfigOps::current_subnet_canister(role) {
        Ok(cfg) => cfg,
        Err(err) => {
            record_provisioning_failure(role, ProvisioningMetricOperation::Allocate, &err);
            return Err(err);
        }
    };
    let target = cfg.initial_cycles;

    if let Some(allocation) = try_allocate_from_pool(role, target.clone()).await? {
        return Ok(allocation);
    }

    // Create new canister
    let pid = match create_canister_with_configured_controllers(role, target).await {
        Ok(pid) => pid,
        Err(err) => {
            record_canister_op(
                role,
                CanisterOpsMetricOperation::Create,
                CanisterOpsMetricOutcome::Failed,
                CanisterOpsMetricReason::NewAllocation,
            );
            record_provisioning(
                role,
                ProvisioningMetricOperation::Allocate,
                ProvisioningMetricOutcome::Failed,
                ProvisioningMetricReason::NewAllocation,
            );
            return Err(err);
        }
    };

    record_canister_op(
        role,
        CanisterOpsMetricOperation::Create,
        CanisterOpsMetricOutcome::Completed,
        CanisterOpsMetricReason::NewAllocation,
    );
    record_provisioning(
        role,
        ProvisioningMetricOperation::Allocate,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::NewAllocation,
    );

    Ok((pid, AllocationSource::New))
}

// Reuse a ready pool canister when one is available.
async fn try_allocate_from_pool(
    role: &CanisterRole,
    target: Cycles,
) -> Result<Option<(Principal, AllocationSource)>, InternalError> {
    let Some(pid) = PoolWorkflow::pop_oldest_ready() else {
        return Ok(None);
    };

    let mut current = match MgmtOps::get_cycles(pid).await {
        Ok(current) => current,
        Err(err) => {
            record_provisioning_failure(role, ProvisioningMetricOperation::Allocate, &err);
            return Err(err);
        }
    };

    if current < target {
        current = topup_pool_allocation(role, pid, current, target).await?;
    }

    log!(
        Topic::CanisterPool,
        Ok,
        "⚡ allocate_canister: reusing {pid} role={role} from pool (current {current})"
    );
    record_canister_op(
        role,
        CanisterOpsMetricOperation::Create,
        CanisterOpsMetricOutcome::Completed,
        CanisterOpsMetricReason::PoolReuse,
    );
    record_provisioning(
        role,
        ProvisioningMetricOperation::Allocate,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::PoolReuse,
    );

    Ok(Some((pid, AllocationSource::Pool)))
}

// Top up a reused pool canister to the configured initial cycle target.
async fn topup_pool_allocation(
    role: &CanisterRole,
    pid: Principal,
    current: Cycles,
    target: Cycles,
) -> Result<Cycles, InternalError> {
    let missing = target.to_u128().saturating_sub(current.to_u128());
    if missing == 0 {
        return Ok(current);
    }

    if let Err(err) = MgmtOps::deposit_cycles(pid, missing).await {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Create,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::PoolTopup,
        );
        record_provisioning(
            role,
            ProvisioningMetricOperation::Allocate,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::PoolTopup,
        );
        return Err(err);
    }

    log!(
        Topic::CanisterPool,
        Ok,
        "⚡ allocate_canister: topped up {pid} by {} to meet target {}",
        Cycles::from(missing),
        target
    );
    Ok(Cycles::new(current.to_u128() + missing))
}

/// Create a fresh canister on the IC with the configured controllers.
async fn create_canister_with_configured_controllers(
    role: &CanisterRole,
    cycles: Cycles,
) -> Result<Principal, InternalError> {
    let root = IcOps::canister_self();
    let mut controllers = Config::get()?.controllers.clone();
    controllers.push(root); // root always controls

    let pid = MgmtOps::create_canister(controllers, cycles.clone()).await?;

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "⚡ create_canister: {pid} role={role} cycles={cycles} source=new (pool empty)"
    );

    Ok(pid)
}

//
// ===========================================================================
// PHASE 2 — INSTALLATION
// ===========================================================================
//

/// Install WASM and initial state into a new canister.
async fn install_canister(
    pid: Principal,
    role: &CanisterRole,
    parent_pid: Principal,
    module_source: &ApprovedModuleSource,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), InternalError> {
    record_provisioning(
        role,
        ProvisioningMetricOperation::Install,
        ProvisioningMetricOutcome::Started,
        ProvisioningMetricReason::Ok,
    );
    record_canister_op(
        role,
        CanisterOpsMetricOperation::Install,
        CanisterOpsMetricOutcome::Started,
        CanisterOpsMetricReason::Ok,
    );

    let payload = match ProvisionWorkflow::build_nonroot_init_payload(role, parent_pid) {
        Ok(payload) => payload,
        Err(err) => {
            record_canister_op_failure(role, CanisterOpsMetricOperation::Install, &err);
            record_provisioning_failure(role, ProvisioningMetricOperation::Install, &err);
            return Err(err);
        }
    };
    let module_hash = module_source.module_hash().to_vec();

    // Register before install so init hooks can observe the registry; roll back on failure.
    // otherwise if the init() tries to create a canister via root, it will panic
    if let Err(err) = validate_registration_policy(role, parent_pid) {
        record_canister_op(
            role,
            CanisterOpsMetricOperation::Install,
            CanisterOpsMetricOutcome::Failed,
            CanisterOpsMetricReason::Topology,
        );
        record_provisioning(
            role,
            ProvisioningMetricOperation::Install,
            ProvisioningMetricOutcome::Failed,
            ProvisioningMetricReason::Topology,
        );
        return Err(err);
    }

    let created_at = IcOps::now_secs();
    if let Err(err) = SubnetRegistryOps::register_unchecked(
        pid,
        role,
        parent_pid,
        module_hash.clone(),
        created_at,
    ) {
        record_canister_op_failure(role, CanisterOpsMetricOperation::Install, &err);
        record_provisioning_failure(role, ProvisioningMetricOperation::Install, &err);
        return Err(err);
    }

    if let Err(err) = ModuleInstallWorkflow::install_with_payload(
        CanisterInstallMode::Install,
        pid,
        module_source,
        payload,
        extra_arg,
    )
    .await
    {
        record_canister_op_failure(role, CanisterOpsMetricOperation::Install, &err);
        record_provisioning_failure(role, ProvisioningMetricOperation::Install, &err);

        let removed = SubnetRegistryOps::remove(&pid);
        if removed.is_none() {
            log!(
                Topic::CanisterLifecycle,
                Warn,
                "⚠️ install_canister rollback: {pid} missing from registry after failed install"
            );
        }

        return Err(err);
    }

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "⚡ install_canister: {pid} ({role}, source={}, size={}, chunks={})",
        module_source.source_label(),
        module_source.payload_size(),
        module_source.chunk_count(),
    );

    record_canister_op(
        role,
        CanisterOpsMetricOperation::Install,
        CanisterOpsMetricOutcome::Completed,
        CanisterOpsMetricReason::Ok,
    );
    record_provisioning(
        role,
        ProvisioningMetricOperation::Install,
        ProvisioningMetricOutcome::Completed,
        ProvisioningMetricReason::Ok,
    );

    Ok(())
}

// Record one canister operation metric for a known role.
fn record_canister_op(
    role: &CanisterRole,
    operation: CanisterOpsMetricOperation,
    outcome: CanisterOpsMetricOutcome,
    reason: CanisterOpsMetricReason,
) {
    CanisterOpsMetrics::record(operation, role, outcome, reason);
}

// Record one failed canister operation metric using the structured error category.
fn record_canister_op_failure(
    role: &CanisterRole,
    operation: CanisterOpsMetricOperation,
    err: &InternalError,
) {
    record_canister_op(
        role,
        operation,
        CanisterOpsMetricOutcome::Failed,
        CanisterOpsMetricReason::from_error(err),
    );
}

// Record one provisioning metric for a known role.
fn record_provisioning(
    role: &CanisterRole,
    operation: ProvisioningMetricOperation,
    outcome: ProvisioningMetricOutcome,
    reason: ProvisioningMetricReason,
) {
    ProvisioningMetrics::record(operation, role, outcome, reason);
}

// Record one failed provisioning metric using the structured error category.
fn record_provisioning_failure(
    role: &CanisterRole,
    operation: ProvisioningMetricOperation,
    err: &InternalError,
) {
    record_provisioning(
        role,
        operation,
        ProvisioningMetricOutcome::Failed,
        ProvisioningMetricReason::from_error(err),
    );
}

// Record one delete metric using the registry role when it is still available.
fn record_delete_metric(
    role: Option<&CanisterRole>,
    outcome: CanisterOpsMetricOutcome,
    reason: CanisterOpsMetricReason,
) {
    if let Some(role) = role {
        CanisterOpsMetrics::record(CanisterOpsMetricOperation::Delete, role, outcome, reason);
    } else {
        CanisterOpsMetrics::record_unknown_role(
            CanisterOpsMetricOperation::Delete,
            outcome,
            reason,
        );
    }
}

// Validate create-time registry policy using targeted registry lookups instead of a full export.
fn validate_registration_policy(
    role: &CanisterRole,
    parent_pid: Principal,
) -> Result<(), InternalError> {
    let canister_cfg = ConfigOps::current_subnet_canister(role)?;
    let parent_role = SubnetRegistryOps::get(parent_pid)
        .map(|record| record.role)
        .ok_or(policy::topology::TopologyPolicyError::ParentNotFound(
            parent_pid,
        ))?;
    let parent_cfg = ConfigOps::current_subnet_canister(&parent_role)?;

    let observed = policy::topology::registry::RegistryRegistrationObservation {
        existing_role_pid: matches!(canister_cfg.kind, CanisterKind::Root)
            .then(|| SubnetRegistryOps::find_pid_for_role(role))
            .flatten(),
        existing_singleton_under_parent_pid: matches!(canister_cfg.kind, CanisterKind::Singleton)
            .then(|| {
                if role.is_wasm_store() {
                    None
                } else {
                    SubnetRegistryOps::find_child_pid_for_role(parent_pid, role)
                }
            })
            .flatten(),
    };

    policy::topology::registry::RegistryPolicy::can_register_role_observed(
        role,
        parent_pid,
        observed,
        &canister_cfg,
        &parent_role,
        &parent_cfg,
    )
    .map_err(policy::topology::TopologyPolicyError::from)?;

    Ok(())
}
