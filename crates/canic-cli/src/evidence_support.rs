//! Module: canic_cli::evidence_support
//!
//! Responsibility: share command-provenance helpers for evidence envelopes.
//! Does not own: evidence schemas, policy evaluation, or report rendering.
//! Boundary: normalizes optional path arguments for stable command provenance.

use std::time::{SystemTime, UNIX_EPOCH};

/// Return the current evidence timestamp in the canonical Unix-seconds form.
pub fn current_evidence_timestamp() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "unix:{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    ))
}
