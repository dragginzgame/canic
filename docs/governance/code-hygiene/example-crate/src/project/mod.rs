//! Module: project
//!
//! Responsibility: project admission example and owner-local state.
//! Does not own: endpoint authorization, stable storage, or workflow execution.
//! Boundary: validates project requests before publishing accepted project facts.

mod admission;
mod snapshot;

#[cfg(test)]
mod tests;

use crate::{
    diagnostic::StyleDiagnostic,
    workflow::{WorkflowStep, WorkflowStepKind},
};
use std::collections::BTreeMap;

pub use admission::{ProjectAdmission, ProjectAdmissionReport};
pub(crate) use snapshot::AcceptedProjectRecord;

const MAX_SUBNET_LABEL_BYTES: usize = 64;

///
/// ProjectExample
///
/// Owner-local project example used to demonstrate accepted state flow.
/// The project module owns normalized project facts; workflows consume reports
/// instead of reconstructing state from storage internals.
///

#[derive(Default)]
pub struct ProjectExample {
    records: BTreeMap<String, AcceptedProjectRecord>,
}

impl ProjectExample {
    /// Admit one project record and return the workflow step selected for it.
    pub fn admit(
        &mut self,
        project_id: impl Into<String>,
        subnet_label: impl Into<String>,
    ) -> Result<ProjectAdmissionReport, StyleDiagnostic> {
        let admission = ProjectAdmission::new(project_id, subnet_label)?;
        let record = admission.accepted_record();
        let step = WorkflowStep::new(WorkflowStepKind::ProjectInstall, admission.project_id())?;

        self.records
            .insert(admission.project_id().to_owned(), record);

        Ok(ProjectAdmissionReport::new(admission, step))
    }

    /// Return the accepted subnet label for one project when it is known.
    #[must_use]
    pub fn record_subnet_label(&self, project_id: &str) -> Option<&str> {
        self.records
            .get(project_id)
            .map(AcceptedProjectRecord::subnet_label)
    }

    /// Return the accepted project identifier stored for one project key.
    #[must_use]
    pub fn record_project_id(&self, project_id: &str) -> Option<&str> {
        self.records
            .get(project_id)
            .map(AcceptedProjectRecord::project_id)
    }

    /// Return a read-only workflow step without mutating accepted state.
    pub fn read_step(&self, project_id: &str) -> Result<WorkflowStep, StyleDiagnostic> {
        WorkflowStep::new(WorkflowStepKind::ProjectRead, project_id)
    }

    /// Return the example subnet-label bound used by admission callers.
    #[must_use]
    pub const fn max_subnet_label_bytes() -> usize {
        MAX_SUBNET_LABEL_BYTES
    }
}
