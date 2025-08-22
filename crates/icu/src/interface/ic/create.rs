use crate::{
    Error, Log,
    canister::CanisterType,
    config::Config,
    ic::mgmt::{
        self, CanisterInstallMode, CanisterSettings, CreateCanisterArgs, InstallCodeArgs,
        WasmModule,
    },
    interface::{
        InterfaceError,
        ic::IcError,
        state::{StateBundle, cascade, update_canister},
    },
    log,
    memory::{
        CanisterDirectory, CanisterPool, CanisterRegistry, CanisterState, canister::CanisterEntry,
    },
    state::canister::CanisterCatalog,
    utils::cycles::format_cycles,
};
use candid::{Principal, encode_args};

const CYCLES: u128 = 5_000_000_000_000;

///
/// CreateCanisterResult
///

pub struct CreateCanisterResult {
    pub canister_pid: Principal,
    pub cycles: u128,
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
        Err(InterfaceError::NotRoot)?;
    }

    //
    // Phase 0: Check Pool
    //
    let (canister_pid, cycles) = if let Some((pid, entry)) = CanisterPool::pop_first() {
        log!(Log::Ok, "⚡ reusing {pid} from pool ({entry:?})");

        (pid, entry.cycles)
    } else {
        let CreateCanisterResult {
            canister_pid,
            cycles,
        } = create_canister().await?;

        (canister_pid, cycles)
    };

    // Phase 1: insert with Created status
    register_created(canister_pid, canister_type, parents);

    // Phase 2: install wasm
    install_canister(canister_pid, canister_type, parents, extra_arg).await?;

    // Phase 3: mark as installed + cascade
    register_installed(canister_type, canister_pid).await?;

    log!(
        Log::Ok,
        "⚡ create_canister: {} ({}, {})",
        canister_pid,
        canister_type,
        format_cycles(cycles),
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
pub(super) async fn create_canister() -> Result<CreateCanisterResult, Error> {
    let controllers = get_controllers()?;
    let settings = Some(CanisterSettings {
        controllers: Some(controllers),
        ..Default::default()
    });
    let cc_args = CreateCanisterArgs { settings };

    // create
    let canister_pid = mgmt::create_canister_with_extra_cycles(&cc_args, CYCLES)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?
        .canister_id;

    Ok(CreateCanisterResult {
        canister_pid,
        cycles: CYCLES,
    })
}

///
/// install_canister
/// fetches wasm + encodes args + installs
///
pub(super) async fn install_canister(
    canister_pid: Principal,
    canister_type: &CanisterType,
    parents: &[CanisterEntry],
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    let bundle = StateBundle::all();
    let arg = encode_args((bundle, parents, extra_arg))
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    // fetch the canister by its type
    let canister = CanisterCatalog::try_get(canister_type)?;
    let bytes = canister.wasm;

    // install code
    let install_args = InstallCodeArgs {
        mode: CanisterInstallMode::Install,
        canister_id: canister_pid,
        wasm_module: WasmModule::from(bytes),
        arg,
    };
    mgmt::install_code(&install_args)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    log!(
        Log::Ok,
        "⚡ install_canister: {} ({}, {:2}KiB)",
        canister_pid,
        canister_type,
        bytes.len() as f64 / 1_024.0,
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
    let cfg = CanisterCatalog::try_get(canister_type)?;

    // flip to Installed
    CanisterRegistry::install(canister_pid, cfg.module_hash())?;

    // if this type uses the directory, insert + cascade
    if cfg.attributes.uses_directory {
        CanisterDirectory::insert(canister_type.clone(), canister_pid)?;

        let bundle = StateBundle::subnet_directory();
        update_canister(&canister_pid, &bundle).await?;
        cascade(&bundle).await?;
    }

    Ok(())
}
