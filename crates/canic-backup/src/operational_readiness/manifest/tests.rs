//! Module: operational_readiness::manifest::tests
//!
//! Responsibility: guard the frozen 0.94 protocol count and identities.
//! Does not own: journey execution or product fixtures.
//! Boundary: fails when operation variants or case generators drift.

use super::*;

use std::collections::{BTreeMap, BTreeSet};

#[test]
fn frozen_protocol_manifest_has_exact_counts_and_unique_case_ids() {
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

    assert_eq!(cases.len(), 106);
    assert_eq!(case_ids.len(), cases.len());
    assert_eq!(area_counts.get(&ProtocolArea::Backup), Some(&52));
    assert_eq!(area_counts.get(&ProtocolArea::Verification), Some(&3));
    assert_eq!(area_counts.get(&ProtocolArea::Restore), Some(&41));
    assert_eq!(area_counts.get(&ProtocolArea::Rejection), Some(&10));
}

#[test]
fn frozen_protocol_manifest_covers_every_named_design_point() {
    let cases = protocol_cases();
    let point_ids = cases
        .iter()
        .map(|case| case.point_id)
        .collect::<BTreeSet<_>>();
    let expected = (1..=18)
        .map(|number| format!("CANIC-094-B{number:02}"))
        .chain((1..=3).map(|number| format!("CANIC-094-V{number:02}")))
        .chain((1..=14).map(|number| format!("CANIC-094-R{number:02}")))
        .chain((1..=10).map(|number| format!("CANIC-094-C{number:02}")))
        .collect::<BTreeSet<_>>();

    assert_eq!(
        point_ids,
        expected.iter().map(String::as_str).collect::<BTreeSet<_>>()
    );
}

#[test]
fn frozen_protocol_manifest_retains_exact_variant_multipliers() {
    let cases = protocol_cases();
    let point_counts = cases.iter().fold(BTreeMap::new(), |mut counts, case| {
        *counts.entry(case.point_id).or_insert(0usize) += 1;
        counts
    });

    assert_eq!(point_counts["CANIC-094-B04"], 12);
    assert_eq!(point_counts["CANIC-094-B16"], 12);
    assert_eq!(point_counts["CANIC-094-B18"], 4);
    assert_eq!(point_counts["CANIC-094-R04"], 12);
    assert_eq!(point_counts["CANIC-094-R12"], 12);
    assert_eq!(point_counts["CANIC-094-R14"], 4);
    assert!(cases.iter().all(|case| {
        !case.case_id.is_empty()
            && match &case.subject {
                ProtocolSubject::Boundary(label) => !label.is_empty(),
                ProtocolSubject::BackupOperation(operation) => {
                    !backup_operation_label(operation).is_empty()
                }
                ProtocolSubject::RestoreOperation(operation) => {
                    !restore_operation_label(operation).is_empty()
                }
            }
            && !position_label(case.position).is_empty()
    }));
}
