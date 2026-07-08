use super::super::super::*;
use super::{
    super::{
        digest::deployment_root_verification_report_digest,
        error::DeploymentRootVerificationReportError,
    },
    checks::{RootVerificationCheckName, present_value, root_observation_source_label_from_source},
    shared::root_verification_transition,
};

/// Validate archived root-verification report consistency and digest stability.
///
/// A valid report is still passive evidence: only a future successful
/// receipt-backed state write can record verified root state.
pub fn validate_deployment_root_verification_report(
    report: &DeploymentRootVerificationReportV1,
) -> Result<(), DeploymentRootVerificationReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            DeploymentRootVerificationReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: report.schema_version,
            },
        );
    }
    ensure_root_verification_field("report_id", report.report_id.as_str())?;
    ensure_root_verification_sha256("report_digest", report.report_digest.as_str())?;
    ensure_root_verification_field("requested_at", report.requested_at.as_str())?;
    ensure_root_verification_field("deployment_name", report.deployment_name.as_str())?;
    ensure_root_verification_field("network", report.network.as_str())?;
    ensure_root_verification_field(
        "expected_fleet_template",
        report.expected_fleet_template.as_str(),
    )?;
    ensure_root_verification_field(
        "expected_root_principal",
        report.expected_root_principal.as_str(),
    )?;
    ensure_root_verification_field("source_check_id", report.source_check_id.as_str())?;
    ensure_root_verification_sha256("source_check_digest", report.source_check_digest.as_str())?;
    ensure_root_verification_field(
        "source_deployment_plan_id",
        report.source_deployment_plan_id.as_str(),
    )?;
    ensure_root_verification_sha256(
        "source_deployment_plan_digest",
        report.source_deployment_plan_digest.as_str(),
    )?;
    ensure_root_verification_field("source_inventory_id", report.source_inventory_id.as_str())?;
    ensure_root_verification_sha256(
        "source_inventory_digest",
        report.source_inventory_digest.as_str(),
    )?;
    if report.evidence_status != report_evidence_status(report)
        || report.state_transition != report_state_transition(report)
    {
        return Err(DeploymentRootVerificationReportError::StatusMismatch);
    }
    ensure_root_verification_report_checks_consistent(report)?;
    if report.report_digest != deployment_root_verification_report_digest(report) {
        return Err(DeploymentRootVerificationReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}
fn report_evidence_status(
    report: &DeploymentRootVerificationReportV1,
) -> DeploymentRootVerificationEvidenceStatusV1 {
    if report.blockers.is_empty()
        && report.identity_checks.iter().all(|check| check.satisfied)
        && report.evidence_checks.iter().all(|check| check.satisfied)
    {
        DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied
    } else {
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    }
}

const fn report_state_transition(
    report: &DeploymentRootVerificationReportV1,
) -> DeploymentRootVerificationStateTransitionV1 {
    root_verification_transition(report.evidence_status, report.current_root_verification)
}

fn ensure_root_verification_report_checks_consistent(
    report: &DeploymentRootVerificationReportV1,
) -> Result<(), DeploymentRootVerificationReportError> {
    ensure_report_check_names(
        &report.identity_checks,
        &[
            RootVerificationCheckName::DeploymentName,
            RootVerificationCheckName::Network,
            RootVerificationCheckName::FleetTemplate,
            RootVerificationCheckName::RootPrincipal,
            RootVerificationCheckName::PlanDeploymentName,
            RootVerificationCheckName::PlanNetwork,
            RootVerificationCheckName::PlanFleetTemplate,
        ],
    )?;
    ensure_report_check_names(
        &report.evidence_checks,
        &[
            RootVerificationCheckName::ExplicitObservedRoot,
            RootVerificationCheckName::RootObservationSource,
            RootVerificationCheckName::ObservedRootCanisterId,
            RootVerificationCheckName::SourceCheckId,
            RootVerificationCheckName::SourceDeploymentPlanId,
            RootVerificationCheckName::SourceInventoryId,
        ],
    )?;
    for check in report.identity_checks.iter().chain(&report.evidence_checks) {
        if check.satisfied != (check.expected == check.observed) {
            return Err(DeploymentRootVerificationReportError::CheckMismatch {
                check: check.name.clone(),
            });
        }
    }

    ensure_report_check_value(
        &report.identity_checks,
        RootVerificationCheckName::DeploymentName,
        Some(report.deployment_name.as_str()),
        report.observed_deployment_name.as_deref(),
    )?;
    ensure_report_check_value(
        &report.identity_checks,
        RootVerificationCheckName::Network,
        Some(report.network.as_str()),
        report.observed_network.as_deref(),
    )?;
    ensure_report_check_value(
        &report.identity_checks,
        RootVerificationCheckName::FleetTemplate,
        Some(report.expected_fleet_template.as_str()),
        report.observed_fleet_template.as_deref(),
    )?;
    ensure_report_check_value(
        &report.identity_checks,
        RootVerificationCheckName::RootPrincipal,
        Some(report.expected_root_principal.as_str()),
        report.observed_root_principal.as_deref(),
    )?;
    let observed_root_present = report.observed_deployment_name.is_some()
        && report.observed_network.is_some()
        && report.observed_fleet_template.is_some()
        && report.observed_root_principal.is_some()
        && report.observed_root_canister_id.is_some()
        && report.observed_root_observation_source.is_some();
    ensure_report_check_value(
        &report.evidence_checks,
        RootVerificationCheckName::ExplicitObservedRoot,
        Some("present"),
        observed_root_present.then_some("present"),
    )?;
    ensure_report_check_value(
        &report.evidence_checks,
        RootVerificationCheckName::RootObservationSource,
        Some("IcpCanisterStatus"),
        report
            .observed_root_observation_source
            .as_ref()
            .map(root_observation_source_label_from_source),
    )?;
    ensure_report_check_value(
        &report.evidence_checks,
        RootVerificationCheckName::ObservedRootCanisterId,
        Some(report.expected_root_principal.as_str()),
        report.observed_root_canister_id.as_deref(),
    )?;
    ensure_report_check_value(
        &report.evidence_checks,
        RootVerificationCheckName::SourceCheckId,
        Some("present"),
        present_value(report.source_check_id.as_str()),
    )?;
    ensure_report_check_value(
        &report.evidence_checks,
        RootVerificationCheckName::SourceDeploymentPlanId,
        Some("present"),
        present_value(report.source_deployment_plan_id.as_str()),
    )?;
    ensure_report_check_value(
        &report.evidence_checks,
        RootVerificationCheckName::SourceInventoryId,
        Some("present"),
        present_value(report.source_inventory_id.as_str()),
    )?;
    Ok(())
}

fn ensure_report_check_names(
    checks: &[DeploymentRootVerificationCheckV1],
    expected: &[RootVerificationCheckName],
) -> Result<(), DeploymentRootVerificationReportError> {
    for check in checks {
        if !expected
            .iter()
            .any(|expected_name| check.name == expected_name.label())
        {
            return Err(DeploymentRootVerificationReportError::CheckMismatch {
                check: check.name.clone(),
            });
        }
    }
    for expected_name in expected {
        let expected_name = expected_name.label();
        if checks
            .iter()
            .filter(|check| check.name == expected_name)
            .count()
            != 1
        {
            return Err(DeploymentRootVerificationReportError::CheckMismatch {
                check: expected_name.to_string(),
            });
        }
    }
    Ok(())
}

fn ensure_report_check_value(
    checks: &[DeploymentRootVerificationCheckV1],
    name: RootVerificationCheckName,
    expected: Option<&str>,
    observed: Option<&str>,
) -> Result<(), DeploymentRootVerificationReportError> {
    let name = name.label();
    let Some(check) = checks.iter().find(|check| check.name == name) else {
        return Err(DeploymentRootVerificationReportError::CheckMismatch {
            check: name.to_string(),
        });
    };
    if check.expected.as_deref() == expected
        && check.observed.as_deref() == observed
        && check.satisfied == (expected == observed)
    {
        Ok(())
    } else {
        Err(DeploymentRootVerificationReportError::CheckMismatch {
            check: name.to_string(),
        })
    }
}
const fn ensure_root_verification_field(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentRootVerificationReportError> {
    if value.is_empty() {
        Err(DeploymentRootVerificationReportError::MissingRequiredField { field })
    } else {
        Ok(())
    }
}

fn ensure_root_verification_sha256(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentRootVerificationReportError> {
    if value.is_empty() {
        return Err(DeploymentRootVerificationReportError::MissingRequiredField { field });
    }
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(DeploymentRootVerificationReportError::InvalidSha256Digest { field })
    }
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
