//! Module: project::admission
//!
//! Responsibility: project admission request and report contracts.
//! Does not own: record storage or workflow execution.
//! Boundary: turns caller input into owner-approved project facts.

use crate::{diagnostic::StyleDiagnostic, project::AcceptedProjectRecord, workflow::WorkflowStep};

///
/// ProjectAdmission
///
/// Validated request to admit one project into the example owner module.
/// Admission owns input normalization but does not persist project state.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectAdmission {
    project_id: String,
    subnet_label: String,
}

impl ProjectAdmission {
    /// Build one validated project admission request.
    pub fn new(
        project_id: impl Into<String>,
        subnet_label: impl Into<String>,
    ) -> Result<Self, StyleDiagnostic> {
        let project_id = project_id.into();
        let project_id = project_id.trim();
        let subnet_label = subnet_label.into();
        let subnet_label = subnet_label.trim();

        if project_id.is_empty() {
            return Err(StyleDiagnostic::empty_project_id());
        }

        if subnet_label.is_empty() {
            return Err(StyleDiagnostic::missing_subnet_label());
        }

        Ok(Self {
            project_id: project_id.to_owned(),
            subnet_label: subnet_label.to_owned(),
        })
    }

    /// Return the accepted project identifier.
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Return the accepted subnet label.
    #[must_use]
    pub fn subnet_label(&self) -> &str {
        &self.subnet_label
    }

    /// Convert this admission into an accepted record owned by the project module.
    #[must_use]
    pub(crate) fn accepted_record(&self) -> AcceptedProjectRecord {
        AcceptedProjectRecord::new(&self.project_id, &self.subnet_label)
    }
}

///
/// ProjectAdmissionReport
///
/// Result envelope returned after a project admission has been accepted.
/// The report carries the validated admission and selected workflow step
/// without exposing project storage internals.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectAdmissionReport {
    admission: ProjectAdmission,
    step: WorkflowStep,
}

impl ProjectAdmissionReport {
    /// Build one report from an accepted admission and workflow step.
    #[must_use]
    pub const fn new(admission: ProjectAdmission, step: WorkflowStep) -> Self {
        Self { admission, step }
    }

    /// Return the accepted admission.
    #[must_use]
    pub const fn admission(&self) -> &ProjectAdmission {
        &self.admission
    }

    /// Return the workflow step chosen for the admission.
    #[must_use]
    pub const fn step(&self) -> &WorkflowStep {
        &self.step
    }
}
