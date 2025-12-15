use crate::{
    VERSION,
    cdk::{api::canister_self, println},
    ids::{CanisterRole, SubnetRole},
    log,
    log::Topic,
    model::memory::topology::SubnetIdentity,
    ops::{
        model::memory::{
            CanisterInitPayload,
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            env::EnvOps,
            registry::MemoryRegistryOps,
            topology::SubnetCanisterRegistryOps,
        },
        service::TimerService,
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
    EnvOps::set_canister_role(CanisterRole::ROOT);
    EnvOps::set_root_pid(self_pid);

    match identity {
        SubnetIdentity::Prime => {
            EnvOps::set_prime_root_pid(self_pid);
            EnvOps::set_subnet_role(SubnetRole::PRIME);
            EnvOps::set_subnet_pid(self_pid);
        }
        SubnetIdentity::Standard(params) => {
            EnvOps::set_prime_root_pid(params.prime_root_pid);
            EnvOps::set_subnet_role(params.subnet_type);
            EnvOps::set_subnet_pid(self_pid);
        }
        SubnetIdentity::Manual(subnet_pid) => {
            EnvOps::set_prime_root_pid(self_pid);
            EnvOps::set_subnet_role(SubnetRole::PRIME);
            EnvOps::set_subnet_pid(subnet_pid);
        }
    }

    SubnetCanisterRegistryOps::register_root(self_pid);

    // --- Phase 3: Service startup ---
    if let Err(err) = TimerService::start_all_root() {
        log!(Topic::Init, Warn, "timer startup failed (root): {err}");
    }
}

/// root_post_upgrade
pub fn root_post_upgrade() {
    // --- Phase 1: Init base systems ---
    log!(Topic::Init, Info, "🏁 post_upgrade: root");
    init_eager_tls();
    MemoryRegistryOps::init_memory().unwrap();

    // --- Phase 2: Env registration ---

    // --- Phase 3: Service startup ---
    if let Err(err) = TimerService::start_all_root() {
        log!(Topic::Init, Warn, "timer startup failed (root): {err}");
    }
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
    if let Err(err) = TimerService::start_all() {
        log!(Topic::Init, Warn, "timer startup failed (nonroot): {err}");
    }
}

/// nonroot_post_upgrade
pub fn nonroot_post_upgrade(canister_type: CanisterRole) {
    // --- Phase 1: Init base systems ---
    log!(Topic::Init, Info, "🏁 post_upgrade: {}", canister_type);
    init_eager_tls();
    MemoryRegistryOps::init_memory().unwrap();

    // --- Phase 2: Env registration ---

    // --- Phase 3: Service startup ---
    if let Err(err) = TimerService::start_all() {
        log!(Topic::Init, Warn, "timer startup failed (nonroot): {err}");
    }
}
