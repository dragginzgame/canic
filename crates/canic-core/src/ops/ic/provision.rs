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
    cdk::{api::canister_self, mgmt::CanisterInstallMode},
    config::Config,
    log::Topic,
    ops::{
        OpsError,
        config::ConfigOps,
        ic::IcOpsError,
        orchestration::cascade::state::StateBundle,
        pool::PoolOps,
        prelude::*,
        storage::{
            CanisterInitPayload,
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            env::{EnvData, EnvOps},
            topology::SubnetCanisterRegistryOps,
        },
        wasm::WasmOps,
    },
    types::Cycles,
};
use candid::Principal;
use thiserror::Error as ThisError;

pub(crate) fn build_nonroot_init_payload(
    role: &CanisterRole,
    parent_pid: Principal,
) -> Result<CanisterInitPayload, Error> {
    let env = EnvData {
        prime_root_pid: Some(EnvOps::try_get_prime_root_pid()?),
        subnet_role: Some(EnvOps::try_get_subnet_role()?),
        subnet_pid: Some(EnvOps::try_get_subnet_pid()?),
        root_pid: Some(EnvOps::try_get_root_pid()?),
        canister_role: Some(role.clone()),
        parent_pid: Some(parent_pid),
    };

    Ok(CanisterInitPayload {
        env,
        app_directory: AppDirectoryOps::export(),
        subnet_directory: SubnetDirectoryOps::export(),
    })
}

///
/// ProvisionOpsError
///

#[derive(Debug, ThisError)]
pub enum ProvisionOpsError {
    #[error(transparent)]
    Other(#[from] Error),

    #[error("install failed for {pid}: {source}")]
    InstallFailed { pid: Principal, source: Error },
}

impl From<ProvisionOpsError> for Error {
    fn from(err: ProvisionOpsError) -> Self {
        IcOpsError::from(err).into()
    }
}

//
// ===========================================================================
// DIRECTORY SYNC
// ===========================================================================
//

/// Rebuild AppDirectory and SubnetDirectory from the registry,
/// import them directly, and return the resulting state bundle.
/// When `updated_ty` is provided, only include the sections that list that type.
pub(crate) async fn rebuild_directories_from_registry(
    updated_role: Option<&CanisterRole>,
) -> Result<StateBundle, Error> {
    let mut bundle = StateBundle::default();
    let cfg = Config::get();

    // did a directory change?
    let include_app = updated_role.is_none_or(|role| cfg.app_directory.contains(role));
    let include_subnet = if let Some(role) = updated_role {
        let subnet_cfg = ConfigOps::current_subnet()?;
        subnet_cfg.subnet_directory.contains(role)
    } else {
        true
    };

    if include_app {
        let app_view = AppDirectoryOps::root_build_view();
        AppDirectoryOps::import(app_view.clone());
        bundle.app_directory = Some(app_view);
    }

    if include_subnet {
        let subnet_view = SubnetDirectoryOps::root_build_view();
        SubnetDirectoryOps::import(subnet_view.clone());
        bundle.subnet_directory = Some(subnet_view);
    }

    Ok(bundle)
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
/// 3. Register canister in SubnetCanisterRegistry
/// 4. Cascade topology + sync directories
pub async fn create_and_install_canister(
    role: &CanisterRole,
    parent_pid: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    // must have WASM module registered
    WasmOps::try_get(role)?;

    // Phase 1: allocation
    let pid = allocate_canister(role).await?;

    // Phase 2: installation
    if let Err(err) = install_canister(pid, role, parent_pid, extra_arg).await {
        return Err(ProvisionOpsError::InstallFailed { pid, source: err }.into());
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
/// 2. Remove from SubnetCanisterRegistry
/// 3. Cascade topology
/// 4. Sync directories
pub async fn delete_canister(pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    // Phase 0: uninstall code
    super::uninstall_code(pid).await?;

    // Phase 1: delete the canister
    super::delete_canister(pid).await?;

    // Phase 2: remove registry record
    let removed_entry = SubnetCanisterRegistryOps::remove(&pid);
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

/// Uninstall code from a canister without deleting it.
pub async fn uninstall_canister(pid: Principal) -> Result<(), Error> {
    super::uninstall_code(pid).await?;

    log!(Topic::CanisterLifecycle, Ok, "🗑️ uninstall_canister: {pid}");

    Ok(())
}

//
// ===========================================================================
// PHASE 1 — ALLOCATION (Pool → Create)
// ===========================================================================
//

/// Allocate a canister ID and ensure it meets the initial cycle target.
///
/// Reuses a canister from the pool if available; otherwise creates a new one.
pub async fn allocate_canister(role: &CanisterRole) -> Result<Principal, Error> {
    // use ConfigOps for a clean, ops-layer config lookup
    let cfg = ConfigOps::current_subnet_canister(role)?;
    let target = cfg.initial_cycles;

    // Reuse from pool
    if let Some((pid, _)) = PoolOps::pop_ready() {
        let mut current = super::get_cycles(pid).await?;

        if current < target {
            let missing = target.to_u128().saturating_sub(current.to_u128());
            if missing > 0 {
                super::deposit_cycles(pid, missing).await?;
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

        return Ok(pid);
    }

    // Create new canister
    let pid = create_canister_with_configured_controllers(target).await?;
    log!(
        Topic::CanisterPool,
        Info,
        "⚡ allocate_canister: pool empty"
    );

    Ok(pid)
}

/// Create a fresh canister on the IC with the configured controllers.
async fn create_canister_with_configured_controllers(cycles: Cycles) -> Result<Principal, Error> {
    let root = canister_self();
    let mut controllers = Config::get().controllers.clone();
    controllers.push(root); // root always controls

    let pid = super::create_canister(controllers, cycles.clone()).await?;

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

    let payload = build_nonroot_init_payload(role, parent_pid)?;

    let module_hash = wasm.module_hash();

    // Register before install so init hooks can observe the registry; roll back on failure.
    // otherwise if the init() tries to create a canister via root, it will panic
    SubnetCanisterRegistryOps::register(pid, role, parent_pid, module_hash.clone());

    if let Err(err) = super::install_canic_code(
        CanisterInstallMode::Install,
        pid,
        wasm.bytes(),
        payload,
        extra_arg,
    )
    .await
    {
        let removed = SubnetCanisterRegistryOps::remove(&pid);
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
