//! Module: diagnostic
//! Responsibility: compact diagnostic vocabulary for the example crate.
//! Does not own: production error codes or runtime error mapping.
//! Boundary: carries typed failures without relying on string matching.

use std::fmt::{self, Display};

///
/// StyleDiagnosticCode
///
/// Stable diagnostic categories used by the documentation-only example crate.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StyleDiagnosticCode {
    EmptyProjectId,

    EmptyWorkflowStep,

    MissingSubnetLabel,
}

impl Display for StyleDiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::EmptyProjectId => "empty project id",
            Self::EmptyWorkflowStep => "empty workflow step",
            Self::MissingSubnetLabel => "missing subnet label",
        };

        f.write_str(label)
    }
}

///
/// StyleDiagnostic
///
/// Typed diagnostic value used by constructors that enforce example invariants.
/// The code is the stable contract; the message is only human-facing context.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleDiagnostic {
    code: StyleDiagnosticCode,
    message: &'static str,
}

impl StyleDiagnostic {
    /// Build one diagnostic from a stable code and static message.
    #[must_use]
    pub const fn new(code: StyleDiagnosticCode, message: &'static str) -> Self {
        Self { code, message }
    }

    /// Return the diagnostic code for an empty project identifier.
    #[must_use]
    pub const fn empty_project_id() -> Self {
        Self::new(
            StyleDiagnosticCode::EmptyProjectId,
            "project id must not be empty",
        )
    }

    /// Return the diagnostic code for an empty workflow step label.
    #[must_use]
    pub const fn empty_workflow_step() -> Self {
        Self::new(
            StyleDiagnosticCode::EmptyWorkflowStep,
            "workflow step label must not be empty",
        )
    }

    /// Return the diagnostic code for an absent subnet label.
    #[must_use]
    pub const fn missing_subnet_label() -> Self {
        Self::new(
            StyleDiagnosticCode::MissingSubnetLabel,
            "subnet label must not be empty",
        )
    }

    /// Return the stable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> StyleDiagnosticCode {
        self.code
    }

    /// Return the human-facing diagnostic message.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        self.message
    }
}

impl Display for StyleDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for StyleDiagnostic {}
