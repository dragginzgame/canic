//! Module: app::admission
//!
//! Responsibility: App admission request and report contracts.
//! Does not own: record storage or workflow execution.
//! Boundary: turns caller input into owner-approved App facts.

use crate::{app::AcceptedAppRecord, diagnostic::StyleDiagnostic, workflow::WorkflowStep};

///
/// AppAdmission
///
/// Validated request to admit one App into the example owner module.
/// Admission owns input normalization but does not persist App state.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppAdmission {
    app_id: String,
    subnet_label: String,
}

impl AppAdmission {
    /// Build one validated App admission request.
    pub fn new(
        app_id: impl Into<String>,
        subnet_label: impl Into<String>,
    ) -> Result<Self, StyleDiagnostic> {
        let app_id = app_id.into();
        let app_id = app_id.trim();
        let subnet_label = subnet_label.into();
        let subnet_label = subnet_label.trim();

        if app_id.is_empty() {
            return Err(StyleDiagnostic::empty_app_id());
        }

        if subnet_label.is_empty() {
            return Err(StyleDiagnostic::missing_subnet_label());
        }

        Ok(Self {
            app_id: app_id.to_owned(),
            subnet_label: subnet_label.to_owned(),
        })
    }

    /// Return the accepted App identifier.
    #[must_use]
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Return the accepted subnet label.
    #[must_use]
    pub fn subnet_label(&self) -> &str {
        &self.subnet_label
    }

    /// Convert this admission into an accepted record owned by the App module.
    #[must_use]
    pub(crate) fn accepted_record(&self) -> AcceptedAppRecord {
        AcceptedAppRecord::new(&self.app_id, &self.subnet_label)
    }
}

///
/// AppAdmissionReport
///
/// Result envelope returned after an App admission has been accepted.
/// The report carries the validated admission and selected workflow step
/// without exposing App storage internals.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppAdmissionReport {
    admission: AppAdmission,
    step: WorkflowStep,
}

impl AppAdmissionReport {
    /// Build one report from an accepted admission and workflow step.
    #[must_use]
    pub const fn new(admission: AppAdmission, step: WorkflowStep) -> Self {
        Self { admission, step }
    }

    /// Return the accepted admission.
    #[must_use]
    pub const fn admission(&self) -> &AppAdmission {
        &self.admission
    }

    /// Return the workflow step chosen for the admission.
    #[must_use]
    pub const fn step(&self) -> &WorkflowStep {
        &self.step
    }
}
