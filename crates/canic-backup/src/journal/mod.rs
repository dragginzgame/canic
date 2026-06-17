//! Module: journal
//!
//! Responsibility: track resumable backup artifact download state.
//! Does not own: execution planning, snapshot capture, or manifest validation.
//! Boundary: persists artifact progress for backup resume and integrity checks.

mod report;
#[cfg(test)]
mod tests;
mod types;
mod validation;

pub use report::{ArtifactResumeReport, JournalResumeReport, JournalStateCounts};
pub use types::{
    ArtifactJournalEntry, ArtifactState, DownloadJournal, DownloadOperationMetrics, ResumeAction,
};
pub use validation::JournalValidationError;
