//! IC Interfaces
//! Thin wrappers around the management canister and network-specific helpers.

pub mod call;
pub mod canister;
pub mod cycles;
pub mod helper;
pub mod sns;
pub mod timer;

pub use helper::*;

use crate::{
    Error,
    cdk::mgmt::{
        self, CanisterInstallMode, CanisterStatusArgs, CanisterStatusResult, DeleteCanisterArgs,
        DepositCyclesArgs, InstallCodeArgs, UninstallCodeArgs, UpdateSettingsArgs, WasmModule,
    },
    env::nns::NNS_REGISTRY_CANISTER,
    interface::prelude::*,
    log,
    log::Topic,
    spec::nns::{GetSubnetForCanisterRequest, GetSubnetForCanisterResponse},
};
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};

//
// ────────────────────────────── CANISTER STATUS ──────────────────────────────
//

/// Query the management canister for a canister's status.
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

/// Deposits cycles into a canister.
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
// ────────────────────────────── TOPOLOGY LOOKUPS ─────────────────────────────
//

/// Queries the NNS registry for the subnet that this canister belongs to.
pub async fn get_current_subnet_pid() -> Result<Option<Principal>, Error> {
    let request = GetSubnetForCanisterRequest::new(canister_self());

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

/// Installs or upgrades a canister with the given wasm + args.
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

/// Uninstalls code from a canister.
pub async fn uninstall_code(canister_pid: Principal) -> Result<(), Error> {
    let args = UninstallCodeArgs {
        canister_id: canister_pid,
    };

    mgmt::uninstall_code(&args).await.map_err(Error::from)?;

    Ok(())
}

/// Deletes a canister (code + controllers) via the management canister.
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

/// Updates canister settings via the management canister.
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

    decode_one(&response).map_err(Error::from)
}
