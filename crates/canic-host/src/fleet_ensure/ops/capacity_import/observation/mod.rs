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

/// One prepared paid observation. Preparation has already completed every free precheck.
pub trait PreparedCapacityImportObservation {
    /// Authenticated sample produced by the paid call and its postchecks.
    type View;

    /// Issue at most one management status request after workflow persists its allowance.
    fn observe(self) -> impl Future<Output = Result<Self::View, CapacityImportJournalError>>;
}

/// Fresh observations from the selected network and signer, including verified ownership.
///
/// Each source observation may issue at most one management status request. Destination
/// observation may issue at most one Root management status request; membership and
/// disposition inspection must be read-only. Preparation spends no paid allowance;
/// workflow retains each allowance immediately before executing the prepared observation.
pub trait CapacityImportObserver {
    /// Prepared destination inspection, independent of the borrowed journal and observer.
    type Destination: PreparedCapacityImportObservation<View = CapacityImportDestinationView>;
    /// Prepared source inspection, independent of the borrowed journal and observer.
    type Source: PreparedCapacityImportObservation<View = CapacityImportSourceView>;

    /// Verify current infrastructure modules, exact Fleet membership and Root readiness.
    fn prepare_destination(
        &mut self,
        plan: &CapacityImportPlanRecord,
    ) -> impl Future<Output = Result<Self::Destination, CapacityImportJournalError>>;

    /// Bind certified placement and management fields to verified absence/retirement evidence.
    fn prepare_source(
        &mut self,
        plan: &CapacityImportPlanRecord,
        canister: Principal,
    ) -> impl Future<Output = Result<Self::Source, CapacityImportJournalError>>;
}
