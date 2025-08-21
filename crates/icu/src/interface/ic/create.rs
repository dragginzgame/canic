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
        CanisterPool, CanisterState, SubnetDirectory, SubnetRegistry,
        canister_state::CanisterParent, subnet_registry::CanisterStatus,
    },
    state::canister::CanisterRegistry,
    utils::cycles::format_cycles,
};
use candid::{Principal, encode_args};

///
/// CreateCanisterResult
///

pub struct CreateCanisterResult {
    pub canister_pid: Principal,
    pub cycles: u128,
}

///
/// create_canister_full
/// creates the canister, installs it, adds it to registries
///
pub async fn create_canister_full(
    canister_type: &CanisterType,
    parents: &[CanisterParent],
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    if !CanisterState::is_root() {
        Err(InterfaceError::NotRoot)?;
    }

    let CreateCanisterResult {
        canister_pid,
        cycles,
    } = create_canister().await?;

    // Phase 1: insert as pending
    register_pending_canister(canister_pid, canister_type, parents);

    // Phase 2: install wasm
    install_canister(canister_type, canister_pid, parents, extra_arg).await?;

    // Phase 3: mark installed + cascade
    mark_canister_installed(canister_type, canister_pid).await?;

    log!(
        Log::Ok,
        "⚡ create_canister_full: {} {} ({})",
        canister_pid,
        canister_type,
        format_cycles(cycles),
    );

    Ok(canister_pid)
}

///
/// create_canister_pool
/// creates an empty canister and registers it with the CanisterPool
///
pub async fn create_canister_pool() -> Result<Principal, Error> {
    if !CanisterState::is_root() {
        Err(InterfaceError::NotRoot)?;
    }

    let CreateCanisterResult {
        canister_pid,
        cycles,
    } = create_canister().await?;

    log!(
        Log::Ok,
        "⚡ create_canister_pool: pid {} ({})",
        canister_pid,
        format_cycles(cycles)
    );

    CanisterPool::register(canister_pid, cycles);

    Ok(canister_pid)
}

///
/// get_controllers
/// we get the hardcoded list from config, plus root
///
fn get_controllers() -> Result<Vec<Principal>, Error> {
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
async fn create_canister() -> Result<CreateCanisterResult, Error> {
    let cycles = 5_000_000_000_000;
    let controllers = get_controllers()?;
    let settings = Some(CanisterSettings {
        controllers: Some(controllers),
        ..Default::default()
    });
    let cc_args = CreateCanisterArgs { settings };

    // create
    let canister_pid = mgmt::create_canister_with_extra_cycles(&cc_args, cycles)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?
        .canister_id;

    Ok(CreateCanisterResult {
        canister_pid,
        cycles,
    })
}

///
/// install_canister
/// fetches wasm + encodes args + installs
///
async fn install_canister(
    canister_type: &CanisterType,
    canister_pid: Principal,
    parents: &[CanisterParent],
    extra_arg: Option<Vec<u8>>,
) -> Result<(), Error> {
    let bundle = StateBundle::all();
    let arg = encode_args((bundle, parents, extra_arg))
        .map_err(IcError::from)
        .map_err(InterfaceError::from)?;

    // fetch the canister by its type
    let canister = CanisterRegistry::try_get(canister_type)?;
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

    // log wasm size
    #[allow(clippy::cast_precision_loss)]
    let bytes_fmt = bytes.len() as f64 / 1_000.0;
    log!(
        Log::Ok,
        "⚡ install_canister: installed {bytes_fmt} KiB on {canister_pid} ({canister_type})"
    );

    Ok(())
}

///
/// register_pending_canister
///
/// Insert into SubnetRegistry immediately after creation,
/// before install_code runs.
///
fn register_pending_canister(
    canister_pid: Principal,
    canister_type: &CanisterType,
    parents: &[CanisterParent],
) {
    SubnetRegistry::register_pending(
        canister_pid,
        canister_type,
        parents.last().map(|p| p.principal),
    );
}

///
/// mark_canister_installed
///
/// Update SubnetRegistry entry from Pending → Installed,
/// then update SubnetDirectory + cascade if needed.
///
async fn mark_canister_installed(
    canister_type: &CanisterType,
    canister_pid: Principal,
) -> Result<(), Error> {
    // flip to Installed
    SubnetRegistry::set_status(canister_pid, CanisterStatus::Installed)?;

    // if this type uses the directory, insert + cascade
    let uses_directory = CanisterRegistry::try_get(canister_type)?
        .attributes
        .uses_directory();

    if uses_directory {
        SubnetDirectory::can_insert(canister_type)?;
        SubnetDirectory::insert(canister_type.clone(), canister_pid)?;

        let bundle = StateBundle::subnet_directory();
        update_canister(&canister_pid, &bundle).await?;
        cascade(&bundle).await?;
    }

    Ok(())
}
