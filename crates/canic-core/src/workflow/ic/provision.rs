// =============================================================================
// PROVISIONING (ROOT ORCHESTRATOR HELPERS)
// =============================================================================

//! Provisioning helpers for creating, installing, and tearing down canisters.
//!
//! These routines bundle the multi-phase orchestration that root performs when
//! scaling out the topology: reserving cycles, recording registry state,
//! installing WASM modules, and cascading state updates to descendants.

use crate::{
    Error,
    access::env,
    config::Config,
    domain::policy,
    dto::{abi::v1::CanisterInitPayload, env::EnvView},
    ops::{
        config::ConfigOps,
        ic::{
            mgmt::{CanisterInstallMode, MgmtOps},
            now_secs,
        },
        runtime::{env::EnvOps, wasm::WasmOps},
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{
        cascade::snapshot::StateSnapshotBuilder,
        ic::IcWorkflowError,
        pool::PoolWorkflow,
        prelude::*,
        topology::directory::{
            builder::{RootAppDirectoryBuilder, RootSubnetDirectoryBuilder},
            mapper::{AppDirectoryMapper, SubnetDirectoryMapper},
        },
    },
};
use thiserror::Error as ThisError;

///
/// ProvisionWorkflowError
///

#[derive(Debug, ThisError)]
pub enum ProvisionWorkflowError {
    #[error("install failed for {pid}")]
    InstallFailed { pid: Principal },
}

impl From<ProvisionWorkflowError> for Error {
    fn from(err: ProvisionWorkflowError) -> Self {
        IcWorkflowError::from(err).into()
    }
}

///
/// ProvisionWorkflow
///

pub struct ProvisionWorkflow;

impl ProvisionWorkflow {
    pub fn build_nonroot_init_payload(
        role: &CanisterRole,
        parent_pid: Principal,
    ) -> Result<CanisterInitPayload, Error> {
        let env = EnvView {
            prime_root_pid: Some(EnvOps::prime_root_pid()?),
            subnet_role: Some(EnvOps::subnet_role()?),
            subnet_pid: Some(EnvOps::subnet_pid()?),
            root_pid: Some(EnvOps::root_pid()?),
            canister_role: Some(role.clone()),
            parent_pid: Some(parent_pid),
        };

        let app_directory = AppDirectoryMapper::snapshot_to_view(AppDirectoryOps::snapshot());
        let subnet_directory =
            SubnetDirectoryMapper::snapshot_to_view(SubnetDirectoryOps::snapshot());

        Ok(CanisterInitPayload {
            env,
            app_directory,
            subnet_directory,
        })
    }

    //
    // ===========================================================================
    // DIRECTORY SYNC
    // ===========================================================================
    //

    /// Rebuild AppDirectory and SubnetDirectory from the registry,
    /// import them directly, and return a builder containing the sections to sync.
    ///
    /// When `updated_role` is provided, only include the sections that list that role.
    pub fn rebuild_directories_from_registry(
        updated_role: Option<&CanisterRole>,
    ) -> Result<StateSnapshotBuilder, Error> {
        let cfg = ConfigOps::get()?;
        let subnet_cfg = ConfigOps::current_subnet()?;
        let registry = SubnetRegistryOps::snapshot();

        let include_app = updated_role.is_none_or(|role| cfg.app_directory.contains(role));
        let include_subnet =
            updated_role.is_none_or(|role| subnet_cfg.subnet_directory.contains(role));

        let mut builder = StateSnapshotBuilder::new()?;

        if include_app {
            let app_snapshot = RootAppDirectoryBuilder::build(&registry, &cfg.app_directory);

            AppDirectoryOps::import(app_snapshot);
            builder = builder.with_app_directory();
        }

        if include_subnet {
            let subnet_snapshot =
                RootSubnetDirectoryBuilder::build(&registry, &subnet_cfg.subnet_directory);

            SubnetDirectoryOps::import(subnet_snapshot);
            builder = builder.with_subnet_directory();
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
    ) -> Result<Principal, Error> {
        // must have WASM module registered
        WasmOps::try_get(role)?;

        // Phase 1: allocation
        let (pid, source) = allocate_canister(role).await?;

        // Phase 2: installation
        if install_canister(pid, role, parent_pid, extra_arg)
            .await
            .is_err()
        {
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

            return Err(ProvisionWorkflowError::InstallFailed { pid }.into());
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
    pub async fn uninstall_and_delete_canister(pid: Principal) -> Result<(), Error> {
        env::require_root()?;

        // Phase 0: uninstall code
        MgmtOps::uninstall_code(pid).await?;

        // Phase 1: delete the canister
        MgmtOps::delete_canister(pid).await?;

        // Phase 2: remove registry record
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
async fn allocate_canister(role: &CanisterRole) -> Result<(Principal, AllocationSource), Error> {
    // use ConfigOps for a clean, ops-layer config lookup
    let cfg = ConfigOps::current_subnet_canister(role)?;
    let target = cfg.initial_cycles;

    // Reuse from pool
    if let Some(entry) = PoolWorkflow::pop_oldest_ready() {
        let pid = entry.pid;
        let mut current = MgmtOps::get_cycles(pid).await?;

        if current < target {
            let missing = target.to_u128().saturating_sub(current.to_u128());
            if missing > 0 {
                MgmtOps::deposit_cycles(pid, missing).await?;
                current = Cycles::new(current.to_u128() + missing);

                log!(
                    Topic::CanisterPool,
                    Ok,
                    "⚡ allocate_canister: topped up {pid} by {} to meet target {}",
                    Cycles::from(missing),
                    target
                );
            }
        }

        log!(
            Topic::CanisterPool,
            Ok,
            "⚡ allocate_canister: reusing {pid} from pool (current {current})"
        );

        return Ok((pid, AllocationSource::Pool));
    }

    // Create new canister
    let pid = create_canister_with_configured_controllers(target).await?;
    log!(
        Topic::CanisterPool,
        Info,
        "⚡ allocate_canister: pool empty"
    );

    Ok((pid, AllocationSource::New))
}

/// Create a fresh canister on the IC with the configured controllers.
async fn create_canister_with_configured_controllers(cycles: Cycles) -> Result<Principal, Error> {
    let root = canister_self();
    let mut controllers = Config::get()?.controllers.clone();
    controllers.push(root); // root always controls

    let pid = MgmtOps::create_canister(controllers, cycles.clone()).await?;

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "⚡ create_canister: {pid} ({cycles})"
    );

    Ok(pid)
}

//
// ===========================================================================
// PHASE 2 — INSTALLATION
// ===========================================================================
//

/// Install WASM and initial state into a new canister.
#[allow(clippy::cast_precision_loss)]
async fn install_canister(
    pid: Principal,
    role: &CanisterRole,
    parent_pid: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    // Fetch and register WASM
    let wasm = WasmOps::try_get(role)?;

    let payload = ProvisionWorkflow::build_nonroot_init_payload(role, parent_pid)?;
    let module_hash = wasm.module_hash();

    // Register before install so init hooks can observe the registry; roll back on failure.
    // otherwise if the init() tries to create a canister via root, it will panic
    let registry_snapshot = SubnetRegistryOps::snapshot();
    let canister_cfg = ConfigOps::current_subnet_canister(role)?;
    policy::topology::registry::RegistryPolicy::can_register_role(
        role,
        &registry_snapshot,
        &canister_cfg,
    )
    .map_err(Error::from)?;
    let created_at = now_secs();
    SubnetRegistryOps::register_unchecked(pid, role, parent_pid, module_hash.clone(), created_at)?;

    if let Err(err) = MgmtOps::install_canister_with_payload(
        CanisterInstallMode::Install,
        pid,
        wasm.bytes(),
        payload,
        extra_arg,
    )
    .await
    {
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
        "⚡ install_canister: {pid} ({role}, {:.2} KiB)",
        wasm.len() as f64 / 1_024.0,
    );

    Ok(())
}
