use crate::{
    Error,
    cdk::mgmt::CanisterInstallMode,
    config::Config,
    interface::{ic::install_code, prelude::*},
    memory::{
        CanisterDirectory, CanisterPool, CanisterRegistry, CanisterState, canister::CanisterEntry,
    },
    ops::{
        prelude::*,
        state::{StateBundle, cascade, update_canister},
    },
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
    let pid = create_canister(cycles).await?;

    Ok((pid, cycles))
}

///
/// create_and_install_canister
/// creates the canister, installs it, adds it to registries
///
pub async fn create_and_install_canister(
    canister_type: &CanisterType,
    parents: &[CanisterEntry],
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    if !CanisterState::is_root() {
        Err(OpsError::NotRoot)?;
    }

    // Validate canister type and wasm presence up-front so the pool path
    // cannot bypass config/wasm validation.
    let _ = Config::try_get_canister(canister_type)?; // must exist in config
    let _ = WasmRegistry::try_get(canister_type)?; // must have a wasm registered

    // Phase 0: allocate canister id + cycles
    let (canister_pid, cycles) = allocate_canister(canister_type).await?;

    // Phase 1: insert with Created status
    register_created(canister_pid, canister_type, parents);

    // Phase 2: install wasm
    install_canister(canister_pid, canister_type, parents, extra_arg).await?;

    // Phase 3: mark as installed + cascade
    register_installed(canister_type, canister_pid).await?;

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
    let root_pid = CanisterState::get_root_pid();
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
/// install_canister
/// fetches wasm + encodes args + installs
///
#[allow(clippy::cast_precision_loss)]
pub(super) async fn install_canister(
    canister_pid: Principal,
    canister_type: &CanisterType,
    parents: &[CanisterEntry],
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    // fetch the canister by its type
    let wasm = WasmRegistry::try_get(canister_type)?;

    // install code
    let bundle = StateBundle::all();
    let args = (bundle, parents, extra_arg);
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

///
/// register_created
///
/// Insert into SubnetRegistry immediately after creation,
/// before install_code runs.
///
pub(super) fn register_created(
    canister_pid: Principal,
    canister_type: &CanisterType,
    parents: &[CanisterEntry],
) {
    CanisterRegistry::create(
        canister_pid,
        canister_type,
        parents.last().map(|p| p.principal),
    );
}

///
/// register_installed
///
/// Update SubnetRegistry entry from Pending → Installed,
/// then update SubnetDirectory + cascade if needed.
///
pub(super) async fn register_installed(
    canister_type: &CanisterType,
    canister_pid: Principal,
) -> Result<(), Error> {
    let canister = Config::try_get_canister(canister_type)?;
    let wasm = WasmRegistry::try_get(canister_type)?;

    // flip to Installed
    CanisterRegistry::install(canister_pid, wasm.module_hash())?;

    // if this type uses the directory, generate fresh view + cascade
    if canister.uses_directory {
        let view = CanisterDirectory::generate_from_registry();
        let bundle = StateBundle::with_canister_directory(view);
        update_canister(&canister_pid, &bundle).await?;
        cascade(&bundle).await?;
    }

    Ok(())
}
