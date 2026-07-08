//! Module: canic_cli::blob_storage::render
//!
//! Responsibility: render blob-storage CLI action results for operators.
//! Does not own: output transport, JSON schema evolution, or readiness policy.
//! Boundary: turns render-ready models into compact terminal text.

use crate::blob_storage::model::{BlobStorageActionResult, BlobStorageStatusResult};

pub(super) fn render_action_result(result: &BlobStorageActionResult) -> String {
    let mut lines = vec![
        format!(
            "Blob storage {} {}",
            result.action.name.label(),
            action_status_label(result.action.dry_run)
        ),
        format!("Deployment: {}", result.deployment),
        format!("Target: {}", result.target.input),
        format!("Method: {}", result.action.method),
        format!("Mode: {}", result.action.mode.label()),
        result.action.requested_cycles.as_ref().map_or_else(
            || "Requested cycles: -".to_string(),
            |cycles| format!("Requested cycles: {cycles}"),
        ),
    ];
    if let Some(report) = &result.funding_report {
        lines.push(format!("Attached cycles: {}", report.attached_cycles));
        lines.push(format!(
            "Project cycles: {} -> {}",
            report.project_cycles_before, report.project_cycles_after
        ));
        lines.push(format!("Reserve cycles: {}", report.reserve_cycles));
        lines.push(format!(
            "Cashier total after: {}",
            report.cashier_total_after
        ));
        if let Some(reason) = &report.skipped_reason {
            lines.push(format!("Skipped reason: {reason}"));
        }
    }
    append_list(&mut lines, "Warnings", &result.warnings);
    if let Some(status) = &result.post_status {
        lines.push(String::new());
        lines.push("Post status:".to_string());
        for line in render_status_result(status).lines() {
            lines.push(format!("  {line}"));
        }
    }
    lines.join("\n")
}

pub(super) fn render_dry_run_command(result: &BlobStorageActionResult) -> String {
    format!("Command: {}", result.action.command)
}

const fn action_status_label(dry_run: bool) -> &'static str {
    if dry_run { "dry run" } else { "completed" }
}

pub(super) fn render_status_result(result: &BlobStorageStatusResult) -> String {
    let mut lines = vec![
        format!("Blob storage status: {}", result.target.input),
        format!("Deployment: {}", result.deployment),
        format!("Target: {}", result.target.canister_id),
        format!("Configured: {}", yes_no(result.configured)),
        format!(
            "Cashier: {}",
            result.cashier.canister_id.as_deref().unwrap_or("-")
        ),
        format!(
            "Payment account: {}",
            result.cashier.payment_account.as_deref().unwrap_or("-")
        ),
        format!(
            "Cashier balance: {}",
            cycles_or_dash(result.cashier.balance_cycles.as_deref())
        ),
        format!(
            "Upload balance: min {}, target {}",
            cycles_or_dash(result.policy.min_upload_balance_cycles.as_deref()),
            cycles_or_dash(result.policy.target_upload_balance_cycles.as_deref())
        ),
        format!(
            "Project reserve: {}",
            cycles_or_dash(result.policy.project_cycles_reserve_cycles.as_deref())
        ),
        format!(
            "Project cycles available: {}",
            result.policy.project_cycles_available
        ),
        format!("Gateways: {} synced", result.gateways.principal_count),
        format!(
            "Last gateway sync: {}",
            result
                .gateways
                .last_sync_at_ns
                .as_deref()
                .unwrap_or("never")
        ),
        format!("Readiness: {}", result.readiness.state.label()),
    ];

    append_list(&mut lines, "Blockers", &result.readiness.blockers);
    append_list(&mut lines, "Warnings", &result.readiness.warnings);
    if !result.next.is_empty() {
        lines.push("Next:".to_string());
        for action in &result.next {
            if let Some(command) = &action.command {
                lines.push(format!("  {command}"));
            }
        }
    }
    lines.join("\n")
}

fn append_list(lines: &mut Vec<String>, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    lines.push(format!("{label}:"));
    for value in values {
        lines.push(format!("  - {value}"));
    }
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn cycles_or_dash(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}
