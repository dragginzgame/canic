pub mod cycles;
pub mod log;
pub mod metrics;

use crate::{
    VERSION,
    cdk::{
        api::{canister_self, trap},
        println,
        types::Principal,
    },
    ids::{CanisterRole, SubnetRole},
    log::Topic,
    ops::{
        ic::{Network, build_network},
        service::TimerService,
        storage::{
            CanisterInitPayload,
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            env::{EnvData, EnvOps},
            memory::MemoryRegistryOps,
            topology::{SubnetCanisterRegistryOps, SubnetIdentity},
        },
    },
};
use canic_memory::runtime::init_eager_tls;

fn init_memory_or_trap(phase: &str) {
    if let Err(err) = MemoryRegistryOps::init_memory() {
        println!("[canic] FATAL: memory init failed during {phase}: {err}");
        let msg = format!("canic init failed during {phase}: memory init failed: {err}");
        trap(&msg);
    }
}

fn ensure_nonroot_env(canister_role: CanisterRole, mut env: EnvData) -> EnvData {
    let mut missing = Vec::new();
    if env.prime_root_pid.is_none() {
        missing.push("prime_root_pid");
    }
    if env.subnet_role.is_none() {
        missing.push("subnet_role");
    }
    if env.subnet_pid.is_none() {
        missing.push("subnet_pid");
    }
    if env.root_pid.is_none() {
        missing.push("root_pid");
    }
    if env.canister_role.is_none() {
        missing.push("canister_role");
    }
    if env.parent_pid.is_none() {
        missing.push("parent_pid");
    }

    if missing.is_empty() {
        return env;
    }

    assert!(
        build_network() != Some(Network::Ic),
        "nonroot init missing env fields on ic: {}",
        missing.join(", ")
    );

    let root_pid = Principal::from_slice(&[0xBB; 29]);
    let subnet_pid = Principal::from_slice(&[0xAA; 29]);

    env.prime_root_pid.get_or_insert(root_pid);
    env.subnet_role.get_or_insert(SubnetRole::PRIME);
    env.subnet_pid.get_or_insert(subnet_pid);
    env.root_pid.get_or_insert(root_pid);
    env.canister_role.get_or_insert(canister_role);
    env.parent_pid.get_or_insert(root_pid);

    env
}

/// root_init
/// Bootstraps the root canister runtime and environment.
pub fn root_init(identity: SubnetIdentity) {
    // --- Phase 1: Init base systems ---

    // log - clear some space
    println!("");
    println!("");
    println!("");
    crate::log!(
        Topic::Init,
        Info,
        "🔧 --------------------- 'canic v{VERSION} -----------------------",
    );
    crate::log!(Topic::Init, Info, "🏁 init: root ({identity:?})");

    // init
    init_eager_tls();
    init_memory_or_trap("root_init");

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
    TimerService::start_all_root();
}

/// root_post_upgrade
pub fn root_post_upgrade() {
    // --- Phase 1: Init base systems ---
    crate::log!(Topic::Init, Info, "🏁 post_upgrade: root");
    init_eager_tls();
    init_memory_or_trap("root_post_upgrade");

    // --- Phase 2: Env registration ---

    // --- Phase 3: Service startup ---
    TimerService::start_all_root();
}

/// nonroot_init
pub fn nonroot_init(canister_role: CanisterRole, payload: CanisterInitPayload) {
    // --- Phase 1: Init base systems ---
    crate::log!(Topic::Init, Info, "🏁 init: {}", canister_role);
    init_eager_tls();
    init_memory_or_trap("nonroot_init");

    // --- Phase 2: Payload registration ---
    let env = ensure_nonroot_env(canister_role, payload.env);
    EnvOps::import(env);
    AppDirectoryOps::import(payload.app_directory);
    SubnetDirectoryOps::import(payload.subnet_directory);

    // --- Phase 3: Service startup ---
    TimerService::start_all();
}

/// nonroot_post_upgrade
pub fn nonroot_post_upgrade(canister_role: CanisterRole) {
    // --- Phase 1: Init base systems ---
    crate::log!(Topic::Init, Info, "🏁 post_upgrade: {}", canister_role);
    init_eager_tls();
    init_memory_or_trap("nonroot_post_upgrade");

    // --- Phase 2: Env registration ---

    // --- Phase 3: Service startup ---
    TimerService::start_all();
}
