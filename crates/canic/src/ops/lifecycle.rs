use crate::{
    VERSION,
    cdk::{api::canister_self, println},
    log,
    log::Level,
    memory::{
        Env,
        directory::{AppDirectory, SubnetDirectory},
        registry,
        topology::{SubnetCanisterRegistry, SubnetIdentity},
    },
    ops::{CanisterInitPayload, ext::cycles::CycleTrackerOps, root::reserve::CanisterReserveOps},
    runtime,
    types::{CanisterType, SubnetType},
};

/// root_init
/// Bootstraps the root canister runtime and environment.
pub fn root_init(identity: SubnetIdentity) {
    // --- Phase 1: Init base systems ---

    // log - clear some space
    println!("");
    println!("");
    println!("");
    log!(
        Level::Info,
        "🔧 --------------------- 'canic v{VERSION} -----------------------",
    );
    log!(Level::Info, "🏁 init: root ({identity:?})");

    // init
    runtime::init_eager_tls();
    registry::init_memory();

    // --- Phase 2: Env registration ---
    let self_pid = canister_self();
    Env::set_canister_type(CanisterType::ROOT);
    Env::set_root_pid(self_pid);

    match identity {
        SubnetIdentity::Prime => {
            Env::set_prime_root_pid(self_pid);
            Env::set_subnet_type(SubnetType::PRIME);
        }
        SubnetIdentity::Standard(params) => {
            Env::set_prime_root_pid(params.prime_root_pid);
            Env::set_subnet_type(params.subnet_type);
        }
        SubnetIdentity::Test => panic!("not sure what to do with test"),
    }

    SubnetCanisterRegistry::register_root(self_pid);

    // --- Phase 3: Service startup ---
    CycleTrackerOps::start();
    CanisterReserveOps::start();
}

/// root_post_upgrade
pub fn root_post_upgrade() {
    // --- Phase 1: Init base systems ---
    log!(Level::Info, "🏁 post_upgrade: root");
    runtime::init_eager_tls();

    // --- Phase 2: Env registration ---

    // --- Phase 3: Service startup ---
    CycleTrackerOps::start();
    CanisterReserveOps::start();
}

/// nonroot_init
pub fn nonroot_init(canister_type: CanisterType, payload: CanisterInitPayload) {
    // --- Phase 1: Init base systems ---
    log!(Level::Info, "🏁 init: {}", canister_type);
    runtime::init_eager_tls();
    registry::init_memory();

    // --- Phase 2: Payload registration ---
    Env::import(payload.env);
    AppDirectory::import(payload.app_directory);
    SubnetDirectory::import(payload.subnet_directory);

    // --- Phase 3: Service startup ---
    CycleTrackerOps::start();
}

/// nonroot_post_upgrade
pub fn nonroot_post_upgrade(canister_type: CanisterType) {
    // --- Phase 1: Init base systems ---
    log!(Level::Info, "🏁 post_upgrade: {}", canister_type);
    runtime::init_eager_tls();

    // --- Phase 2: Env registration ---

    // --- Phase 3: Service startup ---
    CycleTrackerOps::start();
}
