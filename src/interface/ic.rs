use crate::{
    Log,
    cycles::format_cycles,
    helper::get_wasm_hash,
    ic::{
        call::{Call, CallFailed, CandidDecodeFailed, Error as CallError},
        mgmt::{
            self, CanisterInstallMode, CanisterSettings, CanisterStatusArgs, CanisterStatusResult,
            CreateCanisterArgs, DepositCyclesArgs, InstallCodeArgs, WasmModule,
        },
    },
    log,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// IcError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum IcError {
    #[error("call rejected: {0}")]
    CallFailed(String),

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

// canister_self
#[must_use]
pub fn canister_self() -> Principal {
    crate::ic::api::canister_self()
}

// canister_status
pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusResult, IcError> {
    let args = CanisterStatusArgs {
        canister_id: canister_pid,
    };
    let res = mgmt::canister_status(&args).await?;

    Ok(res)
}

// deposit_cycles
pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), IcError> {
    let args = DepositCyclesArgs {
        canister_id: canister_pid,
    };
    mgmt::deposit_cycles(&args, cycles).await?;

    Ok(())
}

// install_code
pub async fn install_code(args: &InstallCodeArgs) -> Result<(), IcError> {
    mgmt::install_code(args).await?;

    Ok(())
}

// module_hash
pub async fn module_hash(canister_id: Principal) -> Result<Option<Vec<u8>>, IcError> {
    let response = canister_status(canister_id).await?;

    Ok(response.module_hash)
}

///
/// create_canister
///

pub async fn create_canister(
    name: &str,
    bytes: &[u8],
    controllers: Vec<Principal>,
    parent_pid: Principal,
) -> Result<Principal, IcError> {
    //
    // create canister
    //

    let cycles = 5_000_000_000_000;
    let settings = Some(CanisterSettings {
        controllers: Some(controllers),
        ..Default::default()
    });
    let args = CreateCanisterArgs { settings };
    let canister_pid = mgmt::create_canister_with_extra_cycles(&args, cycles)
        .await?
        .canister_id;

    //
    // install code
    //

    let install_args = InstallCodeArgs {
        mode: CanisterInstallMode::Install,
        canister_id: canister_pid,
        wasm_module: WasmModule::from(bytes),
        arg: ::candid::utils::encode_args((canister_self(), parent_pid)).expect("args encode"),
    };
    mgmt::install_code(&install_args).await?;

    //
    // call init_async
    //

    Call::unbounded_wait(canister_pid, "init_async").await?;

    //
    // debug
    //

    #[allow(clippy::cast_precision_loss)]
    let bytes_fmt = bytes.len() as f64 / 1_000.0;
    log!(
        Log::Ok,
        "canister_create: {} created ({} KB) {} with {}",
        name,
        bytes_fmt,
        canister_pid,
        format_cycles(cycles)
    );

    Ok(canister_pid)
}

/// upgrade_canister
pub async fn upgrade_canister(canister_pid: Principal, bytes: &[u8]) -> Result<(), IcError> {
    // module_hash
    let module_hash = module_hash(canister_pid).await?;
    if module_hash == Some(get_wasm_hash(bytes)) {
        Err(IcError::WasmHashMatches)?;
    }

    // args
    let install_args = InstallCodeArgs {
        mode: CanisterInstallMode::Upgrade(None),
        canister_id: canister_pid,
        wasm_module: WasmModule::from(bytes),
        arg: vec![],
    };
    mgmt::install_code(&install_args).await?;

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
