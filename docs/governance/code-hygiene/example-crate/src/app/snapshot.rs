//! Module: app::snapshot
//!
//! Responsibility: owner-local accepted App record representation.
//! Does not own: stable-memory schema or lifecycle hooks.
//! Boundary: records accepted App facts after admission validation.

///
/// AcceptedAppRecord
///
/// Owner-local accepted App fact stored by the App module.
/// This type stays `pub(crate)` so callers must go through admission reports
/// and App queries instead of depending on storage internals.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AcceptedAppRecord {
    app_id: String,
    subnet_label: String,
}

impl AcceptedAppRecord {
    /// Build one accepted record from already-validated admission input.
    #[must_use]
    pub(crate) fn new(app_id: &str, subnet_label: &str) -> Self {
        Self {
            app_id: app_id.to_owned(),
            subnet_label: subnet_label.to_owned(),
        }
    }

    /// Return the App identifier covered by this accepted record.
    #[must_use]
    pub(crate) fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Return the accepted subnet label.
    #[must_use]
    pub(crate) fn subnet_label(&self) -> &str {
        &self.subnet_label
    }
}
