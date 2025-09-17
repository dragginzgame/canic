use crate::{
    Error, Log,
    cdk::mgmt::{self, CanisterInstallMode, CanisterSettings, CreateCanisterArgs},
    interface::{
        ic::{canister_status, install_code},
        prelude::*,
    },
    log,
    utils::wasm::get_wasm_hash,
};

///
/// create_canister
/// allocates PID + cycles + controllers
///
pub async fn create_canister(
    controllers: Vec<Principal>,
    cycles: Cycles,
) -> Result<Principal, Error> {
    let settings = Some(CanisterSettings {
        controllers: Some(controllers),
        ..Default::default()
    });
    let cc_args = CreateCanisterArgs { settings };

    // create
    let canister_pid = mgmt::create_canister_with_extra_cycles(&cc_args, cycles.to_u128())
        .await?
        .canister_id;

    Ok(canister_pid)
}

/// upgrade_canister
pub async fn upgrade_canister(canister_pid: Principal, bytes: &[u8]) -> Result<(), Error> {
    // module_hash
    let canister_status = canister_status(canister_pid).await?;
    if canister_status.module_hash == Some(get_wasm_hash(bytes)) {
        Err(InterfaceError::WasmHashMatches)?;
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
