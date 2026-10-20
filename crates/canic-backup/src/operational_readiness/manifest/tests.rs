//! Module: operational_readiness::manifest::tests
//!
//! Responsibility: guard the maintained protocol identities.
//! Does not own: journey execution or product fixtures.
//! Boundary: fails when operation variants or case generators drift.

use super::*;

use std::collections::{BTreeMap, BTreeSet};

#[test]
fn protocol_manifest_has_required_areas_and_unique_case_ids() {
    let cases = protocol_cases();
    assert_case_defined("CANIC-094-B01/execution-journal-publication/before-durable-write");
    let case_ids = cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect::<BTreeSet<_>>();
    let area_counts = cases.iter().fold(BTreeMap::new(), |mut counts, case| {
        *counts.entry(case.area).or_insert(0usize) += 1;
        counts
    });

    assert!(!cases.is_empty());
    assert_eq!(case_ids.len(), cases.len());
    for area in [
        ProtocolArea::Backup,
        ProtocolArea::Verification,
        ProtocolArea::Restore,
        ProtocolArea::Rejection,
    ] {
        assert!(area_counts.get(&area).is_some_and(|count| *count > 0));
    }
}

#[test]
fn protocol_manifest_covers_retained_design_points() {
    let cases = protocol_cases();
    let point_ids = cases
        .iter()
        .map(|case| case.point_id)
        .collect::<BTreeSet<_>>();
    let expected = (1..=1)
        .map(|number| format!("CANIC-094-B{number:02}"))
        .chain((1..=3).map(|number| format!("CANIC-094-V{number:02}")))
        .chain((1..=14).map(|number| format!("CANIC-094-R{number:02}")))
        .chain(
            (1..=10)
                .filter(|number| *number != 3)
                .map(|number| format!("CANIC-094-C{number:02}")),
        )
        .collect::<BTreeSet<_>>();

    assert_eq!(
        point_ids,
        expected.iter().map(String::as_str).collect::<BTreeSet<_>>()
    );
}

#[test]
fn protocol_manifest_retains_restore_operations() {
    let cases = protocol_cases();
    let point_counts = cases.iter().fold(BTreeMap::new(), |mut counts, case| {
        *counts.entry(case.point_id).or_insert(0usize) += 1;
        counts
    });

    assert_eq!(
        point_counts["CANIC-094-R04"],
        restore_operations().len() * 2
    );
    assert_eq!(
        point_counts["CANIC-094-R12"],
        restore_operations().len() * 2
    );
    assert_eq!(
        point_counts["CANIC-094-R14"],
        restore_mutating_operations().len()
    );
    assert!(cases.iter().all(|case| {
        !case.case_id.is_empty()
            && match &case.subject {
                ProtocolSubject::Boundary(label) => !label.is_empty(),
                ProtocolSubject::RestoreOperation(operation) => {
                    !restore_operation_label(operation).is_empty()
                }
            }
            && !position_label(case.position).is_empty()
    }));
}
