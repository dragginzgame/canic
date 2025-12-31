pub mod cycles;
pub mod log;
pub mod random;

use crate::{
    VERSION,
    cdk::{
        api::{canister_self, trap},
        println,
    },
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::{CanisterRole, SubnetRole},
    log::Topic,
    ops::{
        adapter::directory::{app_directory_from_view, subnet_directory_from_view},
        runtime::{env::EnvOps, memory::MemoryOps},
        storage::{
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            registry::SubnetRegistryOps,
        },
    },
    workflow,
};
use canic_memory::runtime::init_eager_tls;

///
/// Runtime
/// Coordinates periodic background services (timers) for Canic canisters.
///

pub struct Runtime;

impl Runtime {
    /// Start timers that should run on all canisters.
    pub fn start_all() {
        workflow::runtime::cycles::scheduler::start();
        workflow::runtime::log::retention::start();
        workflow::runtime::random::scheduler::start();
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() {
        EnvOps::require_root().unwrap_or_else(|e| fatal("start_all_root", e));

        // start shared timers too
        Self::start_all();

        // root-only services
        workflow::pool::scheduler::start();
    }
}

//
// ─────────────────────────────────────────────────────────────
// Fatal helpers (lifecycle boundary)
// ─────────────────────────────────────────────────────────────
//

fn fatal(phase: &str, err: impl std::fmt::Display) -> ! {
    let msg = format!("canic init failed during {phase}: {err}");
    println!("[canic] FATAL: {msg}");
    trap(&msg);
}

fn init_memory_or_trap(phase: &str) {
    if let Err(err) = MemoryOps::init_memory() {
        fatal(phase, format!("memory init failed: {err}"));
    }
}

///
/// init_root_canister
/// Bootstraps the root canister runtime and environment.
///

pub fn init_root_canister(identity: SubnetIdentity) {
    // --- Phase 1: Init base systems ---
    init_eager_tls();
    init_memory_or_trap("init_root_canister");
    crate::log::set_ready();

    // log header
    println!("");
    println!("");
    println!("");
    crate::log!(
        Topic::Init,
        Info,
        "🔧 --------------------- canic v{VERSION} -----------------------",
    );
    crate::log!(Topic::Init, Info, "🏁 init: root ({identity:?})");

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

    SubnetRegistryOps::register_root(self_pid);

    // --- Phase 3: Service startup ---
    Runtime::start_all_root();
}

///
/// post_upgrade_root_canister
///

pub fn post_upgrade_root_canister() {
    // --- Phase 1: Init base systems ---
    init_eager_tls();
    init_memory_or_trap("post_upgrade_root_canister");
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 post_upgrade_root_canister");

    // ---  Phase 2 intentionally omitted: post-upgrade does not re-import env or directories.

    // --- Phase 3: Service startup ---
    Runtime::start_all_root();
}

///
/// init_nonroot_canister
///

pub fn init_nonroot_canister(canister_role: CanisterRole, payload: CanisterInitPayload) {
    // --- Phase 1: Init base systems ---
    init_eager_tls();
    init_memory_or_trap("init_nonroot_canister");
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 init: {}", canister_role);

    // --- Phase 2: Payload registration ---
    if let Err(err) = EnvOps::init_from_view(payload.env, canister_role) {
        fatal("init_nonroot_canister", format!("env import failed: {err}"));
    }

    AppDirectoryOps::import(app_directory_from_view(payload.app_directory));
    SubnetDirectoryOps::import(subnet_directory_from_view(payload.subnet_directory));

    // --- Phase 3: Service startup ---
    Runtime::start_all();
}

///
/// post_upgrade_nonroot_canister
///

pub fn post_upgrade_nonroot_canister(canister_role: CanisterRole) {
    // --- Phase 1: Init base systems ---
    init_eager_tls();
    init_memory_or_trap("post_upgrade_nonroot_canister");
    crate::log::set_ready();
    crate::log!(
        Topic::Init,
        Info,
        "🏁 post_upgrade_nonroot_canister: {}",
        canister_role
    );

    // ---  Phase 2 intentionally omitted: post-upgrade does not re-import env or directories.

    // --- Phase 3: Service startup ---
    Runtime::start_all();
}
