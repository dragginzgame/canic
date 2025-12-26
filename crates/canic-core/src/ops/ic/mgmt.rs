//! Ops-scoped IC helpers.
//!
//! These wrappers attach ops-level concerns such as metrics recording around
//! IC management canister calls and common ICC call patterns.

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
    workflow::CanisterInitPayload,
};
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};

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
// ──────────────────────────────── RANDOMNESS ────────────────────────────────
//

/// Query the management canister for raw randomness and record metrics.
pub async fn raw_rand() -> Result<[u8; 32], Error> {
    let response = Call::unbounded_wait(Principal::management_canister(), "raw_rand").await?;
    let bytes: Vec<u8> = decode_one(&response)?;
    let len = bytes.len();
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| Error::CustomError(format!("raw_rand returned {len} bytes")))?;

    Ok(seed)
}

//
// ────────────────────────────── TOPOLOGY LOOKUPS ─────────────────────────────
//

/// Queries the NNS registry for the subnet that this canister belongs to and records ICC metrics.
pub async fn try_get_current_subnet_pid() -> Result<Option<Principal>, Error> {
    let request = GetSubnetForCanisterRequest::new(crate::cdk::api::canister_self());

    let subnet_id_opt = Call::unbounded_wait(*NNS_REGISTRY_CANISTER, "get_subnet_for_canister")
        .with_arg(request)
        .await?
        .candid::<GetSubnetForCanisterResponse>()?
        .map_err(Error::CallFailed)?
        .subnet_id;

    if let Some(subnet_id) = subnet_id_opt {
        log!(
            Topic::Topology,
            Info,
            "try_get_current_subnet_pid: {subnet_id}"
        );
    } else {
        log!(
            Topic::Topology,
            Warn,
            "try_get_current_subnet_pid: not found"
        );
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

/// Installs or reinstalls a Canic non-root canister with the standard init args.
pub async fn install_canic_code(
    mode: CanisterInstallMode,
    canister_pid: Principal,
    wasm: &[u8],
    payload: CanisterInitPayload,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    install_code(mode, canister_pid, wasm, (payload, extra_arg)).await
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

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "🗑️ uninstall_canister: {canister_pid}"
    );

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
