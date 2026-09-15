//! Module: ops::cascade_report::tests
//!
//! Responsibility: qualify bounded state-cascade observations and malformed replies.
//! Does not own: IC transport or application state.
//! Boundary: counts, identities and encoded response size remain independently checked.

use super::*;

fn principal(index: usize) -> Principal {
    let mut bytes = [255; 29];
    bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
    Principal::from_slice(&bytes)
}

#[test]
fn bounded_details_preserve_all_outcomes_and_failures() {
    let mut report = StateCascadeReport::default();
    for index in 0..MAX_REPORTED_STATE_TARGETS + 7 {
        StateCascadeReportOps::merge(
            &mut report,
            StateCascadeReportOps::applied(principal(index), None),
        )
        .unwrap();
    }
    let error = InternalError::conflict();
    StateCascadeReportOps::merge(
        &mut report,
        StateCascadeReportOps::unconfirmed(principal(1000), error),
    )
    .unwrap();
    assert_eq!(report.targets.len(), MAX_REPORTED_STATE_TARGETS);
    assert_eq!(report.omitted_targets, 8);
    assert_eq!(
        report.successful_targets,
        (MAX_REPORTED_STATE_TARGETS + 7) as u64
    );
    assert_eq!(report.unconfirmed_targets, 1);
    assert!(StateCascadeReportOps::require_complete(&report).is_err());
}

#[test]
fn bounded_report_and_local_receipt_fit_detail_budget() {
    let mut report = StateCascadeReport::default();
    for index in 0..MAX_REPORTED_STATE_TARGETS {
        StateCascadeReportOps::merge(
            &mut report,
            StateCascadeReportOps::applied(
                principal(index),
                Some(InternalError::conflict().into()),
            ),
        )
        .unwrap();
    }
    let response: Result<_, Error> = Ok(crate::dto::state::FleetStateCommandResult {
        change: crate::dto::state::SetStateResponse {
            previous: true,
            current: false,
            changed: true,
        },
        propagation: report,
        reconciliation_error: Some(InternalError::conflict().into()),
    });
    let bytes = candid::encode_one(response).unwrap();
    assert!(bytes.len() < CASCADE_SNAPSHOT_MAX_BYTES);
}

#[test]
fn replies_bind_local_application_and_reconciliation_to_recipient() {
    let report =
        StateCascadeReportOps::applied(principal(1), Some(InternalError::conflict().into()));
    StateCascadeReportOps::validate_reply(principal(1), &report).unwrap();
    assert!(StateCascadeReportOps::require_complete(&report).is_err());
    assert!(StateCascadeReportOps::validate_reply(principal(2), &report).is_err());
    let unconfirmed = StateCascadeReportOps::unconfirmed(principal(1), InternalError::conflict());
    assert!(StateCascadeReportOps::validate_reply(principal(1), &unconfirmed).is_err());
    let applied = StateCascadeReportOps::applied(principal(1), None);
    StateCascadeReportOps::require_complete(&applied).unwrap();
}

#[test]
fn malformed_aggregation_is_rejected_before_mutating_observations() {
    let initial = StateCascadeReportOps::applied(principal(1), None);
    let mut duplicate = initial.clone();
    duplicate.targets.push(duplicate.targets[0].clone());
    duplicate.successful_targets += 1;
    let mut inconsistent = StateCascadeReportOps::applied(principal(2), None);
    inconsistent.unconfirmed_targets = 1;
    let overflow = StateCascadeReport {
        successful_targets: u64::MAX,
        omitted_targets: u64::MAX - 1,
        ..StateCascadeReportOps::applied(principal(2), None)
    };
    for invalid in [initial.clone(), duplicate, inconsistent, overflow] {
        let mut report = initial.clone();
        assert!(StateCascadeReportOps::merge(&mut report, invalid).is_err());
        assert_eq!(report, initial);
    }
}
