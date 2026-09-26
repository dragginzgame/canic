//! Authenticated observation boundary used before host controller handoffs.
//!
//! Implementations own live inventory and disposition verification, never record mutation.

use crate::fleet_ensure::{
    model::capacity_import::CapacityImportPlanRecord,
    ops::capacity_import::journal::CapacityImportJournalError,
    view::capacity_import::{CapacityImportDestinationView, CapacityImportSourceView},
};
use candid::Principal;
use std::future::Future;

/// Fresh observations from the selected network and signer, including verified ownership.
///
/// Each source observation may issue at most one management status request. Destination
/// observation may issue at most one Root management status request; membership and
/// disposition inspection must be read-only. Workflow retains each allowance first.
pub trait CapacityImportObserver {
    /// Verify current infrastructure modules, exact Fleet membership and Root readiness.
    fn destination(
        &mut self,
        plan: &CapacityImportPlanRecord,
    ) -> impl Future<Output = Result<CapacityImportDestinationView, CapacityImportJournalError>>;

    /// Bind certified placement and management fields to verified absence/retirement evidence.
    fn source(
        &mut self,
        plan: &CapacityImportPlanRecord,
        canister: Principal,
    ) -> impl Future<Output = Result<CapacityImportSourceView, CapacityImportJournalError>>;
}
