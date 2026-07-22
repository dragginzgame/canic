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
        workflow::runtime::auth::RuntimeAuthWorkflow::reconcile_root_issuer_renewal()?;
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
    IntentStoreOps::rebuild_expiry_index()
        .map_err(|err| err.with_diagnostic_context("rebuild intent expiry derived index"))?;
    ReceiptBackedIntentOps::reconcile_receipt_indexes()
        .map_err(|err| err.with_diagnostic_context("reconcile receipt-backed intent indexes"))?;
    let _receipt_capacity = ReceiptBackedIntentOps::receipt_capacity()
        .map_err(|err| err.with_diagnostic_context("project receipt capacity"))?;

    Ok(())
}

pub(super) fn rebuild_root_derived_storage_indexes() -> Result<(), InternalError> {
    IcpRefillStoreOps::rebuild_indexes()
        .map_err(|err| err.with_diagnostic_context("rebuild root ICP-refill derived indexes"))?;
    rebuild_derived_storage_indexes()
}

pub(super) fn require_no_resumable_refill_for_upgrade() -> Result<(), InternalError> {
    validate_refill_upgrade_admission(IcpRefillStoreOps::resumable_operation_count())
}

fn validate_refill_upgrade_admission(count: usize) -> Result<(), InternalError> {
    if count == 0 {
        return Ok(());
    }

    Err(InternalError::invariant(
        InternalErrorOrigin::Workflow,
        format!(
            "root upgrade requires all ICP refill operations to be terminal; resumable_count={count}"
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::validate_refill_upgrade_admission;
    use crate::{InternalErrorClass, InternalErrorOrigin};

    #[test]
    fn root_upgrade_accepts_terminal_refill_state() {
        validate_refill_upgrade_admission(0).expect("terminal refill state should permit upgrade");
    }

    #[test]
    fn root_upgrade_rejects_resumable_refill_state() {
        let error = validate_refill_upgrade_admission(1)
            .expect_err("resumable refill state must block upgrade");
        assert_eq!(error.class(), InternalErrorClass::Invariant);
        assert_eq!(error.origin(), InternalErrorOrigin::Workflow);
    }
}
