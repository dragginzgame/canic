use super::*;
use crate::fleet_ensure::model::funding_observation::{
    FundingObservationReviewRecord, FundingQuoteStage,
};
use std::io;

/// Exercise native quote stages using the generated estate's validated complete observation pass.
pub(in crate::fleet_ensure) fn qualify_observation_quotes(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    record: &FundingObservationReviewRecord,
) {
    let root = &record.body.root;
    let base = record.body.recovery_floor_cycles;
    assert_eq!(
        observation_minimum::<io::Error>(plan, journal, root, None, base).unwrap(),
        base
    );
    let recovery = observation_source(plan, journal, root).unwrap();
    assert_eq!(recovery.stage, FundingQuoteStage::Recovery);
    assert_eq!(
        observation_minimum::<io::Error>(plan, journal, root, Some(&recovery), base).unwrap(),
        base + 30_000_000_000_000
    );
    let mut incomplete = journal.clone();
    let review = incomplete.funding_observations.get_mut(root).unwrap();
    review.attempts.clear();
    review.final_root_cycles = None;
    review.approved = false;
    let observation = observation_source(plan, &incomplete, root).unwrap();
    assert_eq!(observation.stage, FundingQuoteStage::Observation);
    assert_ne!(recovery, observation);
    assert_eq!(
        observation_minimum::<io::Error>(plan, &incomplete, root, Some(&observation), base)
            .unwrap(),
        base + record.body.maximum_cycles
    );
    assert!(
        observation_minimum::<io::Error>(plan, &incomplete, root, Some(&recovery), base).is_err()
    );
    let mut changed = recovery;
    changed.review_sha256 = "ab".repeat(32);
    assert!(matches!(
        observation_minimum::<io::Error>(plan, journal, root, Some(&changed), base),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
}
