//! Infra-scoped IC helpers.
//!
//! These wrappers provide low-level IC management canister calls and common
//! ICC call patterns without layering concerns.

use crate::{
    Error,
    cdk::{
        mgmt::{
            self, CanisterInstallMode, CanisterSettings, CanisterStatusArgs, CanisterStatusResult,
            CreateCanisterArgs, DeleteCanisterArgs, DepositCyclesArgs, InstallCodeArgs,
            UninstallCodeArgs, UpdateSettingsArgs, WasmModule,
        },
        types::Cycles,
        utils::wasm::get_wasm_hash,
    },
    infra::ic::call::Call,
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

    Ok(())
}

/// Upgrades a canister to the provided wasm when the module hash differs.
///
/// No-op when the canister already runs the same module.
pub async fn upgrade_canister(canister_pid: Principal, wasm: &[u8]) -> Result<(), Error> {
    let status = canister_status(canister_pid).await?;
    if status.module_hash == Some(get_wasm_hash(wasm)) {
        return Ok(());
    }

    install_code(CanisterInstallMode::Upgrade(None), canister_pid, wasm, ()).await?;

    Ok(())
}

/// Uninstalls code from a canister and records metrics.
pub async fn uninstall_code(canister_pid: Principal) -> Result<(), Error> {
    let args = UninstallCodeArgs {
        canister_id: canister_pid,
    };

    mgmt::uninstall_code(&args).await.map_err(Error::from)?;
    Ok(())
}

/// Deletes a canister (code + controllers) via the management canister and records metrics.
pub async fn delete_canister(canister_pid: Principal) -> Result<(), Error> {
    let args = DeleteCanisterArgs {
        canister_id: canister_pid,
    };

    mgmt::delete_canister(&args).await.map_err(Error::from)?;
    Ok(())
}

//
// ─────────────────────────────── SETTINGS API ────────────────────────────────
//

/// Updates canister settings via the management canister and records metrics.
pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), Error> {
    mgmt::update_settings(args).await.map_err(Error::from)?;
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
