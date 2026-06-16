//! Module: workflow::route
//!
//! Responsibility: workflow step labels and step-kind classification.
//! Does not own: project validation or execution side effects.
//! Boundary: validates step labels before workflow code receives them.

use crate::diagnostic::StyleDiagnostic;

///
/// WorkflowStepKind
///
/// Coarse workflow family selected by the owner module.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowStepKind {
    ProjectInstall,

    ProjectRead,
}

impl WorkflowStepKind {
    /// Return whether this workflow step can mutate accepted project state.
    #[must_use]
    pub const fn is_write(self) -> bool {
        matches!(self, Self::ProjectInstall)
    }
}

///
/// WorkflowStep
///
/// Validated workflow step selected by a project owner before execution.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowStep {
    kind: WorkflowStepKind,
    label: String,
}

impl WorkflowStep {
    /// Build one validated workflow step.
    pub fn new(kind: WorkflowStepKind, label: impl Into<String>) -> Result<Self, StyleDiagnostic> {
        let label = label.into();
        let label = label.trim();

        if label.is_empty() {
            return Err(StyleDiagnostic::empty_workflow_step());
        }

        Ok(Self {
            kind,
            label: label.to_owned(),
        })
    }

    /// Return the workflow family.
    #[must_use]
    pub const fn kind(&self) -> WorkflowStepKind {
        self.kind
    }

    /// Return the normalized workflow label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use crate::{
        diagnostic::StyleDiagnosticCode,
        workflow::{WorkflowStep, WorkflowStepKind},
    };

    #[test]
    fn workflow_labels_are_normalized() {
        let step = WorkflowStep::new(WorkflowStepKind::ProjectRead, " project-alpha ")
            .expect("trimmed workflow labels should be valid");

        assert_eq!(step.label(), "project-alpha");
        assert!(!step.kind().is_write());
    }

    #[test]
    fn empty_workflow_labels_return_typed_diagnostic() {
        let err = WorkflowStep::new(WorkflowStepKind::ProjectRead, " ")
            .expect_err("empty workflow labels should fail");

        assert_eq!(err.code(), StyleDiagnosticCode::EmptyWorkflowStep);
    }
}
