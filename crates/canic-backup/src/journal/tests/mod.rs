use super::*;

const ROOT: &str = "aaaaa-aa";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Build one valid durable journal for validation tests.
fn valid_journal() -> DownloadJournal {
    DownloadJournal {
        journal_version: 1,
        backup_id: "fbk_test_001".to_string(),
        discovery_topology_hash: Some(HASH.to_string()),
        pre_snapshot_topology_hash: Some(HASH.to_string()),
        operation_metrics: DownloadOperationMetrics::default(),
        artifacts: vec![ArtifactJournalEntry {
            canister_id: ROOT.to_string(),
            snapshot_id: "snap-1".to_string(),
            state: ArtifactState::Durable,
            temp_path: None,
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(HASH.to_string()),
            updated_at: "2026-04-10T12:00:00Z".to_string(),
        }],
    }
}

// Ensure durable artifact journals validate.
#[test]
fn valid_journal_passes_validation() {
    let journal = valid_journal();

    journal.validate().expect("journal should validate");
}

// Ensure state determines the next idempotent resume action.
#[test]
fn resume_action_matches_artifact_state() {
    let mut entry = valid_journal().artifacts.remove(0);

    entry.state = ArtifactState::Created;
    assert_eq!(entry.resume_action(), ResumeAction::Download);

    entry.state = ArtifactState::Downloaded;
    assert_eq!(entry.resume_action(), ResumeAction::VerifyChecksum);

    entry.state = ArtifactState::ChecksumVerified;
    assert_eq!(entry.resume_action(), ResumeAction::Finalize);

    entry.state = ArtifactState::Durable;
    assert_eq!(entry.resume_action(), ResumeAction::Skip);
}

// Ensure resume reports summarize states and next idempotent actions.
#[test]
fn resume_report_counts_states_and_actions() {
    let mut journal = valid_journal();
    journal.artifacts[0].state = ArtifactState::Created;
    journal.artifacts[0].checksum = None;
    let mut downloaded = journal.artifacts[0].clone();
    downloaded.snapshot_id = "snap-2".to_string();
    downloaded.state = ArtifactState::Downloaded;
    downloaded.temp_path = Some("artifacts/root.tmp".to_string());
    let mut durable = valid_journal().artifacts.remove(0);
    durable.snapshot_id = "snap-3".to_string();
    journal.artifacts.push(downloaded);
    journal.artifacts.push(durable);

    let report = journal.resume_report();

    assert_eq!(report.total_artifacts, 3);
    assert_eq!(report.discovery_topology_hash.as_deref(), Some(HASH));
    assert_eq!(report.pre_snapshot_topology_hash.as_deref(), Some(HASH));
    assert!(!report.is_complete);
    assert_eq!(report.pending_artifacts, 2);
    assert_eq!(report.counts.created, 1);
    assert_eq!(report.counts.downloaded, 1);
    assert_eq!(report.counts.durable, 1);
    assert_eq!(report.counts.download, 1);
    assert_eq!(report.counts.verify_checksum, 1);
    assert_eq!(report.counts.skip, 1);
    assert_eq!(report.artifacts[0].resume_action, ResumeAction::Download);
}

// Ensure journal transitions cannot move backward.
#[test]
fn state_transitions_are_monotonic() {
    let mut entry = valid_journal().artifacts.remove(0);

    let err = entry
        .advance_to(
            ArtifactState::Downloaded,
            "2026-04-10T12:01:00Z".to_string(),
        )
        .expect_err("durable cannot move back to downloaded");

    assert!(matches!(
        err,
        JournalValidationError::InvalidStateTransition { .. }
    ));
}

// Ensure checksum is required once an artifact is durable.
#[test]
fn durable_artifact_requires_checksum() {
    let mut journal = valid_journal();
    journal.artifacts[0].checksum = None;

    let err = journal
        .validate()
        .expect_err("durable artifact without checksum should fail");

    assert!(matches!(err, JournalValidationError::EmptyField(_)));
}

// Ensure duplicate canister/snapshot rows are rejected.
#[test]
fn duplicate_artifacts_fail_validation() {
    let mut journal = valid_journal();
    journal.artifacts.push(journal.artifacts[0].clone());

    let err = journal
        .validate()
        .expect_err("duplicate artifact should fail");

    assert!(matches!(
        err,
        JournalValidationError::DuplicateArtifact { .. }
    ));
}

// Ensure journals round-trip through the JSON format.
#[test]
fn journal_round_trips_through_json() {
    let journal = valid_journal();

    let encoded = serde_json::to_string(&journal).expect("serialize journal");
    let decoded: DownloadJournal = serde_json::from_str(&encoded).expect("deserialize journal");

    decoded.validate().expect("decoded journal should validate");
}
