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
    memory::{CanisterParent, CanisterState, SubnetDirectory, SubnetRegistry},
    state::canister::CanisterRegistry,
};
use candid::{Principal, encode_args};

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
/// raw_create_canister
/// allocates PID + cycles + controllers
///
async fn raw_create_canister() -> Result<Principal, Error> {
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

    Ok(canister_pid)
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
        "⚡ install_canister: installed {bytes_fmt} KB on {canister_pid} ({canister_type})"
    );

    Ok(())
}

///
/// register_installed_canister
/// updates SubnetRegistry + Directory
///
async fn register_installed_canister(
    canister_type: &CanisterType,
    canister_pid: Principal,
    parents: &[CanisterParent],
) -> Result<(), Error> {
    let uses_directory = CanisterRegistry::try_get(canister_type)?
        .attributes
        .uses_directory();

    // subnet directory
    if uses_directory {
        SubnetDirectory::can_insert(canister_type)?;
    }

    // SubnetRegistry (root only)
    SubnetRegistry::insert(
        canister_pid,
        canister_type,
        parents.last().map(|p| p.principal),
    );
    crate::log!(crate::Log::Warn, "subnet_registry inserting {canister_pid}");

    // insert into the SubnetDirectory
    // and then cascade it down from root
    if uses_directory {
        SubnetDirectory::insert(canister_type.clone(), canister_pid)?;
        let bundle = StateBundle::subnet_directory();
        update_canister(&canister_pid, &bundle).await?;

        cascade(&bundle).await?;
    }

    Ok(())
}

///
/// ic_create_canister_full
/// high-level: create + install + register
///
pub async fn ic_create_canister_full(
    canister_type: &CanisterType,
    parents: &[CanisterParent],
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    let canister_pid = raw_create_canister().await?;

    install_canister(canister_type, canister_pid, parents, extra_arg).await?;
    register_installed_canister(canister_type, canister_pid, parents).await?;

    log!(
        Log::Ok,
        "⚡ create_canister: {canister_type} {canister_pid} installed + registered"
    );

    Ok(canister_pid)
}

///
/// ic_create_canister_empty
/// high-level: create only (empty)
///
pub async fn ic_create_canister_empty() -> Result<Principal, Error> {
    let canister_pid = raw_create_canister().await?;
    log!(Log::Ok, "⚡ create_canister_empty: pid {canister_pid}");

    Ok(canister_pid)
}
