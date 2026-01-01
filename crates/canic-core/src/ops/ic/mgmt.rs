//! ops::ic::mgmt
//!
//! Ops-level wrappers over IC management canister calls.
//! Adds metrics, logging, and normalizes errors into `Error`.

use crate::{
    Error,
    cdk::{
        mgmt::{CanisterInstallMode, CanisterStatusResult, UpdateSettingsArgs},
        types::Cycles,
    },
    infra,
    ops::prelude::*,
    storage::metrics::system::{SystemMetricKind, SystemMetrics},
};
use candid::{CandidType, Principal, utils::ArgumentEncoder};

//
// ──────────────────────────────── CREATE CANISTER ────────────────────────────
//

/// Create a canister with explicit controllers and an initial cycle balance.
pub async fn create_canister(
    controllers: Vec<Principal>,
    cycles: Cycles,
) -> Result<Principal, Error> {
    let pid = infra::ic::mgmt::create_canister(controllers, cycles).await?;

    SystemMetrics::increment(SystemMetricKind::CreateCanister);

    Ok(pid)
}

//
// ────────────────────────────── CANISTER STATUS ──────────────────────────────
//

/// Internal ops entrypoint used by workflow and other ops helpers.
pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusResult, Error> {
    let status = infra::ic::mgmt::canister_status(canister_pid).await?;

    SystemMetrics::increment(SystemMetricKind::CanisterStatus);

    Ok(status)
}

//
// ──────────────────────────────── CYCLES API ─────────────────────────────────
//

/// Returns the local canister's cycle balance (cheap).
#[must_use]
pub fn canister_cycle_balance() -> Cycles {
    infra::ic::mgmt::canister_cycle_balance()
}

/// Deposits cycles into a canister and records metrics.
pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), Error> {
    infra::ic::mgmt::deposit_cycles(canister_pid, cycles).await?;

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
    let seed = infra::ic::mgmt::raw_rand().await?;

    SystemMetrics::increment(SystemMetricKind::RawRand);

    Ok(seed)
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
    infra::ic::mgmt::install_code(mode, canister_pid, wasm, args).await?;

    let metric_kind = match mode {
        CanisterInstallMode::Install => SystemMetricKind::InstallCode,
        CanisterInstallMode::Reinstall => SystemMetricKind::ReinstallCode,
        CanisterInstallMode::Upgrade(_) => SystemMetricKind::UpgradeCode,
    };
    SystemMetrics::increment(metric_kind);

    Ok(())
}

/// Upgrades a canister to the provided wasm.
pub async fn upgrade_canister(canister_pid: Principal, wasm: &[u8]) -> Result<(), Error> {
    install_code(CanisterInstallMode::Upgrade(None), canister_pid, wasm, ()).await?;

    #[allow(clippy::cast_precision_loss)]
    let bytes_kb = wasm.len() as f64 / 1_000.0;
    log!(
        Topic::CanisterLifecycle,
        Ok,
        "canister_upgrade: {canister_pid} ({bytes_kb} KB) upgraded"
    );

    Ok(())
}

/// Uninstalls code from a canister and records metrics.
pub async fn uninstall_code(canister_pid: Principal) -> Result<(), Error> {
    infra::ic::mgmt::uninstall_code(canister_pid).await?;

    SystemMetrics::increment(SystemMetricKind::UninstallCode);

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "🗑️ uninstall_code: {canister_pid}"
    );

    Ok(())
}

/// Deletes a canister (code + controllers) via the management canister.
pub async fn delete_canister(canister_pid: Principal) -> Result<(), Error> {
    infra::ic::mgmt::delete_canister(canister_pid).await?;

    SystemMetrics::increment(SystemMetricKind::DeleteCanister);

    Ok(())
}

//
// ─────────────────────────────── SETTINGS API ────────────────────────────────
//

/// Updates canister settings via the management canister and records metrics.
pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), Error> {
    infra::ic::mgmt::update_settings(args).await?;

    SystemMetrics::increment(SystemMetricKind::UpdateSettings);

    Ok(())
}

//
// ──────────────────────────────── GENERIC HELPERS ────────────────────────────
//

/// Calls a method on a canister and candid-decodes the response into `T`.
pub async fn call_and_decode<T>(
    pid: Principal,
    method: &str,
    arg: impl CandidType,
) -> Result<T, Error>
where
    T: CandidType + for<'de> candid::Deserialize<'de>,
{
    let decoded = infra::ic::mgmt::call_and_decode(pid, method, arg).await?;

    Ok(decoded)
}
