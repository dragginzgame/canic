use crate::{
    VERSION,
    cdk::{api::canister_self, println},
    ids::{CanisterRole, SubnetRole},
    log,
    log::Topic,
    model::memory::topology::SubnetIdentity,
    ops::{
        CanisterInitPayload,
        model::memory::cycles::CycleTrackerOps,
        model::memory::{
            EnvOps,
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            registry::MemoryRegistryOps,
            reserve::CanisterReserveOps,
            topology::SubnetCanisterRegistryOps,
        },
    },
};
use canic_memory::runtime::init_eager_tls;

/// root_init
/// Bootstraps the root canister runtime and environment.
pub fn root_init(identity: SubnetIdentity) {
    // --- Phase 1: Init base systems ---

    // log - clear some space
    println!("");
    println!("");
    println!("");
    log!(
        Topic::Init,
        Info,
        "🔧 --------------------- 'canic v{VERSION} -----------------------",
    );
    log!(Topic::Init, Info, "🏁 init: root ({identity:?})");

    // init
    init_eager_tls();
    MemoryRegistryOps::init_memory().unwrap();

    // --- Phase 2: Env registration ---
    let self_pid = canister_self();
    EnvOps::set_canister_type(CanisterRole::ROOT);
    EnvOps::set_root_pid(self_pid);

    match identity {
        SubnetIdentity::Prime => {
            EnvOps::set_prime_root_pid(self_pid);
            EnvOps::set_subnet_type(SubnetRole::PRIME);
            EnvOps::set_subnet_pid(self_pid);
        }
        SubnetIdentity::Standard(params) => {
            EnvOps::set_prime_root_pid(params.prime_root_pid);
            EnvOps::set_subnet_type(params.subnet_type);
            EnvOps::set_subnet_pid(self_pid);
        }
        SubnetIdentity::Manual(subnet_pid) => {
            EnvOps::set_prime_root_pid(self_pid);
            EnvOps::set_subnet_type(SubnetRole::PRIME);
            EnvOps::set_subnet_pid(subnet_pid);
        }
    }

    SubnetCanisterRegistryOps::register_root(self_pid);

    // --- Phase 3: Service startup ---
    CycleTrackerOps::start();
    CanisterReserveOps::start();
}

/// root_post_upgrade
pub fn root_post_upgrade() {
    // --- Phase 1: Init base systems ---
    log!(Topic::Init, Info, "🏁 post_upgrade: root");
    init_eager_tls();
    MemoryRegistryOps::init_memory().unwrap();

    // --- Phase 2: Env registration ---

    // --- Phase 3: Service startup ---
    CycleTrackerOps::start();
    CanisterReserveOps::start();
}

/// nonroot_init
pub fn nonroot_init(canister_type: CanisterRole, payload: CanisterInitPayload) {
    // --- Phase 1: Init base systems ---
    log!(Topic::Init, Info, "🏁 init: {}", canister_type);
    init_eager_tls();
    MemoryRegistryOps::init_memory().unwrap();

    // --- Phase 2: Payload registration ---
    EnvOps::import(payload.env);
    AppDirectoryOps::import(payload.app_directory);
    SubnetDirectoryOps::import(payload.subnet_directory);

    // --- Phase 3: Service startup ---
    CycleTrackerOps::start();
}

/// nonroot_post_upgrade
pub fn nonroot_post_upgrade(canister_type: CanisterRole) {
    // --- Phase 1: Init base systems ---
    log!(Topic::Init, Info, "🏁 post_upgrade: {}", canister_type);
    init_eager_tls();
    MemoryRegistryOps::init_memory().unwrap();

    // --- Phase 2: Env registration ---

    // --- Phase 3: Service startup ---
    CycleTrackerOps::start();
}
