//! Module: lib
//!
//! Responsibility: documentation-only crate root for Canic style examples.
//! Does not own: runtime behavior, workspace crate API, or production contracts.
//! Boundary: exposes a small App and workflow surface used only by docs.

pub mod app;
pub mod diagnostic;
pub mod workflow;

pub use app::{AppAdmission, AppAdmissionReport};
pub use diagnostic::{StyleDiagnostic, StyleDiagnosticCode};
pub use workflow::{WorkflowStep, WorkflowStepKind};
