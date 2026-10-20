//! Module: operational_readiness::manifest
//!
//! Responsibility: generate the maintained protocol-case inventory.
//! Does not own: crash injection, product state, or journey results.
//! Boundary: maps design point IDs to exact variants and interruption sides.

#[cfg(test)]
mod tests;

use crate::restore::RestoreApplyOperationKind;

/// Test-only maintained protocol area.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProtocolArea {
    Backup,
    Verification,
    Restore,
    Rejection,
}

/// Observable interruption position represented by one protocol case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptionPosition {
    BeforeDurableWrite,
    AfterDurableWrite,
    EffectCommittedReceiptMissing,
    OwnerDeadCommandInFlight,
    Interrupted,
    ResponseLostAfterPersistence,
    Rejection,
}

/// Canonical operation or boundary exercised by one protocol case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolSubject {
    Boundary(&'static str),
    RestoreOperation(RestoreApplyOperationKind),
}

/// One uniquely identified maintained protocol case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolCase {
    pub case_id: String,
    pub point_id: &'static str,
    pub area: ProtocolArea,
    pub subject: ProtocolSubject,
    pub position: InterruptionPosition,
}

/// Generate the complete maintained protocol inventory.
pub fn protocol_cases() -> Vec<ProtocolCase> {
    let mut cases = Vec::new();
    append_backup_cases(&mut cases);
    append_verification_cases(&mut cases);
    append_restore_cases(&mut cases);
    append_rejection_cases(&mut cases);
    cases
}

/// Require one exact case identity to remain in the maintained manifest.
pub fn assert_case_defined(case_id: &str) {
    assert!(
        protocol_cases().iter().any(|case| case.case_id == case_id),
        "protocol case is not defined: {case_id}"
    );
}

fn append_backup_cases(cases: &mut Vec<ProtocolCase>) {
    append_write_sides(
        cases,
        "CANIC-094-B01",
        ProtocolArea::Backup,
        ProtocolSubject::Boundary("execution-journal-publication"),
    );
}

fn append_verification_cases(cases: &mut Vec<ProtocolCase>) {
    for (point_id, boundary) in [
        ("CANIC-094-V01", "before-document-validation"),
        ("CANIC-094-V02", "during-artifact-checksum"),
        ("CANIC-094-V03", "after-result-before-output"),
    ] {
        push_case(
            cases,
            point_id,
            ProtocolArea::Verification,
            ProtocolSubject::Boundary(boundary),
            InterruptionPosition::Interrupted,
        );
    }
}

fn append_restore_cases(cases: &mut Vec<ProtocolCase>) {
    append_write_sides(
        cases,
        "CANIC-094-R01",
        ProtocolArea::Restore,
        ProtocolSubject::Boundary("restore-plan-publication"),
    );
    append_write_sides(
        cases,
        "CANIC-094-R02",
        ProtocolArea::Restore,
        ProtocolSubject::Boundary("apply-journal-publication"),
    );
    push_case(
        cases,
        "CANIC-094-R03",
        ProtocolArea::Restore,
        ProtocolSubject::Boundary("private-upload-staging"),
        InterruptionPosition::Interrupted,
    );
    append_restore_operation_write_sides(cases, "CANIC-094-R04", restore_operations());
    push_case(
        cases,
        "CANIC-094-R05",
        ProtocolArea::Restore,
        ProtocolSubject::Boundary("stopped-canister-precondition"),
        InterruptionPosition::Interrupted,
    );
    for (point_id, operation) in [
        ("CANIC-094-R06", RestoreApplyOperationKind::UploadSnapshot),
        ("CANIC-094-R07", RestoreApplyOperationKind::StopCanister),
        ("CANIC-094-R08", RestoreApplyOperationKind::LoadSnapshot),
        ("CANIC-094-R09", RestoreApplyOperationKind::StartCanister),
        ("CANIC-094-R10", RestoreApplyOperationKind::VerifyMember),
        ("CANIC-094-R11", RestoreApplyOperationKind::VerifyDeployment),
    ] {
        push_case(
            cases,
            point_id,
            ProtocolArea::Restore,
            ProtocolSubject::RestoreOperation(operation),
            InterruptionPosition::EffectCommittedReceiptMissing,
        );
    }
    append_restore_operation_write_sides(cases, "CANIC-094-R12", restore_operations());
    push_case(
        cases,
        "CANIC-094-R13",
        ProtocolArea::Restore,
        ProtocolSubject::Boundary("final-successful-response"),
        InterruptionPosition::ResponseLostAfterPersistence,
    );
    for operation in restore_mutating_operations() {
        push_case(
            cases,
            "CANIC-094-R14",
            ProtocolArea::Restore,
            ProtocolSubject::RestoreOperation(operation),
            InterruptionPosition::OwnerDeadCommandInFlight,
        );
    }
}

fn append_rejection_cases(cases: &mut Vec<ProtocolCase>) {
    for (point_id, subject) in [
        ("CANIC-094-C01", "invalid-json"),
        ("CANIC-094-C02", "identity-or-operation-mismatch"),
        ("CANIC-094-C04", "unsafe-or-invalid-artifact"),
        ("CANIC-094-C05", "partial-private-stage"),
        ("CANIC-094-C06", "publication-journal-disagreement"),
        ("CANIC-094-C07", "lock-evidence"),
        ("CANIC-094-C08", "terminal-receipt-disagreement"),
        ("CANIC-094-C09", "missing-command-identity"),
        ("CANIC-094-C10", "persistence-failure"),
    ] {
        push_case(
            cases,
            point_id,
            ProtocolArea::Rejection,
            ProtocolSubject::Boundary(subject),
            InterruptionPosition::Rejection,
        );
    }
}

fn append_write_sides(
    cases: &mut Vec<ProtocolCase>,
    point_id: &'static str,
    area: ProtocolArea,
    subject: ProtocolSubject,
) {
    for position in [
        InterruptionPosition::BeforeDurableWrite,
        InterruptionPosition::AfterDurableWrite,
    ] {
        push_case(cases, point_id, area, subject.clone(), position);
    }
}

fn append_restore_operation_write_sides(
    cases: &mut Vec<ProtocolCase>,
    point_id: &'static str,
    operations: Vec<RestoreApplyOperationKind>,
) {
    for operation in operations {
        append_write_sides(
            cases,
            point_id,
            ProtocolArea::Restore,
            ProtocolSubject::RestoreOperation(operation),
        );
    }
}

fn push_case(
    cases: &mut Vec<ProtocolCase>,
    point_id: &'static str,
    area: ProtocolArea,
    subject: ProtocolSubject,
    position: InterruptionPosition,
) {
    let case_id = format!(
        "{point_id}/{}/{}",
        subject_label(&subject),
        position_label(position)
    );
    cases.push(ProtocolCase {
        case_id,
        point_id,
        area,
        subject,
        position,
    });
}

pub fn restore_operations() -> Vec<RestoreApplyOperationKind> {
    vec![
        RestoreApplyOperationKind::UploadSnapshot,
        RestoreApplyOperationKind::StopCanister,
        RestoreApplyOperationKind::LoadSnapshot,
        RestoreApplyOperationKind::StartCanister,
        RestoreApplyOperationKind::VerifyMember,
        RestoreApplyOperationKind::VerifyDeployment,
    ]
}

fn restore_mutating_operations() -> Vec<RestoreApplyOperationKind> {
    vec![
        RestoreApplyOperationKind::UploadSnapshot,
        RestoreApplyOperationKind::StopCanister,
        RestoreApplyOperationKind::LoadSnapshot,
        RestoreApplyOperationKind::StartCanister,
    ]
}

fn subject_label(subject: &ProtocolSubject) -> &'static str {
    match subject {
        ProtocolSubject::Boundary(label) => label,
        ProtocolSubject::RestoreOperation(operation) => restore_operation_label(operation),
    }
}

pub const fn restore_operation_label(operation: &RestoreApplyOperationKind) -> &'static str {
    match operation {
        RestoreApplyOperationKind::StopCanister => "stop-canister",
        RestoreApplyOperationKind::StartCanister => "start-canister",
        RestoreApplyOperationKind::UploadSnapshot => "upload-snapshot",
        RestoreApplyOperationKind::LoadSnapshot => "load-snapshot",
        RestoreApplyOperationKind::VerifyMember => "verify-member",
        RestoreApplyOperationKind::VerifyDeployment => "verify-deployment",
    }
}

const fn position_label(position: InterruptionPosition) -> &'static str {
    match position {
        InterruptionPosition::BeforeDurableWrite => "before-durable-write",
        InterruptionPosition::AfterDurableWrite => "after-durable-write",
        InterruptionPosition::EffectCommittedReceiptMissing => "effect-committed-receipt-missing",
        InterruptionPosition::OwnerDeadCommandInFlight => "owner-dead-command-in-flight",
        InterruptionPosition::Interrupted => "interrupted",
        InterruptionPosition::ResponseLostAfterPersistence => "response-lost-after-persistence",
        InterruptionPosition::Rejection => "rejection",
    }
}
