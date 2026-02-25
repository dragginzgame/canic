pub mod cycles;
pub mod intent;
pub mod log;
pub mod random;
pub mod timer;
pub mod wasm;

use crate::{
    InternalError, InternalErrorOrigin, VERSION,
    domain::policy::env::{EnvInput, EnvPolicyError, validate_or_default},
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::SubnetRole,
    ops::{
        config::ConfigOps,
        ic::{IcOps, network::NetworkOps},
        runtime::{
            env::EnvOps,
            memory::{MemoryRegistryInitSummary, MemoryRegistryOps},
        },
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
            state::app::AppStateOps,
        },
    },
    workflow::{self, env::EnvWorkflow, prelude::*},
};

///
/// RuntimeWorkflow
/// Coordinates periodic background services (timers) for Canic canisters.
///

pub struct RuntimeWorkflow;

impl RuntimeWorkflow {
    /// Start timers that should run on all canisters.
    pub fn start_all() {
        workflow::runtime::cycles::CycleTrackerWorkflow::start();
        workflow::runtime::intent::IntentCleanupWorkflow::start();
        workflow::runtime::log::LogRetentionWorkflow::start();
        workflow::runtime::random::RandomWorkflow::start();
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() -> Result<(), InternalError> {
        EnvOps::require_root().map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("root context required: {err}"),
            )
        })?;

        // start shared timers too
        Self::start_all();

        // root-only services
        workflow::pool::scheduler::PoolSchedulerWorkflow::start();
        Ok(())
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

fn init_post_upgrade_memory_registry() -> Result<MemoryRegistryInitSummary, InternalError> {
    MemoryRegistryOps::init_eager_tls();
    MemoryRegistryOps::init_registry().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("memory init failed: {err}"),
        )
    })
}

///
/// init_root_canister
/// Bootstraps the root canister runtime and environment.
///

pub fn init_root_canister(identity: SubnetIdentity) -> Result<(), InternalError> {
    // --- Phase 1: Init base systems ---
    MemoryRegistryOps::init_eager_tls();
    let memory_summary = match MemoryRegistryOps::init_registry() {
        Ok(summary) => summary,
        Err(err) => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("memory init failed: {err}"),
            ));
        }
    };
    crate::log::set_ready();

    // log header
    IcOps::println("");
    IcOps::println("");
    IcOps::println("");
    crate::log!(
        Topic::Init,
        Info,
        "🔧 --------------------- canic v{VERSION} -----------------------",
    );
    crate::log!(Topic::Init, Info, "🏁 init: root ({identity:?})");
    log_memory_summary(&memory_summary);

    // --- Phase 2: Env registration ---
    let self_pid = IcOps::canister_self();
    let (subnet_pid, subnet_role, prime_root_pid) = match identity {
        SubnetIdentity::Prime => (self_pid, SubnetRole::PRIME, self_pid),
        SubnetIdentity::Standard(params) => (self_pid, params.subnet_type, params.prime_root_pid),
        SubnetIdentity::Manual => (IcOps::canister_self(), SubnetRole::PRIME, self_pid),
    };

    let input = EnvInput {
        prime_root_pid: Some(prime_root_pid),
        subnet_role: Some(subnet_role),
        subnet_pid: Some(subnet_pid),
        root_pid: Some(self_pid),
        canister_role: Some(CanisterRole::ROOT),
        parent_pid: Some(prime_root_pid),
    };

    let network = NetworkOps::build_network().ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "runtime network unavailable; set DFX_NETWORK=local|ic at build time".to_string(),
        )
    })?;
    crate::log!(Topic::Init, Info, "build network: {network}");
    let validated = match validate_or_default(network, input) {
        Ok(validated) => validated,
        Err(EnvPolicyError::MissingEnvFields(missing)) => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("env args missing {missing}; local builds require explicit env fields"),
            ));
        }
    };

    if let Err(err) = EnvOps::import_validated(validated) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("env import failed: {err}"),
        ));
    }

    let app_mode = ConfigOps::app_init_mode().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("app mode init failed: {err}"),
        )
    })?;
    AppStateOps::init_mode(app_mode);

    let created_at = IcOps::now_secs();
    SubnetRegistryOps::register_root(self_pid, created_at);

    // --- Phase 3: Service startup ---
    RuntimeWorkflow::start_all_root().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("root service startup failed: {err}"),
        )
    })?;

    Ok(())
}

///
/// post_upgrade_root_canister
///

pub fn post_upgrade_root_canister_after_memory_init(
    memory_summary: MemoryRegistryInitSummary,
) -> Result<(), InternalError> {
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 post_upgrade_root_canister");
    log_memory_summary(&memory_summary);

    // ---  Phase 2 intentionally omitted: post-upgrade does not re-import env or directories.

    // --- Phase 3: Service startup ---
    RuntimeWorkflow::start_all_root().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("root service startup failed: {err}"),
        )
    })?;

    Ok(())
}

///
/// init_nonroot_canister
///

pub fn init_nonroot_canister(
    canister_role: CanisterRole,
    payload: CanisterInitPayload,
) -> Result<(), InternalError> {
    // --- Phase 1: Init base systems ---
    MemoryRegistryOps::init_eager_tls();
    let memory_summary = MemoryRegistryOps::init_registry().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("memory init failed: {err}"),
        )
    })?;
    crate::log::set_ready();
    crate::log!(Topic::Init, Info, "🏁 init: {}", canister_role);
    log_memory_summary(&memory_summary);

    // --- Phase 2: Payload registration ---
    EnvWorkflow::init_env_from_args(payload.env, canister_role).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("env import failed: {err}"),
        )
    })?;

    AppDirectoryOps::import_args_allow_incomplete(payload.app_directory).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("app directory import failed: {err}"),
        )
    })?;
    SubnetDirectoryOps::import_args_allow_incomplete(payload.subnet_directory).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("subnet directory import failed: {err}"),
        )
    })?;

    let app_mode = ConfigOps::app_init_mode().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("app mode init failed: {err}"),
        )
    })?;
    AppStateOps::init_mode(app_mode);

    // --- Phase 3: Service startup ---
    RuntimeWorkflow::start_all();

    Ok(())
}

///
/// post_upgrade_nonroot_canister
///

pub fn post_upgrade_nonroot_canister_after_memory_init(
    canister_role: CanisterRole,
    memory_summary: MemoryRegistryInitSummary,
) {
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
    RuntimeWorkflow::start_all();
}

pub fn init_memory_registry_post_upgrade() -> Result<MemoryRegistryInitSummary, InternalError> {
    init_post_upgrade_memory_registry()
}
