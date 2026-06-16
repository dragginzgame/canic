//! Module: project::snapshot
//! Responsibility: owner-local accepted project record representation.
//! Does not own: stable-memory schema or lifecycle hooks.
//! Boundary: records accepted project facts after admission validation.

///
/// AcceptedProjectRecord
///
/// Owner-local accepted project fact stored by the project module.
/// This type stays `pub(crate)` so callers must go through admission reports
/// and project queries instead of depending on storage internals.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AcceptedProjectRecord {
    project_id: String,
    subnet_label: String,
}

impl AcceptedProjectRecord {
    /// Build one accepted record from already-validated admission input.
    #[must_use]
    pub(crate) fn new(project_id: &str, subnet_label: &str) -> Self {
        Self {
            project_id: project_id.to_owned(),
            subnet_label: subnet_label.to_owned(),
        }
    }

    /// Return the project identifier covered by this accepted record.
    #[must_use]
    pub(crate) fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Return the accepted subnet label.
    #[must_use]
    pub(crate) fn subnet_label(&self) -> &str {
        &self.subnet_label
    }
}
