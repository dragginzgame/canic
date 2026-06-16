//! Module: lib
//! Responsibility: documentation-only crate root for Canic style examples.
//! Does not own: runtime behavior, workspace crate API, or production contracts.
//! Boundary: exposes a small project and workflow surface used only by docs.

pub mod diagnostic;
pub mod project;
pub mod workflow;

pub use diagnostic::{StyleDiagnostic, StyleDiagnosticCode};
pub use project::{ProjectAdmission, ProjectAdmissionReport};
pub use workflow::{WorkflowStep, WorkflowStepKind};
