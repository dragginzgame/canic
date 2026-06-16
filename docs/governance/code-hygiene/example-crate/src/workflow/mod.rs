//! Module: workflow
//!
//! Responsibility: small workflow-step contract for style examples.
//! Does not own: endpoint authorization, project admission, or persistence.
//! Boundary: names the step selected by an owner module.

mod route;

pub use route::{WorkflowStep, WorkflowStepKind};
