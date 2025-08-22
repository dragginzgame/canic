mod create;
mod pool;

pub use create::*;
pub use pool::*;

use crate::{
    Error, Log,
    ic::{
        call::{CallFailed, CandidDecodeFailed, Error as CallError},
        mgmt::{
            self, CanisterInstallMode, CanisterStatusArgs, CanisterStatusResult, DepositCyclesArgs,
            InstallCodeArgs, UninstallCodeArgs, WasmModule, uninstall_code,
        },
    },
    interface::InterfaceError,
    log,
    utils::wasm::get_wasm_hash,
};
use candid::{Error as CandidError, Principal};
use thiserror::Error as ThisError;

///
/// IcError
///

#[derive(Debug, ThisError)]
pub enum IcError {
    #[error("cycles overflow")]
    CyclesOverflow,

    #[error("call rejected: {0}")]
    CallFailed(String),

    #[error("candid error: {0}")]
    CandidError(String),

    #[error("candid error: {0}")]
    CandidDecodeFailed(String),

    #[error("wasm hash matches")]
    WasmHashMatches,
}

impl From<CallFailed> for IcError {
    fn from(error: CallFailed) -> Self {
        Self::CallFailed(error.to_string())
    }
}

impl From<CandidError> for IcError {
    fn from(error: CandidError) -> Self {
        Self::CandidError(error.to_string())
    }
}

impl From<CandidDecodeFailed> for IcError {
    fn from(error: CandidDecodeFailed) -> Self {
        Self::CandidDecodeFailed(error.to_string())
    }
}

impl From<CallError> for IcError {
    fn from(error: CallError) -> Self {
        Self::CallFailed(error.to_string())
    }
}

// canister_status
pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusResult, Error> {
    let args = CanisterStatusArgs {
        canister_id: canister_pid,
    };
    let res = mgmt::canister_status(&args)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    Ok(res)
}

// get_cycles
async fn get_cycles(canister_pid: Principal) -> Result<u128, Error> {
    let status = canister_status(canister_pid).await?;

    let cycles: u128 = status
        .cycles
        .0
        .try_into()
        .map_err(|_| IcError::CyclesOverflow)
        .map_err(InterfaceError::IcError)?;

    Ok(cycles)
}

// deposit_cycles
pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), Error> {
    let args = DepositCyclesArgs {
        canister_id: canister_pid,
    };

    mgmt::deposit_cycles(&args, cycles)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    Ok(())
}

/// upgrade_canister
pub async fn upgrade_canister(canister_pid: Principal, bytes: &[u8]) -> Result<(), Error> {
    // module_hash
    let canister_status = canister_status(canister_pid).await?;
    if canister_status.module_hash == Some(get_wasm_hash(bytes)) {
        Err(InterfaceError::IcError(IcError::WasmHashMatches))?;
    }

    // args
    let install_args = InstallCodeArgs {
        mode: CanisterInstallMode::Upgrade(None),
        canister_id: canister_pid,
        wasm_module: WasmModule::from(bytes),
        arg: vec![],
    };
    mgmt::install_code(&install_args)
        .await
        .map_err(|e| IcError::CallFailed(e.to_string()))
        .map_err(InterfaceError::IcError)?;

    // debug
    #[allow(clippy::cast_precision_loss)]
    let bytes_fmt = bytes.len() as f64 / 1_000.0;
    log!(
        Log::Ok,
        "canister_upgrade: {} ({} KB) upgraded",
        canister_pid,
        bytes_fmt,
    );

    Ok(())
}

// uninstall_canister
pub async fn uninstall_canister(canister_pid: Principal) -> Result<(), Error> {
    let args = UninstallCodeArgs {
        canister_id: canister_pid,
    };

    uninstall_code(&args)
        .await
        .map_err(|e| IcError::CallFailed(e.to_string()))
        .map_err(InterfaceError::IcError)?;

    Ok(())
}
