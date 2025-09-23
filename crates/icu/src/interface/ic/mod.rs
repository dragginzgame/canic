mod canister;
mod cycles;
mod helper;
mod icp;
mod sns;

pub use canister::*;
pub use cycles::*;
pub use helper::*;
pub use icp::*;
pub use sns::*;

use crate::{
    Error,
    cdk::mgmt::{
        self, CanisterInstallMode, CanisterStatusArgs, CanisterStatusResult, DepositCyclesArgs,
        InstallCodeArgs, UninstallCodeArgs, WasmModule,
    },
    interface::prelude::*,
};
use candid::{Principal, decode_one, encode_args, utils::ArgumentEncoder};

// canister_status
pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusResult, Error> {
    let args = CanisterStatusArgs {
        canister_id: canister_pid,
    };
    let res = mgmt::canister_status(&args).await?;

    Ok(res)
}

// canister_cycles_balance
#[must_use]
pub fn canister_cycle_balance() -> Cycles {
    crate::cdk::api::canister_cycle_balance().into()
}

// deposit_cycles
pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), Error> {
    let args = DepositCyclesArgs {
        canister_id: canister_pid,
    };

    mgmt::deposit_cycles(&args, cycles).await?;

    Ok(())
}

// get_cycles
// (an update call, don't use for local balances)
pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, Error> {
    let status = canister_status(canister_pid).await?;
    let cycles: Cycles = status.cycles.into();

    Ok(cycles)
}

/// call_and_decode
/// Generic helper for calls that return `Result<T, Error>`
pub async fn call_and_decode<T: CandidType + for<'de> candid::Deserialize<'de>>(
    pid: Principal,
    method: &str,
    arg: impl CandidType,
) -> Result<T, Error> {
    let response = Call::unbounded_wait(pid, method)
        .with_arg(arg)
        .await
        .map_err(Error::from)?;

    decode_one(&response).map_err(Error::from)
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

    mgmt::install_code(&install_args).await?;

    Ok(())
}

// uninstall_code
pub async fn uninstall_code(canister_pid: Principal) -> Result<(), Error> {
    let args = UninstallCodeArgs {
        canister_id: canister_pid,
    };

    mgmt::uninstall_code(&args).await?;

    Ok(())
}
