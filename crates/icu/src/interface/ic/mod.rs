mod create;
mod pool;
mod upgrade;

pub use create::*;
pub use pool::*;
pub use upgrade::*;

use crate::{
    Error,
    ic::{
        call::{CallFailed, CandidDecodeFailed, Error as CallError},
        mgmt::{
            self, CanisterInstallMode, CanisterStatusArgs, CanisterStatusResult, DepositCyclesArgs,
            InstallCodeArgs, UninstallCodeArgs, WasmModule,
        },
    },
    interface::InterfaceError,
    types::Cycles,
};
use candid::{Error as CandidError, Principal, utils::ArgumentEncoder};
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

// canister_cycles_balance
#[must_use]
pub fn canister_cycle_balance() -> Cycles {
    crate::ic::api::canister_cycle_balance().into()
}

// deposit_cycles
pub async fn deposit_cycles(canister_pid: Principal, cycles: Cycles) -> Result<(), Error> {
    let args = DepositCyclesArgs {
        canister_id: canister_pid,
    };

    mgmt::deposit_cycles(&args, cycles.as_u128())
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    Ok(())
}

// encode_args
pub fn encode_args<T: ArgumentEncoder>(args: T) -> Result<Vec<u8>, Error> {
    let encoded = candid::encode_args(args)
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    Ok(encoded)
}

// get_cycles
// (an update call, don't use for local balances)
async fn get_cycles(canister_pid: Principal) -> Result<Cycles, Error> {
    let status = canister_status(canister_pid).await?;

    let cycles = status
        .cycles
        .try_into()
        .map_err(|_| IcError::CyclesOverflow)
        .map_err(InterfaceError::IcError)?;

    Ok(cycles)
}

// install_code
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
        .map_err(|e| IcError::CallFailed(e.to_string()))
        .map_err(InterfaceError::IcError)?;

    Ok(())
}

// uninstall_code
pub async fn uninstall_code(canister_pid: Principal) -> Result<(), Error> {
    let args = UninstallCodeArgs {
        canister_id: canister_pid,
    };

    mgmt::uninstall_code(&args)
        .await
        .map_err(|e| IcError::CallFailed(e.to_string()))
        .map_err(InterfaceError::IcError)?;

    Ok(())
}
