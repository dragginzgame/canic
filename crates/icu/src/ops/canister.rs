use crate::{
    Error,
    cdk::mgmt::CanisterInstallMode,
    config::Config,
    interface::{ic::install_code, prelude::*},
    memory::{AppState, CanisterPool, SubnetDirectory, SubnetRegistry},
    ops::sync::{SyncBundle, cascade_children},
    state::wasm::WasmRegistry,
};

///
/// allocate_canister
/// firstly looks in the pool to find a canister
///
async fn allocate_canister(ty: &CanisterType) -> Result<(Principal, Cycles), Error> {
    // try pool first
    if let Some((pid, entry)) = CanisterPool::pop_first() {
        log!(Log::Ok, "⚡ reusing {pid} from pool ({entry:?})");

        return Ok((pid, entry.cycles));
    }

    // fallback: fresh canister
    let canister = Config::try_get_canister(ty)?;
    let cycles = canister.initial_cycles;
    let pid = create_canister(cycles.clone()).await?;

    Ok((pid, cycles))
}

///
/// create_and_install_canister
/// creates the canister, installs it, adds it to registries
///
pub async fn create_and_install_canister(
    canister_type: &CanisterType,
    parent: Principal,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    // Validate canister type and wasm presence up-front so the pool path
    // cannot bypass config/wasm validation.
    let _ = Config::try_get_canister(canister_type)?; // must exist in config
    let _ = WasmRegistry::try_get(canister_type)?; // must have a wasm registered

    // Phase 0: allocate canister id + cycles
    let (canister_pid, cycles) = allocate_canister(canister_type).await?;

    // Phase 1: insert with Created status
    register_created(canister_pid, canister_type, parent);

    // Phase 2: install wasm
    install_canister(canister_pid, canister_type, extra_arg).await?;

    // Phase 3: mark as installed + cascade
    register_installed(canister_pid, canister_type).await?;

    log!(
        Log::Ok,
        "⚡ create_canister: {canister_pid} ({canister_type}, {cycles})",
    );

    Ok(canister_pid)
}

///
/// get_controllers
/// we get the hardcoded list from config, plus root
///
pub fn get_controllers() -> Result<Vec<Principal>, Error> {
    let config = Config::try_get()?;
    let mut controllers = config.controllers.clone();

    // push root
    let root_pid = SubnetDirectory::try_get_root()?.pid;
    controllers.push(root_pid);

    Ok(controllers)
}

///
/// create_canister
/// allocates PID + cycles + controllers
///
pub(super) async fn create_canister(cycles: Cycles) -> Result<Principal, Error> {
    let controllers = get_controllers()?;

    // create
    let canister_pid = crate::interface::ic::create_canister(controllers, cycles).await?;

    Ok(canister_pid)
}

///
/// register_created
///
/// Insert into SubnetRegistry immediately after creation,
/// before install_code runs.
///
pub(super) fn register_created(
    canister_pid: Principal,
    canister_type: &CanisterType,
    parent_pid: Principal,
) {
    SubnetRegistry::create(canister_pid, canister_type, parent_pid);
}

///
/// register_installed
///
/// Update SubnetRegistry entry from Pending → Installed,
/// then update SubnetDirectory + cascade if needed.
///

pub(super) async fn register_installed(
    canister_pid: Principal,
    canister_type: &CanisterType,
) -> Result<(), Error> {
    let canister = Config::try_get_canister(canister_type)?;
    let wasm = WasmRegistry::try_get(canister_type)?;

    // flip to Installed
    SubnetRegistry::install(canister_pid, wasm.module_hash())?;

    // if the subnet directory has changed, we need to do a full cascade
    if canister.uses_directory {
        let sd = SubnetRegistry::subnet_directory();
        let bundle = SyncBundle::default().with_subnet_directory(sd);

        cascade_children(&bundle).await?;
    }

    Ok(())
}

///
/// install_canister
/// fetches wasm + encodes args + installs
///
#[allow(clippy::cast_precision_loss)]
pub(super) async fn install_canister(
    canister_pid: Principal,
    canister_type: &CanisterType,
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    // fetch the canister by its type
    let wasm = WasmRegistry::try_get(canister_type)?;

    // create the bundle as we're on root
    let ast = AppState::export();
    let sd = SubnetRegistry::subnet_directory();
    let sp = SubnetRegistry::subnet_parents(canister_pid);
    let sc = SubnetRegistry::subnet_children(canister_pid);

    let bundle = SyncBundle::default()
        .with_app_state(ast)
        .with_subnet_children(sc)
        .with_subnet_directory(sd.clone())
        .with_subnet_parents(sp);

    // install code
    let args = (bundle, extra_arg);
    install_code(
        CanisterInstallMode::Install,
        canister_pid,
        wasm.bytes(),
        args,
    )
    .await?;

    log!(
        Log::Ok,
        "⚡ install_canister: {} ({}, {:.2}KiB)",
        canister_pid,
        canister_type,
        wasm.len() as f64 / 1_024.0,
    );

    Ok(())
}
