use super::*;
use serde::Serialize;
use thiserror::Error as ThisError;

#[derive(Serialize)]
struct DeploymentRootVerificationReportDigestInput<'a> {
    report_id: &'a str,
    requested_at: &'a str,
    evidence_status: DeploymentRootVerificationEvidenceStatusV1,
    state_transition: DeploymentRootVerificationStateTransitionV1,
    deployment_name: &'a str,
    network: &'a str,
    expected_fleet_template: &'a str,
    expected_root_principal: &'a str,
    observed_deployment_name: &'a Option<String>,
    observed_network: &'a Option<String>,
    observed_fleet_template: &'a Option<String>,
    observed_root_principal: &'a Option<String>,
    source: DeploymentRootVerificationSourceV1,
    source_check_id: &'a str,
    source_check_digest: &'a str,
    source_deployment_plan_id: &'a str,
    source_deployment_plan_digest: &'a str,
    source_inventory_id: &'a str,
    source_inventory_digest: &'a str,
    current_root_verification: DeploymentRootVerificationStateV1,
    identity_checks: &'a [DeploymentRootVerificationCheckV1],
    evidence_checks: &'a [DeploymentRootVerificationCheckV1],
    blockers: &'a [SafetyFindingV1],
    warnings: &'a [SafetyFindingV1],
    recommended_next_actions: &'a [String],
}

#[derive(Serialize)]
struct DeploymentRootVerificationReceiptDigestInput<'a> {
    receipt_id: &'a str,
    deployment_name: &'a str,
    network: &'a str,
    fleet_template: &'a str,
    root_principal: &'a str,
    previous_root_verification: DeploymentRootVerificationStateV1,
    new_root_verification: DeploymentRootVerificationStateV1,
    state_transition: DeploymentRootVerificationStateTransitionV1,
    source_report_id: &'a str,
    source_report_digest: &'a str,
    source_check_id: &'a str,
    source_check_digest: &'a str,
    source_deployment_plan_id: &'a str,
    source_deployment_plan_digest: &'a str,
    source_inventory_id: &'a str,
    source_inventory_digest: &'a str,
    verified_at_unix_secs: u64,
    local_state_path: &'a str,
    local_state_digest_before: &'a str,
    local_state_digest_after: &'a str,
    warnings: &'a [SafetyFindingV1],
}

///
/// DeploymentRootVerificationReportError
///
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum DeploymentRootVerificationReportError {
    #[error(
        "deployment root verification report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },

    #[error("deployment root verification report field `{field}` is required")]
    MissingRequiredField { field: &'static str },

    #[error("deployment root verification report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },

    #[error("deployment root verification report status is inconsistent")]
    StatusMismatch,
}

///
/// DeploymentRootVerificationReceiptError
///
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum DeploymentRootVerificationReceiptError {
    #[error(
        "deployment root verification receipt schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },

    #[error("deployment root verification receipt field `{field}` is required")]
    MissingRequiredField { field: &'static str },

    #[error("deployment root verification receipt field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },

    #[error("deployment root verification receipt state transition is inconsistent")]
    StateTransitionMismatch,
}

/// Build a passive 0.47 root-verification report from an existing
/// deployment-truth check.
///
/// This report can prove evidence consistency, but it does not mutate local
/// deployment state or record verified root state.
#[must_use]
pub fn deployment_root_verification_report_from_check(
    request: DeploymentRootVerificationRequestV1,
) -> DeploymentRootVerificationReportV1 {
    let check = &request.deployment_check;
    let observed_root = check.inventory.observed_root.as_ref();
    let identity_checks = root_verification_identity_checks(&request, check, observed_root);
    let evidence_checks = root_verification_evidence_checks(&request, check, observed_root);
    let blockers = root_verification_blockers(&identity_checks, &evidence_checks, check);

    let evidence_status = if blockers.is_empty() {
        DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied
    } else {
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed
    };
    let state_transition =
        root_verification_transition(evidence_status, request.current_root_verification);
    let recommended_next_actions = root_verification_next_actions(evidence_status);
    let mut report = DeploymentRootVerificationReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: request.report_id,
        report_digest: String::new(),
        requested_at: request.requested_at,
        evidence_status,
        state_transition,
        deployment_name: request.deployment_name,
        network: request.network,
        expected_fleet_template: request.expected_fleet_template,
        expected_root_principal: request.expected_root_principal,
        observed_deployment_name: observed_root.map(|root| root.deployment_name.clone()),
        observed_network: observed_root.map(|root| root.network.clone()),
        observed_fleet_template: observed_root.map(|root| root.fleet_template.clone()),
        observed_root_principal: observed_root.map(|root| root.root_principal.clone()),
        source: request.source,
        source_check_id: check.check_id.clone(),
        source_check_digest: stable_json_sha256_hex(check),
        source_deployment_plan_id: check.plan.plan_id.clone(),
        source_deployment_plan_digest: stable_json_sha256_hex(&check.plan),
        source_inventory_id: check.inventory.inventory_id.clone(),
        source_inventory_digest: stable_json_sha256_hex(&check.inventory),
        current_root_verification: request.current_root_verification,
        identity_checks,
        evidence_checks,
        blockers,
        warnings: check.report.warnings.clone(),
        recommended_next_actions,
    };
    report.report_digest = deployment_root_verification_report_digest(&report);
    report
}

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
    ensure_root_verification_field("report_digest", report.report_digest.as_str())?;
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
    ensure_root_verification_field("source_check_digest", report.source_check_digest.as_str())?;
    ensure_root_verification_field(
        "source_deployment_plan_id",
        report.source_deployment_plan_id.as_str(),
    )?;
    ensure_root_verification_field(
        "source_deployment_plan_digest",
        report.source_deployment_plan_digest.as_str(),
    )?;
    ensure_root_verification_field("source_inventory_id", report.source_inventory_id.as_str())?;
    ensure_root_verification_field(
        "source_inventory_digest",
        report.source_inventory_digest.as_str(),
    )?;
    if report.evidence_status != report_evidence_status(report)
        || report.state_transition != report_state_transition(report)
    {
        return Err(DeploymentRootVerificationReportError::StatusMismatch);
    }
    if report.report_digest != deployment_root_verification_report_digest(report) {
        return Err(DeploymentRootVerificationReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Calculate the stable digest for a root-verification state-transition
/// receipt.
#[must_use]
pub fn deployment_root_verification_receipt_digest(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> String {
    stable_json_sha256_hex(&DeploymentRootVerificationReceiptDigestInput {
        receipt_id: &receipt.receipt_id,
        deployment_name: &receipt.deployment_name,
        network: &receipt.network,
        fleet_template: &receipt.fleet_template,
        root_principal: &receipt.root_principal,
        previous_root_verification: receipt.previous_root_verification,
        new_root_verification: receipt.new_root_verification,
        state_transition: receipt.state_transition,
        source_report_id: &receipt.source_report_id,
        source_report_digest: &receipt.source_report_digest,
        source_check_id: &receipt.source_check_id,
        source_check_digest: &receipt.source_check_digest,
        source_deployment_plan_id: &receipt.source_deployment_plan_id,
        source_deployment_plan_digest: &receipt.source_deployment_plan_digest,
        source_inventory_id: &receipt.source_inventory_id,
        source_inventory_digest: &receipt.source_inventory_digest,
        verified_at_unix_secs: receipt.verified_at_unix_secs,
        local_state_path: &receipt.local_state_path,
        local_state_digest_before: &receipt.local_state_digest_before,
        local_state_digest_after: &receipt.local_state_digest_after,
        warnings: &receipt.warnings,
    })
}

/// Validate archived root-verification receipt consistency and digest
/// stability.
pub fn validate_deployment_root_verification_receipt(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> Result<(), DeploymentRootVerificationReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            DeploymentRootVerificationReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: receipt.schema_version,
            },
        );
    }
    ensure_root_verification_receipt_field("receipt_id", receipt.receipt_id.as_str())?;
    ensure_root_verification_receipt_field("receipt_digest", receipt.receipt_digest.as_str())?;
    ensure_root_verification_receipt_field("deployment_name", receipt.deployment_name.as_str())?;
    ensure_root_verification_receipt_field("network", receipt.network.as_str())?;
    ensure_root_verification_receipt_field("fleet_template", receipt.fleet_template.as_str())?;
    ensure_root_verification_receipt_field("root_principal", receipt.root_principal.as_str())?;
    ensure_root_verification_receipt_field("source_report_id", receipt.source_report_id.as_str())?;
    ensure_root_verification_receipt_field(
        "source_report_digest",
        receipt.source_report_digest.as_str(),
    )?;
    ensure_root_verification_receipt_field("source_check_id", receipt.source_check_id.as_str())?;
    ensure_root_verification_receipt_field(
        "source_check_digest",
        receipt.source_check_digest.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "source_deployment_plan_id",
        receipt.source_deployment_plan_id.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "source_deployment_plan_digest",
        receipt.source_deployment_plan_digest.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "source_inventory_id",
        receipt.source_inventory_id.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "source_inventory_digest",
        receipt.source_inventory_digest.as_str(),
    )?;
    ensure_root_verification_receipt_field("local_state_path", receipt.local_state_path.as_str())?;
    ensure_root_verification_receipt_field(
        "local_state_digest_before",
        receipt.local_state_digest_before.as_str(),
    )?;
    ensure_root_verification_receipt_field(
        "local_state_digest_after",
        receipt.local_state_digest_after.as_str(),
    )?;

    if receipt.new_root_verification != DeploymentRootVerificationStateV1::Verified
        || receipt.state_transition != receipt_state_transition(receipt)
    {
        return Err(DeploymentRootVerificationReceiptError::StateTransitionMismatch);
    }
    if receipt.receipt_digest != deployment_root_verification_receipt_digest(receipt) {
        return Err(DeploymentRootVerificationReceiptError::DigestMismatch {
            field: "receipt_digest",
        });
    }
    Ok(())
}

fn root_verification_identity_checks(
    request: &DeploymentRootVerificationRequestV1,
    check: &DeploymentCheckV1,
    observed_root: Option<&DeploymentRootObservationV1>,
) -> Vec<DeploymentRootVerificationCheckV1> {
    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "deployment_name",
        Some(request.deployment_name.as_str()),
        observed_root.map(|root| root.deployment_name.as_str()),
    );
    push_check(
        &mut checks,
        "network",
        Some(request.network.as_str()),
        observed_root.map(|root| root.network.as_str()),
    );
    push_check(
        &mut checks,
        "fleet_template",
        Some(request.expected_fleet_template.as_str()),
        observed_root.map(|root| root.fleet_template.as_str()),
    );
    push_check(
        &mut checks,
        "root_principal",
        Some(request.expected_root_principal.as_str()),
        observed_root.map(|root| root.root_principal.as_str()),
    );
    push_check(
        &mut checks,
        "plan_deployment_name",
        Some(request.deployment_name.as_str()),
        Some(check.plan.deployment_identity.deployment_name.as_str()),
    );
    push_check(
        &mut checks,
        "plan_network",
        Some(request.network.as_str()),
        Some(check.plan.deployment_identity.network.as_str()),
    );
    push_check(
        &mut checks,
        "plan_fleet_template",
        Some(request.expected_fleet_template.as_str()),
        Some(check.plan.fleet_template.as_str()),
    );
    checks
}

fn root_verification_evidence_checks(
    request: &DeploymentRootVerificationRequestV1,
    check: &DeploymentCheckV1,
    observed_root: Option<&DeploymentRootObservationV1>,
) -> Vec<DeploymentRootVerificationCheckV1> {
    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "explicit_observed_root",
        Some("present"),
        observed_root.map(|_| "present"),
    );
    push_check(
        &mut checks,
        "root_observation_source",
        Some("IcpCanisterStatus"),
        observed_root.map(root_observation_source_label),
    );
    push_check(
        &mut checks,
        "observed_root_canister_id",
        Some(request.expected_root_principal.as_str()),
        observed_root.map(|root| root.observed_canister_id.as_str()),
    );
    push_check(
        &mut checks,
        "source_check_id",
        Some("present"),
        present_value(check.check_id.as_str()),
    );
    push_check(
        &mut checks,
        "source_deployment_plan_id",
        Some("present"),
        present_value(check.plan.plan_id.as_str()),
    );
    push_check(
        &mut checks,
        "source_inventory_id",
        Some("present"),
        present_value(check.inventory.inventory_id.as_str()),
    );
    checks
}

fn root_verification_blockers(
    identity_checks: &[DeploymentRootVerificationCheckV1],
    evidence_checks: &[DeploymentRootVerificationCheckV1],
    check: &DeploymentCheckV1,
) -> Vec<SafetyFindingV1> {
    let mut blockers = failed_checks("identity", identity_checks);
    blockers.extend(failed_checks("evidence", evidence_checks));
    blockers.extend(source_check_blockers(check));
    blockers
}

fn push_check(
    checks: &mut Vec<DeploymentRootVerificationCheckV1>,
    name: impl Into<String>,
    expected: Option<&str>,
    observed: Option<&str>,
) {
    checks.push(DeploymentRootVerificationCheckV1 {
        name: name.into(),
        expected: expected.map(str::to_string),
        observed: observed.map(str::to_string),
        satisfied: expected == observed,
    });
}

const fn present_value(value: &str) -> Option<&'static str> {
    if value.is_empty() {
        None
    } else {
        Some("present")
    }
}

const fn root_observation_source_label(root: &DeploymentRootObservationV1) -> &str {
    match root.observation_source {
        DeploymentRootObservationSourceV1::IcpCanisterStatus => "IcpCanisterStatus",
        DeploymentRootObservationSourceV1::LocalDeploymentState => "LocalDeploymentState",
    }
}

fn failed_checks(
    category: &'static str,
    checks: &[DeploymentRootVerificationCheckV1],
) -> Vec<SafetyFindingV1> {
    checks
        .iter()
        .filter(|check| !check.satisfied)
        .map(|check| SafetyFindingV1 {
            code: "root_verification_check_failed".to_string(),
            message: format!("{category} check {} did not match", check.name),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(check.name.clone()),
        })
        .collect()
}

fn source_check_blockers(check: &DeploymentCheckV1) -> Vec<SafetyFindingV1> {
    let hard_failures = &check.report.hard_failures;
    if hard_failures.is_empty() {
        return Vec::new();
    }
    if hard_failures.len() == 1 && is_expected_unverified_root_finding(&hard_failures[0]) {
        return Vec::new();
    }
    hard_failures.clone()
}

fn is_expected_unverified_root_finding(finding: &SafetyFindingV1) -> bool {
    finding.code == "unverified_deployment_root"
        && finding.subject.as_deref() == Some("local_state.unverified_root_canister_id")
}

const fn root_verification_transition(
    status: DeploymentRootVerificationEvidenceStatusV1,
    current: DeploymentRootVerificationStateV1,
) -> DeploymentRootVerificationStateTransitionV1 {
    match (status, current) {
        (
            DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied,
            DeploymentRootVerificationStateV1::NotVerified,
        ) => DeploymentRootVerificationStateTransitionV1::WouldPromoteNotVerifiedToVerified,
        (
            DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied,
            DeploymentRootVerificationStateV1::Verified,
        ) => DeploymentRootVerificationStateTransitionV1::NoStateChange,
        _ => DeploymentRootVerificationStateTransitionV1::Blocked,
    }
}

fn root_verification_next_actions(
    status: DeploymentRootVerificationEvidenceStatusV1,
) -> Vec<String> {
    match status {
        DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied => vec![
            "run the explicit root verification command to write verified local state".to_string(),
        ],
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed => vec![
            "collect a deployment-truth check with matching root evidence before verifying"
                .to_string(),
        ],
        DeploymentRootVerificationEvidenceStatusV1::NotApplicable => Vec::new(),
    }
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

const fn receipt_state_transition(
    receipt: &DeploymentRootVerificationReceiptV1,
) -> DeploymentRootVerificationStateTransitionV1 {
    match receipt.previous_root_verification {
        DeploymentRootVerificationStateV1::NotVerified => {
            DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified
        }
        DeploymentRootVerificationStateV1::Verified => {
            DeploymentRootVerificationStateTransitionV1::NoStateChange
        }
    }
}

fn deployment_root_verification_report_digest(
    report: &DeploymentRootVerificationReportV1,
) -> String {
    stable_json_sha256_hex(&DeploymentRootVerificationReportDigestInput {
        report_id: &report.report_id,
        requested_at: &report.requested_at,
        evidence_status: report.evidence_status,
        state_transition: report.state_transition,
        deployment_name: &report.deployment_name,
        network: &report.network,
        expected_fleet_template: &report.expected_fleet_template,
        expected_root_principal: &report.expected_root_principal,
        observed_deployment_name: &report.observed_deployment_name,
        observed_network: &report.observed_network,
        observed_fleet_template: &report.observed_fleet_template,
        observed_root_principal: &report.observed_root_principal,
        source: report.source,
        source_check_id: &report.source_check_id,
        source_check_digest: &report.source_check_digest,
        source_deployment_plan_id: &report.source_deployment_plan_id,
        source_deployment_plan_digest: &report.source_deployment_plan_digest,
        source_inventory_id: &report.source_inventory_id,
        source_inventory_digest: &report.source_inventory_digest,
        current_root_verification: report.current_root_verification,
        identity_checks: &report.identity_checks,
        evidence_checks: &report.evidence_checks,
        blockers: &report.blockers,
        warnings: &report.warnings,
        recommended_next_actions: &report.recommended_next_actions,
    })
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

const fn ensure_root_verification_receipt_field(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentRootVerificationReceiptError> {
    if value.is_empty() {
        Err(DeploymentRootVerificationReceiptError::MissingRequiredField { field })
    } else {
        Ok(())
    }
}
