pub mod cycles;
pub mod log;
pub mod random;

use crate::{
    VERSION, access,
    cdk::{api::trap, println},
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::SubnetRole,
    ops::{
        runtime::{
            env::{EnvOps, EnvSnapshot},
            memory::{MemoryOps, MemoryRegistryInitSummary},
        },
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{
        self,
        prelude::*,
        topology::directory::mapper::{AppDirectoryMapper, SubnetDirectoryMapper},
    },
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
        access::env::require_root().unwrap_or_else(|e| fatal("start_all_root", e));

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

fn init_memory_or_trap(phase: &str) -> MemoryRegistryInitSummary {
    match MemoryOps::init_registry() {
        Ok(summary) => summary,
        Err(err) => fatal(phase, format!("memory init failed: {err}")),
    }
}

fn log_memory_summary(summary: &MemoryRegistryInitSummary) {
    for range in &summary.ranges {
        let used = summary
            .entries
            .iter()
            .filter(|entry| entry.id >= range.start && entry.id <= range.end)
            .count();

        crate::log!(
            Topic::Memory,
            Info,
            "💾 memory.range: {} [{}-{}] ({}/{} slots used)",
            range.crate_name,
            range.start,
            range.end,
            used,
            range.end - range.start + 1,
        );
    }
}

///
/// init_root_canister
/// Bootstraps the root canister runtime and environment.
///

pub fn init_root_canister(identity: SubnetIdentity) {
    // --- Phase 1: Init base systems ---
    init_eager_tls();
    let memory_summary = init_memory_or_trap("init_root_canister");
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
    log_memory_summary(&memory_summary);

    // --- Phase 2: Env registration ---
    let self_pid = canister_self();
    let (subnet_pid, subnet_role, prime_root_pid) = match identity {
        SubnetIdentity::Prime => (self_pid, SubnetRole::PRIME, self_pid),
        SubnetIdentity::Standard(params) => (self_pid, params.subnet_type, params.prime_root_pid),
        SubnetIdentity::Manual => (canister_self(), SubnetRole::PRIME, self_pid),
    };

    let snapshot = EnvSnapshot {
        prime_root_pid: Some(prime_root_pid),
        root_pid: Some(self_pid),
        subnet_pid: Some(subnet_pid),
        subnet_role: Some(subnet_role),
        canister_role: Some(CanisterRole::ROOT),
        parent_pid: Some(prime_root_pid),
    };

    if let Err(err) = EnvOps::import(snapshot) {
        fatal("init_root_canister", format!("env import failed: {err}"));
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
    let memory_summary = init_memory_or_trap("post_upgrade_root_canister");
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 post_upgrade_root_canister");
    log_memory_summary(&memory_summary);

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
    let memory_summary = init_memory_or_trap("init_nonroot_canister");
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 init: {}", canister_role);
    log_memory_summary(&memory_summary);

    // --- Phase 2: Payload registration ---
    if let Err(err) = crate::workflow::env::init_env_from_view(payload.env, canister_role) {
        fatal("init_nonroot_canister", format!("env import failed: {err}"));
    }

    AppDirectoryOps::import(AppDirectoryMapper::view_to_snapshot(payload.app_directory));
    SubnetDirectoryOps::import(SubnetDirectoryMapper::view_to_snapshot(
        payload.subnet_directory,
    ));

    // --- Phase 3: Service startup ---
    Runtime::start_all();
}

///
/// post_upgrade_nonroot_canister
///

pub fn post_upgrade_nonroot_canister(canister_role: CanisterRole) {
    // --- Phase 1: Init base systems ---
    init_eager_tls();
    let memory_summary = init_memory_or_trap("post_upgrade_nonroot_canister");
    crate::log::set_ready();
    crate::log!(
        Topic::Init,
        Info,
        "🏁 post_upgrade_nonroot_canister: {}",
        canister_role
    );
    log_memory_summary(&memory_summary);

    // ---  Phase 2 intentionally omitted: post-upgrade does not re-import env or directories.

    // --- Phase 3: Service startup ---
    Runtime::start_all();
}
