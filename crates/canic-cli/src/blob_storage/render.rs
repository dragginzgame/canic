//! Module: canic_cli::blob_storage::render
//!
//! Responsibility: render blob-storage CLI action results for operators.
//! Does not own: output transport, JSON schema evolution, or readiness policy.
//! Boundary: turns render-ready models into compact terminal text.

use crate::blob_storage::model::BlobStorageActionResult;

pub(super) fn render_action_result(result: &BlobStorageActionResult) -> String {
    [
        format!("Blob storage {} dry run", result.action.name),
        format!("Deployment: {}", result.deployment),
        format!("Target: {}", result.target.input),
        format!("Method: {}", result.action.method),
        format!("Mode: {}", result.action.mode),
        result.action.requested_cycles.as_ref().map_or_else(
            || "Requested cycles: -".to_string(),
            |cycles| format!("Requested cycles: {cycles}"),
        ),
    ]
    .join("\n")
}

pub(super) fn render_dry_run_command(result: &BlobStorageActionResult) -> String {
    format!("Command: {}", result.action.command)
}
