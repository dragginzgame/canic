use super::super::super::*;
use thiserror::Error as ThisError;

///
/// AuthorityEvidenceError
///
#[derive(Debug, ThisError)]
pub enum AuthorityEvidenceError {
    #[error("authority evidence is missing required field: {field}")]
    MissingRequiredField { field: &'static str },

    #[error(
        "authority evidence {component} has unsupported schema version: expected {expected}, found {found}"
    )]
    SchemaVersionMismatch {
        component: &'static str,
        expected: u32,
        found: u32,
    },

    #[error(
        "authority report does not match reconciliation plan: {field} differs (plan={plan_value}, report={report_value})"
    )]
    PlanReportMismatch {
        field: &'static str,
        plan_value: String,
        report_value: String,
    },

    #[error("authority report content does not match reconciliation plan: {field} differs")]
    PlanReportContentMismatch { field: &'static str },

    #[error("authority dry-run receipt contains attempted controller actions: {count}")]
    DryRunReceiptAttemptedActions { count: usize },

    #[error("authority dry-run receipt has invalid operation status: {status:?}")]
    DryRunReceiptStatus { status: DeploymentExecutionStatusV1 },

    #[error("authority dry-run receipt has invalid command result: {result:?}")]
    DryRunReceiptCommandResult { result: DeploymentCommandResultV1 },

    #[error("authority dry-run receipt is complete but has no finished_at timestamp")]
    DryRunReceiptMissingFinishedAt,

    #[error(
        "authority evidence generated_at does not match receipt finished_at (evidence={evidence_value}, receipt={receipt_value})"
    )]
    EvidenceGeneratedAtMismatch {
        evidence_value: String,
        receipt_value: String,
    },

    #[error(
        "authority dry-run receipt has invalid timestamp order: {field} ({left}) is after {other_field} ({right})"
    )]
    DryRunReceiptTimestampOrder {
        field: &'static str,
        left: String,
        other_field: &'static str,
        right: String,
    },

    #[error(
        "authority receipt check id does not match report check id (receipt={receipt_value}, report={report_value})"
    )]
    CheckIdMismatch {
        receipt_value: String,
        report_value: String,
    },

    #[error(
        "authority evidence check id does not match nested {component} check id (evidence={evidence_value}, nested={nested_value})"
    )]
    EvidenceCheckIdMismatch {
        component: &'static str,
        evidence_value: String,
        nested_value: String,
    },
}
