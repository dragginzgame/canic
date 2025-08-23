use crate::{
    Error, Log,
    ic::mgmt::CanisterInstallMode,
    interface::{
        InterfaceError,
        ic::{IcError, canister_status, install_code},
    },
    log,
    utils::wasm::get_wasm_hash,
};
use candid::Principal;

/// upgrade_canister
pub async fn upgrade_canister(canister_pid: Principal, bytes: &[u8]) -> Result<(), Error> {
    // module_hash
    let canister_status = canister_status(canister_pid).await?;
    if canister_status.module_hash == Some(get_wasm_hash(bytes)) {
        Err(InterfaceError::IcError(IcError::WasmHashMatches))?;
    }

    // args
    install_code(CanisterInstallMode::Upgrade(None), canister_pid, bytes, ()).await?;

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
