use super::super::*;

#[test]
fn authority_receipt_rejects_mismatched_report_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.inventory_id = "other-inventory".to_string();

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched report inventory should fail receipt construction");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportMismatch {
            field: "inventory_id",
            ..
        }
    );
}

#[test]
fn authority_receipt_rejects_mismatched_report_content() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(plan, sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.automatic_actions.clear();

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched report content should fail receipt construction");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "automatic_actions",
        }
    );
}

#[test]
fn authority_receipt_rejects_mismatched_check_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some("other-check".to_string()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched check id should fail receipt construction");

    std::assert_matches!(err, AuthorityEvidenceError::CheckIdMismatch { .. });
}

#[test]
fn authority_receipt_rejects_unsupported_source_schema_version() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let mut reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    reconciliation.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("unsupported plan schema should fail receipt construction");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "plan",
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found
        } if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1
    );
}

#[test]
fn authority_receipt_rejects_blank_receipt_identity_inputs() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        " ",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("blank receipt operation id should fail receipt construction");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.operation_id",
        }
    );
}

#[test]
fn authority_receipt_rejects_missing_report_check_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.check_id = None;

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("receipt construction should require report check provenance");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "report.check_id",
        }
    );
}

#[test]
fn authority_receipt_rejects_missing_finished_at() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        None,
    )
    .expect_err("completed dry-run receipt should require finished_at");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.finished_at",
        }
    );
}

#[test]
fn authority_receipt_rejects_finished_before_started() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:02Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("receipt construction should reject invalid timestamp order");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptTimestampOrder {
            field: "receipt.started_at",
            other_field: "receipt.finished_at",
            ..
        }
    );
}
