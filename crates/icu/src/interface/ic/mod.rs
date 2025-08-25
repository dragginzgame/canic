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
use candid::{Principal, utils::ArgumentEncoder};

// canister_status
pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusResult, Error> {
    let args = CanisterStatusArgs {
        canister_id: canister_pid,
    };
    let res = mgmt::canister_status(&args)
        .await
        .map_err(InterfaceError::from)?;

    Ok(res)
}

// canister_cycles_balance
#[must_use]
pub fn canister_cycle_balance() -> Cycles {
    crate::cdk::api::canister_cycle_balance().into()
}

// deposit_cycles
pub async fn deposit_cycles(canister_pid: Principal, cycles: Cycles) -> Result<(), Error> {
    let args = DepositCyclesArgs {
        canister_id: canister_pid,
    };

    mgmt::deposit_cycles(&args, cycles.as_u128())
        .await
        .map_err(InterfaceError::from)?;

    Ok(())
}

// encode_args
pub fn encode_args<T: ArgumentEncoder>(args: T) -> Result<Vec<u8>, Error> {
    let encoded = candid::encode_args(args).map_err(InterfaceError::from)?;

    Ok(encoded)
}

// get_cycles
// (an update call, don't use for local balances)
pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, Error> {
    let status = canister_status(canister_pid).await?;

    let cycles = status
        .cycles
        .try_into()
        .map_err(|_| InterfaceError::CyclesOverflow)?;

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
        .map_err(InterfaceError::CallError)?;

    Ok(())
}

// uninstall_code
pub async fn uninstall_code(canister_pid: Principal) -> Result<(), Error> {
    let args = UninstallCodeArgs {
        canister_id: canister_pid,
    };

    mgmt::uninstall_code(&args)
        .await
        .map_err(InterfaceError::CallError)?;

    Ok(())
}
