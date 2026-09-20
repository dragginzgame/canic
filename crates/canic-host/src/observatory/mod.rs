//! Passive Fleet observatory and bounded downstream presentation.
//!
//! Existing authenticated role endpoints own observations; this owner cannot mutate a Fleet.

pub mod model;
pub mod ops;
pub mod policy;
#[cfg(test)]
mod tests;
pub mod view;
pub mod workflow;

/// Failure to assemble or present one bounded observatory snapshot.
#[derive(Debug, thiserror::Error)]
pub enum ObservatoryError {
    #[error("observatory {0} exceeds its explicit bound")]
    Bound(&'static str),
    #[error("invalid observatory profile or selection")]
    Profile,
    #[error("Fleet authority changed during observation; collect a new snapshot")]
    AuthorityChanged,
    #[error("cost comparison cannot use this evidence: {0:?}")]
    Comparison(view::CostComparisonFailure),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
