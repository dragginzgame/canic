use super::{
    DeploymentComparisonReportError, digest::deployment_comparison_report_digest,
    status::comparison_status,
};
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentComparisonReportV1, DeploymentComparisonTargetV1,
};

/// Validate archived comparison report consistency and digest stability.
pub fn validate_deployment_comparison_report(
    report: &DeploymentComparisonReportV1,
) -> Result<(), DeploymentComparisonReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(DeploymentComparisonReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_comparison_field("report_id", report.report_id.as_str())?;
    ensure_comparison_field("report_digest", report.report_digest.as_str())?;
    ensure_comparison_field("compared_at", report.compared_at.as_str())?;
    validate_comparison_target("left", &report.left)?;
    validate_comparison_target("right", &report.right)?;
    if report.status != comparison_status(&report.hard_failures, &report.warnings) {
        return Err(DeploymentComparisonReportError::StatusMismatch);
    }
    if report.report_digest != deployment_comparison_report_digest(report) {
        return Err(DeploymentComparisonReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

fn validate_comparison_target(
    prefix: &'static str,
    target: &DeploymentComparisonTargetV1,
) -> Result<(), DeploymentComparisonReportError> {
    ensure_comparison_field(field_name(prefix, "label"), target.label.as_str())?;
    ensure_comparison_field(field_name(prefix, "check_id"), target.check_id.as_str())?;
    ensure_comparison_field(
        field_name(prefix, "check_digest"),
        target.check_digest.as_str(),
    )?;
    ensure_comparison_field(field_name(prefix, "plan_id"), target.plan_id.as_str())?;
    ensure_comparison_field(
        field_name(prefix, "plan_digest"),
        target.plan_digest.as_str(),
    )?;
    ensure_comparison_field(
        field_name(prefix, "inventory_id"),
        target.inventory_id.as_str(),
    )?;
    ensure_comparison_field(
        field_name(prefix, "inventory_digest"),
        target.inventory_digest.as_str(),
    )?;
    ensure_comparison_field(
        field_name(prefix, "deployment_name"),
        target.deployment_identity.deployment_name.as_str(),
    )?;
    ensure_comparison_field(
        field_name(prefix, "network"),
        target.deployment_identity.network.as_str(),
    )?;
    Ok(())
}

fn field_name(prefix: &'static str, field: &'static str) -> &'static str {
    match (prefix, field) {
        ("left", "label") => "left.label",
        ("left", "check_id") => "left.check_id",
        ("left", "check_digest") => "left.check_digest",
        ("left", "plan_id") => "left.plan_id",
        ("left", "plan_digest") => "left.plan_digest",
        ("left", "inventory_id") => "left.inventory_id",
        ("left", "inventory_digest") => "left.inventory_digest",
        ("left", "deployment_name") => "left.deployment_identity.deployment_name",
        ("left", "network") => "left.deployment_identity.network",
        ("right", "label") => "right.label",
        ("right", "check_id") => "right.check_id",
        ("right", "check_digest") => "right.check_digest",
        ("right", "plan_id") => "right.plan_id",
        ("right", "plan_digest") => "right.plan_digest",
        ("right", "inventory_id") => "right.inventory_id",
        ("right", "inventory_digest") => "right.inventory_digest",
        ("right", "deployment_name") => "right.deployment_identity.deployment_name",
        ("right", "network") => "right.deployment_identity.network",
        _ => field,
    }
}

fn ensure_comparison_field(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentComparisonReportError> {
    if value.trim().is_empty() {
        return Err(DeploymentComparisonReportError::MissingRequiredField { field });
    }
    Ok(())
}
