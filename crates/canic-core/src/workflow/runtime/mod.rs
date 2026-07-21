//! Module: workflow::runtime
//!
//! Responsibility: coordinate runtime startup services and memory initialization.
//! Does not own: lifecycle adapters, endpoint authorization, or stable schemas.
//! Boundary: lifecycle workflows call runtime startup after environment restore.

pub mod auth;
pub mod cycles;
pub mod install;
pub mod intent;
pub mod log;
mod nonroot;
mod root;
pub mod timer;

use crate::ops::storage::{
    icp_refill::IcpRefillStoreOps,
    intent::{IntentStoreOps, ReceiptBackedIntentOps},
};
use crate::{
    InternalError, InternalErrorOrigin,
    log::Topic,
    ops::runtime::{env::EnvOps, memory::MemoryRegistryOps},
    workflow,
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
    pub fn start_all() -> Result<(), InternalError> {
        workflow::runtime::log::LogRetentionWorkflow::start()?;
        workflow::runtime::cycles::CycleWorkflow::start()?;
        workflow::runtime::intent::IntentCleanupWorkflow::start()?;
        Ok(())
    }

    /// Start timers that should run on role-attestation-refreshing non-root canisters.
    pub fn start_all_with_role_attestation_refresh() -> Result<(), InternalError> {
        Self::start_all()
    }

    /// Start timers that should run only on root canisters.
    pub fn start_all_root() -> Result<(), InternalError> {
        EnvOps::require_root().map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("root context required: {err}"),
            )
        })?;

        // Start shared runtime owners before root-only services.
        workflow::runtime::log::LogRetentionWorkflow::start()?;
        workflow::runtime::cycles::CycleWorkflow::start()?;
        workflow::runtime::intent::IntentCleanupWorkflow::start()?;

        // root-only services
        workflow::pool::scheduler::PoolSchedulerWorkflow::start();
        workflow::runtime::auth::RuntimeAuthWorkflow::start_root_delegation_renewal_timer_if_configured()?;
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

pub(super) fn rebuild_derived_storage_indexes() -> Result<(), InternalError> {
    IcpRefillStoreOps::rebuild_indexes()
        .map_err(|err| err.with_diagnostic_context("rebuild ICP-refill derived indexes"))?;
    IntentStoreOps::rebuild_expiry_index()
        .map_err(|err| err.with_diagnostic_context("rebuild intent expiry derived index"))?;
    ReceiptBackedIntentOps::rebuild_placement_acknowledgement_index().map_err(|err| {
        err.with_diagnostic_context("rebuild placement acknowledgement derived index")
    })?;

    Ok(())
}
