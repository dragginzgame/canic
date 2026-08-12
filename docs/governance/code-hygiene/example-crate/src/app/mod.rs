//! Module: app
//!
//! Responsibility: App admission example and owner-local state.
//! Does not own: endpoint authorization, stable storage, or workflow execution.
//! Boundary: validates App requests before publishing accepted App facts.

mod admission;
mod snapshot;

#[cfg(test)]
mod tests;

use crate::{
    diagnostic::StyleDiagnostic,
    workflow::{WorkflowStep, WorkflowStepKind},
};
use std::collections::BTreeMap;

pub use admission::{AppAdmission, AppAdmissionReport};
pub(crate) use snapshot::AcceptedAppRecord;

const MAX_SUBNET_LABEL_BYTES: usize = 64;

///
/// AppExample
///
/// Owner-local App example used to demonstrate accepted state flow.
/// The App module owns normalized App facts; workflows consume reports
/// instead of reconstructing state from storage internals.
///

#[derive(Default)]
pub struct AppExample {
    records: BTreeMap<String, AcceptedAppRecord>,
}

impl AppExample {
    /// Admit one App record and return the workflow step selected for it.
    pub fn admit(
        &mut self,
        app_id: impl Into<String>,
        subnet_label: impl Into<String>,
    ) -> Result<AppAdmissionReport, StyleDiagnostic> {
        let admission = AppAdmission::new(app_id, subnet_label)?;
        let record = admission.accepted_record();
        let step = WorkflowStep::new(WorkflowStepKind::AppInstall, admission.app_id())?;

        self.records.insert(admission.app_id().to_owned(), record);

        Ok(AppAdmissionReport::new(admission, step))
    }

    /// Return the accepted subnet label for one App when it is known.
    #[must_use]
    pub fn record_subnet_label(&self, app_id: &str) -> Option<&str> {
        self.records
            .get(app_id)
            .map(AcceptedAppRecord::subnet_label)
    }

    /// Return the accepted App identifier stored for one App key.
    #[must_use]
    pub fn record_app_id(&self, app_id: &str) -> Option<&str> {
        self.records.get(app_id).map(AcceptedAppRecord::app_id)
    }

    /// Return a read-only workflow step without mutating accepted state.
    pub fn read_step(&self, app_id: &str) -> Result<WorkflowStep, StyleDiagnostic> {
        WorkflowStep::new(WorkflowStepKind::AppRead, app_id)
    }

    /// Return the example subnet-label bound used by admission callers.
    #[must_use]
    pub const fn max_subnet_label_bytes() -> usize {
        MAX_SUBNET_LABEL_BYTES
    }
}
