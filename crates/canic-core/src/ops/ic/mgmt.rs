use crate::{
    Error, PublicError,
    cdk::{
        mgmt::{CanisterInstallMode, CanisterStatusResult, UpdateSettingsArgs},
        types::Cycles,
    },
    infra::InfraError,
    infra::ic::mgmt as infra_mgmt,
    log,
    log::Topic,
    model::metrics::system::{SystemMetricKind, SystemMetrics},
    ops::ic::call::Call,
};
use candid::{CandidType, Principal, decode_one, utils::ArgumentEncoder};

//
// ──────────────────────────────── CREATE CANISTER ────────────────────────────
//

/// Create a canister with explicit controllers and an initial cycle balance.
pub async fn create_canister(
    controllers: Vec<Principal>,
    cycles: Cycles,
) -> Result<Principal, Error> {
    let pid = infra_mgmt::create_canister(controllers, cycles).await?;

    SystemMetrics::increment(SystemMetricKind::CreateCanister);

    Ok(pid)
}

//
// ────────────────────────────── CANISTER STATUS ──────────────────────────────
//

/// Query the management canister for a canister's status and record metrics.
pub(crate) async fn canister_status_internal(
    canister_pid: Principal,
) -> Result<CanisterStatusResult, Error> {
    let status = infra_mgmt::canister_status(canister_pid).await?;

    SystemMetrics::increment(SystemMetricKind::CanisterStatus);

    Ok(status)
}

pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusResult, PublicError> {
    canister_status_internal(canister_pid)
        .await
        .map_err(PublicError::from)
}

//
// ──────────────────────────────── CYCLES API ─────────────────────────────────
//

/// Returns the local canister's cycle balance (cheap).
#[must_use]
pub fn canister_cycle_balance() -> Cycles {
    infra_mgmt::canister_cycle_balance()
}

/// Deposits cycles into a canister and records metrics.
pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), Error> {
    infra_mgmt::deposit_cycles(canister_pid, cycles).await?;

    SystemMetrics::increment(SystemMetricKind::DepositCycles);

    Ok(())
}

/// Gets a canister's cycle balance (expensive: calls mgmt canister).
pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, Error> {
    let status = canister_status_internal(canister_pid).await?;

    Ok(status.cycles.into())
}

//
// ──────────────────────────────── RANDOMNESS ────────────────────────────────
//

/// Query the management canister for raw randomness and record metrics.
pub async fn raw_rand() -> Result<[u8; 32], Error> {
    let response = Call::unbounded_wait(Principal::management_canister(), "raw_rand")
        .await
        .map_err(InfraError::from)?;
    let bytes: Vec<u8> = decode_one(&response).map_err(InfraError::from)?;
    let len = bytes.len();
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| InfraError::from(infra_mgmt::MgmtInfraError::RawRandInvalidLength { len }))?;

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
    infra_mgmt::install_code(mode, canister_pid, wasm, args).await?;

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
    infra_mgmt::uninstall_code(canister_pid).await?;

    SystemMetrics::increment(SystemMetricKind::UninstallCode);

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "🗑️ uninstall_code: {canister_pid}"
    );

    Ok(())
}

/// Deletes a canister (code + controllers) via the management canister and records metrics.
pub async fn delete_canister(canister_pid: Principal) -> Result<(), Error> {
    infra_mgmt::delete_canister(canister_pid).await?;

    SystemMetrics::increment(SystemMetricKind::DeleteCanister);

    Ok(())
}

//
// ─────────────────────────────── SETTINGS API ────────────────────────────────
//

/// Updates canister settings via the management canister and records metrics.
pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), Error> {
    infra_mgmt::update_settings(args).await?;
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
        .map_err(InfraError::from)?;

    let decoded = candid::decode_one(&response).map_err(InfraError::from)?;

    Ok(decoded)
}
