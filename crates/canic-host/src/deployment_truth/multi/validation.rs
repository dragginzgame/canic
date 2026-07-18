use super::{
    DeploymentComparisonReportError, digest::deployment_comparison_report_digest,
    status::comparison_status,
};
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentComparisonReportV1, DeploymentComparisonTargetV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeploymentComparisonFieldLabel(&'static str);

impl DeploymentComparisonFieldLabel {
    const COMPARED_AT: Self = Self("compared_at");
    const LEFT_CHECK_DIGEST: Self = Self("left.check_digest");
    const LEFT_CHECK_ID: Self = Self("left.check_id");
    const LEFT_DEPLOYMENT_NAME: Self = Self("left.deployment_identity.deployment_name");
    const LEFT_INVENTORY_DIGEST: Self = Self("left.inventory_digest");
    const LEFT_INVENTORY_ID: Self = Self("left.inventory_id");
    const LEFT_LABEL: Self = Self("left.label");
    const LEFT_NETWORK: Self = Self("left.deployment_identity.environment");
    const LEFT_PLAN_DIGEST: Self = Self("left.plan_digest");
    const LEFT_PLAN_ID: Self = Self("left.plan_id");
    const REPORT_DIGEST: Self = Self("report_digest");
    const REPORT_ID: Self = Self("report_id");
    const RIGHT_CHECK_DIGEST: Self = Self("right.check_digest");
    const RIGHT_CHECK_ID: Self = Self("right.check_id");
    const RIGHT_DEPLOYMENT_NAME: Self = Self("right.deployment_identity.deployment_name");
    const RIGHT_INVENTORY_DIGEST: Self = Self("right.inventory_digest");
    const RIGHT_INVENTORY_ID: Self = Self("right.inventory_id");
    const RIGHT_LABEL: Self = Self("right.label");
    const RIGHT_NETWORK: Self = Self("right.deployment_identity.environment");
    const RIGHT_PLAN_DIGEST: Self = Self("right.plan_digest");
    const RIGHT_PLAN_ID: Self = Self("right.plan_id");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeploymentComparisonTargetSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeploymentComparisonTargetField {
    CheckDigest,
    CheckId,
    DeploymentName,
    InventoryDigest,
    InventoryId,
    Label,
    Environment,
    PlanDigest,
    PlanId,
}

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
    ensure_comparison_field(
        DeploymentComparisonFieldLabel::REPORT_ID,
        report.report_id.as_str(),
    )?;
    ensure_comparison_field(
        DeploymentComparisonFieldLabel::REPORT_DIGEST,
        report.report_digest.as_str(),
    )?;
    ensure_comparison_field(
        DeploymentComparisonFieldLabel::COMPARED_AT,
        report.compared_at.as_str(),
    )?;
    validate_comparison_target(DeploymentComparisonTargetSide::Left, &report.left)?;
    validate_comparison_target(DeploymentComparisonTargetSide::Right, &report.right)?;
    if report.status != comparison_status(&report.hard_failures, &report.warnings) {
        return Err(DeploymentComparisonReportError::StatusMismatch);
    }
    if report.report_digest != deployment_comparison_report_digest(report) {
        return Err(DeploymentComparisonReportError::DigestMismatch {
            field: DeploymentComparisonFieldLabel::REPORT_DIGEST.as_str(),
        });
    }
    Ok(())
}

fn validate_comparison_target(
    side: DeploymentComparisonTargetSide,
    target: &DeploymentComparisonTargetV1,
) -> Result<(), DeploymentComparisonReportError> {
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::Label),
        target.label.as_str(),
    )?;
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::CheckId),
        target.check_id.as_str(),
    )?;
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::CheckDigest),
        target.check_digest.as_str(),
    )?;
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::PlanId),
        target.plan_id.as_str(),
    )?;
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::PlanDigest),
        target.plan_digest.as_str(),
    )?;
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::InventoryId),
        target.inventory_id.as_str(),
    )?;
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::InventoryDigest),
        target.inventory_digest.as_str(),
    )?;
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::DeploymentName),
        target.deployment_identity.deployment_name.as_str(),
    )?;
    ensure_comparison_field(
        comparison_target_field_label(side, DeploymentComparisonTargetField::Environment),
        target.deployment_identity.environment.as_str(),
    )?;
    Ok(())
}

const fn comparison_target_field_label(
    side: DeploymentComparisonTargetSide,
    field: DeploymentComparisonTargetField,
) -> DeploymentComparisonFieldLabel {
    match (side, field) {
        (DeploymentComparisonTargetSide::Left, DeploymentComparisonTargetField::CheckDigest) => {
            DeploymentComparisonFieldLabel::LEFT_CHECK_DIGEST
        }
        (DeploymentComparisonTargetSide::Left, DeploymentComparisonTargetField::CheckId) => {
            DeploymentComparisonFieldLabel::LEFT_CHECK_ID
        }
        (DeploymentComparisonTargetSide::Left, DeploymentComparisonTargetField::DeploymentName) => {
            DeploymentComparisonFieldLabel::LEFT_DEPLOYMENT_NAME
        }
        (
            DeploymentComparisonTargetSide::Left,
            DeploymentComparisonTargetField::InventoryDigest,
        ) => DeploymentComparisonFieldLabel::LEFT_INVENTORY_DIGEST,
        (DeploymentComparisonTargetSide::Left, DeploymentComparisonTargetField::InventoryId) => {
            DeploymentComparisonFieldLabel::LEFT_INVENTORY_ID
        }
        (DeploymentComparisonTargetSide::Left, DeploymentComparisonTargetField::Label) => {
            DeploymentComparisonFieldLabel::LEFT_LABEL
        }
        (DeploymentComparisonTargetSide::Left, DeploymentComparisonTargetField::Environment) => {
            DeploymentComparisonFieldLabel::LEFT_NETWORK
        }
        (DeploymentComparisonTargetSide::Left, DeploymentComparisonTargetField::PlanDigest) => {
            DeploymentComparisonFieldLabel::LEFT_PLAN_DIGEST
        }
        (DeploymentComparisonTargetSide::Left, DeploymentComparisonTargetField::PlanId) => {
            DeploymentComparisonFieldLabel::LEFT_PLAN_ID
        }
        (DeploymentComparisonTargetSide::Right, DeploymentComparisonTargetField::CheckDigest) => {
            DeploymentComparisonFieldLabel::RIGHT_CHECK_DIGEST
        }
        (DeploymentComparisonTargetSide::Right, DeploymentComparisonTargetField::CheckId) => {
            DeploymentComparisonFieldLabel::RIGHT_CHECK_ID
        }
        (
            DeploymentComparisonTargetSide::Right,
            DeploymentComparisonTargetField::DeploymentName,
        ) => DeploymentComparisonFieldLabel::RIGHT_DEPLOYMENT_NAME,
        (
            DeploymentComparisonTargetSide::Right,
            DeploymentComparisonTargetField::InventoryDigest,
        ) => DeploymentComparisonFieldLabel::RIGHT_INVENTORY_DIGEST,
        (DeploymentComparisonTargetSide::Right, DeploymentComparisonTargetField::InventoryId) => {
            DeploymentComparisonFieldLabel::RIGHT_INVENTORY_ID
        }
        (DeploymentComparisonTargetSide::Right, DeploymentComparisonTargetField::Label) => {
            DeploymentComparisonFieldLabel::RIGHT_LABEL
        }
        (DeploymentComparisonTargetSide::Right, DeploymentComparisonTargetField::Environment) => {
            DeploymentComparisonFieldLabel::RIGHT_NETWORK
        }
        (DeploymentComparisonTargetSide::Right, DeploymentComparisonTargetField::PlanDigest) => {
            DeploymentComparisonFieldLabel::RIGHT_PLAN_DIGEST
        }
        (DeploymentComparisonTargetSide::Right, DeploymentComparisonTargetField::PlanId) => {
            DeploymentComparisonFieldLabel::RIGHT_PLAN_ID
        }
    }
}

fn ensure_comparison_field(
    field: DeploymentComparisonFieldLabel,
    value: &str,
) -> Result<(), DeploymentComparisonReportError> {
    if value.trim().is_empty() {
        return Err(DeploymentComparisonReportError::MissingRequiredField {
            field: field.as_str(),
        });
    }
    Ok(())
}
