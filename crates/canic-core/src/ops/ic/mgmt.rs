//! Ops-scoped IC helpers.
//!
//! These wrappers attach ops-level concerns such as metrics recording around
//! IC management canister calls and common ICC call patterns.

#![allow(clippy::disallowed_methods)]

use crate::{
    Error,
    cdk::{
        mgmt::{
            self, CanisterInstallMode, CanisterSettings, CanisterStatusArgs, CanisterStatusResult,
            CreateCanisterArgs, DeleteCanisterArgs, DepositCyclesArgs, InstallCodeArgs,
            UninstallCodeArgs, UpdateSettingsArgs, WasmModule,
        },
        utils::wasm::get_wasm_hash,
    },
    env::nns::NNS_REGISTRY_CANISTER,
    log,
    log::Topic,
    model::metrics::system::{SystemMetricKind, SystemMetrics},
    ops::ic::call::Call,
    spec::nns::{GetSubnetForCanisterRequest, GetSubnetForCanisterResponse},
    types::Cycles,
};
use candid::{CandidType, Principal, encode_args, utils::ArgumentEncoder};

//
// ──────────────────────────────── CREATE CANISTER ────────────────────────────
//

/// Create a canister with explicit controllers and an initial cycle balance.
pub async fn create_canister(
    controllers: Vec<Principal>,
    cycles: Cycles,
) -> Result<Principal, Error> {
    let settings = Some(CanisterSettings {
        controllers: Some(controllers),
        ..Default::default()
    });
    let args = CreateCanisterArgs { settings };

    let pid = mgmt::create_canister_with_extra_cycles(&args, cycles.to_u128())
        .await?
        .canister_id;

    SystemMetrics::increment(SystemMetricKind::CreateCanister);
    Ok(pid)
}

//
// ────────────────────────────── CANISTER STATUS ──────────────────────────────
//

/// Query the management canister for a canister's status and record metrics.
pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusResult, Error> {
    let args = CanisterStatusArgs {
        canister_id: canister_pid,
    };

    let status = mgmt::canister_status(&args).await.map_err(Error::from)?;
    SystemMetrics::increment(SystemMetricKind::CanisterStatus);
    Ok(status)
}

//
// ──────────────────────────────── CYCLES API ─────────────────────────────────
//

/// Returns the local canister's cycle balance (cheap).
#[must_use]
pub fn canister_cycle_balance() -> Cycles {
    crate::cdk::api::canister_cycle_balance().into()
}

/// Deposits cycles into a canister and records metrics.
pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), Error> {
    let args = DepositCyclesArgs {
        canister_id: canister_pid,
    };
    mgmt::deposit_cycles(&args, cycles)
        .await
        .map_err(Error::from)?;

    SystemMetrics::increment(SystemMetricKind::DepositCycles);
    Ok(())
}

/// Gets a canister's cycle balance (expensive: calls mgmt canister).
pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, Error> {
    let status = canister_status(canister_pid).await?;
    Ok(status.cycles.into())
}

//
// ────────────────────────────── TOPOLOGY LOOKUPS ─────────────────────────────
//

/// Queries the NNS registry for the subnet that this canister belongs to and records ICC metrics.
pub async fn get_current_subnet_pid() -> Result<Option<Principal>, Error> {
    let request = GetSubnetForCanisterRequest::new(crate::cdk::api::canister_self());

    let subnet_id_opt = Call::unbounded_wait(*NNS_REGISTRY_CANISTER, "get_subnet_for_canister")
        .with_arg(request)
        .await?
        .candid::<GetSubnetForCanisterResponse>()?
        .map_err(Error::CallFailed)?
        .subnet_id;

    if let Some(subnet_id) = subnet_id_opt {
        log!(Topic::Topology, Info, "get_current_subnet_pid: {subnet_id}");
    } else {
        log!(Topic::Topology, Warn, "get_current_subnet_pid: not found");
    }

    Ok(subnet_id_opt)
}

//
// ────────────────────────────── INSTALL / UNINSTALL ──────────────────────────
//

/// Installs or upgrades a canister with the given wasm + args and records metrics.
pub async fn install_code<T: ArgumentEncoder>(
    mode: CanisterInstallMode,
    canister_pid: Principal,
    wasm: &[u8],
    args: T,
) -> Result<(), Error> {
    let arg = encode_args(args)?;
    let install_args = InstallCodeArgs {
        mode,
        canister_id: canister_pid,
        wasm_module: WasmModule::from(wasm),
        arg,
    };

    mgmt::install_code(&install_args)
        .await
        .map_err(Error::from)?;

    let metric_kind = match mode {
        CanisterInstallMode::Install => SystemMetricKind::InstallCode,
        CanisterInstallMode::Reinstall => SystemMetricKind::ReinstallCode,
        CanisterInstallMode::Upgrade(_) => SystemMetricKind::UpgradeCode,
    };
    SystemMetrics::increment(metric_kind);

    Ok(())
}

/// Upgrades a canister to the provided wasm when the module hash differs.
///
/// No-op when the canister already runs the same module.
pub async fn upgrade_canister(canister_pid: Principal, wasm: &[u8]) -> Result<(), Error> {
    let status = canister_status(canister_pid).await?;
    if status.module_hash == Some(get_wasm_hash(wasm)) {
        log!(
            Topic::CanisterLifecycle,
            Info,
            "canister_upgrade: {canister_pid} already running target module"
        );

        return Ok(());
    }

    install_code(CanisterInstallMode::Upgrade(None), canister_pid, wasm, ()).await?;

    #[allow(clippy::cast_precision_loss)]
    let bytes_fmt = wasm.len() as f64 / 1_000.0;
    log!(
        Topic::CanisterLifecycle,
        Ok,
        "canister_upgrade: {canister_pid} ({bytes_fmt} KB) upgraded"
    );

    Ok(())
}

/// Uninstalls code from a canister and records metrics.
pub async fn uninstall_code(canister_pid: Principal) -> Result<(), Error> {
    let args = UninstallCodeArgs {
        canister_id: canister_pid,
    };

    mgmt::uninstall_code(&args).await.map_err(Error::from)?;
    SystemMetrics::increment(SystemMetricKind::UninstallCode);

    Ok(())
}

/// Deletes a canister (code + controllers) via the management canister and records metrics.
pub async fn delete_canister(canister_pid: Principal) -> Result<(), Error> {
    let args = DeleteCanisterArgs {
        canister_id: canister_pid,
    };

    mgmt::delete_canister(&args).await.map_err(Error::from)?;
    SystemMetrics::increment(SystemMetricKind::DeleteCanister);

    Ok(())
}

//
// ─────────────────────────────── SETTINGS API ────────────────────────────────
//

/// Updates canister settings via the management canister and records metrics.
pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), Error> {
    mgmt::update_settings(args).await.map_err(Error::from)?;
    SystemMetrics::increment(SystemMetricKind::UpdateSettings);
    Ok(())
}

//
// ──────────────────────────────── GENERIC HELPERS ────────────────────────────
//

/// Calls a method on a canister and candid-decodes the response into `T`.
pub async fn call_and_decode<T: CandidType + for<'de> candid::Deserialize<'de>>(
    pid: Principal,
    method: &str,
    arg: impl CandidType,
) -> Result<T, Error> {
    let response = Call::unbounded_wait(pid, method)
        .with_arg(arg)
        .await
        .map_err(Error::from)?;

    candid::decode_one(&response).map_err(Error::from)
}

// =============================================================================
// PROVISIONING (ROOT ORCHESTRATOR HELPERS)
// =============================================================================

pub mod provisioning {
    //! Provisioning helpers for creating, installing, and tearing down canisters.
    //!
    //! These routines bundle the multi-phase orchestration that root performs when
    //! scaling out the topology: reserving cycles, recording registry state,
    //! installing WASM modules, and cascading state updates to descendants.

    use crate::ops::prelude::*;
    use crate::types::Cycles;
    use crate::{
        Error,
        cdk::{api::canister_self, mgmt::CanisterInstallMode},
        config::Config,
        log::Topic,
        ops::{
            OpsError,
            config::ConfigOps,
            model::memory::{
                CanisterInitPayload,
                directory::{AppDirectoryOps, SubnetDirectoryOps},
                env::{EnvData, EnvOps},
                reserve::CanisterReserveOps,
                topology::SubnetCanisterRegistryOps,
            },
            sync::state::StateBundle,
            wasm::WasmOps,
        },
    };
    use candid::Principal;
    use thiserror::Error as ThisError;

    #[derive(Debug, ThisError)]
    pub enum ProvisioningError {
        #[error(transparent)]
        Other(#[from] Error),

        #[error("install failed for {pid}: {source}")]
        InstallFailed { pid: Principal, source: Error },
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
        let include_subnet = updated_role.is_none_or(|role| {
            ConfigOps::current_subnet()
                .map(|c| c.subnet_directory.contains(role))
                // default to true if config is unavailable to avoid skipping a needed rebuild
                .unwrap_or(true)
        });

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
    /// 1. Allocate a canister ID and cycles (preferring the reserve pool)
    /// 2. Install WASM + bootstrap initial state
    /// 3. Register canister in SubnetCanisterRegistry
    /// 4. Cascade topology + sync directories
    pub async fn create_and_install_canister(
        role: &CanisterRole,
        parent_pid: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, ProvisioningError> {
        // must have WASM module registered
        WasmOps::try_get(role)?;

        // Phase 1: allocation
        let pid = allocate_canister(role).await?;

        // Phase 2: installation
        if let Err(err) = install_canister(pid, role, parent_pid, extra_arg).await {
            return Err(ProvisioningError::InstallFailed { pid, source: err });
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
    pub async fn delete_canister(
        pid: Principal,
    ) -> Result<(Option<CanisterRole>, Option<Principal>), Error> {
        OpsError::require_root()?;
        let parent_pid = SubnetCanisterRegistryOps::get_parent(pid);

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

        Ok((removed_entry.map(|e| e.role), parent_pid))
    }

    /// Uninstall code from a canister without deleting it.
    pub async fn uninstall_canister(pid: Principal) -> Result<(), Error> {
        super::uninstall_code(pid).await?;

        log!(Topic::CanisterLifecycle, Ok, "🗑️ uninstall_canister: {pid}");

        Ok(())
    }

    //
    // ===========================================================================
    // PHASE 1 — ALLOCATION (Reserve → Create)
    // ===========================================================================
    //

    /// Allocate a canister ID and ensure it meets the initial cycle target.
    ///
    /// Reuses a canister from the reserve if available; otherwise creates a new one.
    pub async fn allocate_canister(role: &CanisterRole) -> Result<Principal, Error> {
        // use ConfigOps for a clean, ops-layer config lookup
        let cfg = ConfigOps::current_subnet_canister(role)?;

        let target = cfg.initial_cycles;

        // Reuse from reserve
        if let Some((pid, _)) = CanisterReserveOps::pop_first() {
            let mut current = super::get_cycles(pid).await?;

            if current < target {
                let missing = target.to_u128().saturating_sub(current.to_u128());
                if missing > 0 {
                    super::deposit_cycles(pid, missing).await?;
                    current = Cycles::new(current.to_u128() + missing);

                    log!(
                        Topic::CanisterReserve,
                        Ok,
                        "⚡ allocate_canister: topped up {pid} by {} to meet target {}",
                        Cycles::from(missing),
                        target
                    );
                }
            }

            log!(
                Topic::CanisterReserve,
                Ok,
                "⚡ allocate_canister: reusing {pid} from pool (current {current})"
            );

            return Ok(pid);
        }

        // Create new canister
        let pid = create_canister_with_configured_controllers(target).await?;
        log!(
            Topic::CanisterReserve,
            Info,
            "⚡ allocate_canister: pool empty"
        );

        Ok(pid)
    }

    /// Create a fresh canister on the IC with the configured controllers.
    async fn create_canister_with_configured_controllers(
        cycles: Cycles,
    ) -> Result<Principal, Error> {
        let mut controllers = Config::get().controllers.clone();
        controllers.push(canister_self()); // root always controls

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

        // Construct init payload
        let env = EnvData {
            prime_root_pid: Some(EnvOps::try_get_prime_root_pid()?),
            subnet_role: Some(EnvOps::try_get_subnet_role()?),
            subnet_pid: Some(EnvOps::try_get_subnet_pid()?),
            root_pid: Some(EnvOps::try_get_root_pid()?),
            canister_role: Some(role.clone()),
            parent_pid: Some(parent_pid),
        };

        let payload = CanisterInitPayload {
            env,
            app_directory: AppDirectoryOps::export(),
            subnet_directory: SubnetDirectoryOps::export(),
        };

        let module_hash = wasm.module_hash();

        // Register before install so init hooks can observe the registry; roll back on failure.
        // otherwise if the init() tries to create a canister via root, it will panic
        SubnetCanisterRegistryOps::register(pid, role, parent_pid, module_hash.clone());

        if let Err(err) = super::install_code(
            CanisterInstallMode::Install,
            pid,
            wasm.bytes(),
            (payload, extra_arg),
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
}
