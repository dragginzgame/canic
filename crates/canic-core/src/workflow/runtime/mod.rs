pub mod auth;
pub mod cycles;
pub mod install;
pub mod intent;
pub mod log;
mod nonroot;
mod root;
pub mod timer;

use crate::{
    InternalError, InternalErrorOrigin,
    ops::runtime::{env::EnvOps, memory::MemoryRegistryOps},
    workflow::{self, prelude::*},
};

pub use nonroot::{init_nonroot_canister, post_upgrade_nonroot_canister_after_memory_init};
pub use root::{init_root_canister, post_upgrade_root_canister_after_memory_init};

///
/// RuntimeWorkflow
/// Coordinates periodic background services (timers) for Canic canisters.
///

pub struct RuntimeWorkflow;

impl RuntimeWorkflow {
    /// Start timers that should run on all non-root canisters.
    pub fn start_all() {
        workflow::runtime::log::LogRetentionWorkflow::start();
        workflow::runtime::cycles::CycleTrackerWorkflow::start();
    }

    /// Start timers that should run on role-attestation-refreshing non-root canisters.
    pub fn start_all_with_role_attestation_refresh() {
        Self::start_all();
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() -> Result<(), InternalError> {
        EnvOps::require_root().map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("root context required: {err}"),
            )
        })?;

        // start shared timers too, but root only records cycle balance samples
        workflow::runtime::log::LogRetentionWorkflow::start();
        workflow::runtime::cycles::CycleTrackerWorkflow::start_standard_only();

        // root-only services
        workflow::pool::scheduler::PoolSchedulerWorkflow::start();
        Ok(())
    }
}

pub(super) fn log_memory_summary() {
    crate::log!(Topic::Memory, Info, "💾 memory.registry: bootstrapped");
}

fn init_post_upgrade_memory_registry() -> Result<(), InternalError> {
    MemoryRegistryOps::bootstrap_registry().map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("memory init failed: {err}"),
        )
    })
}

pub fn init_memory_registry_post_upgrade() -> Result<(), InternalError> {
    init_post_upgrade_memory_registry()
}
