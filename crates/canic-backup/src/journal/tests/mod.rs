use super::*;

const ROOT: &str = "aaaaa-aa";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn valid_journal() -> DownloadJournal {
    DownloadJournal {
        journal_version: 1,
        backup_id: "fbk_test_001".to_string(),
        discovery_topology_hash: HASH.to_string(),
        pre_snapshot_topology_hash: HASH.to_string(),
        operation_metrics: DownloadOperationMetrics::default(),
        artifacts: vec![ArtifactJournalEntry {
            canister_id: ROOT.to_string(),
            snapshot_id: "snap-1".to_string(),
            snapshot_taken_at_timestamp: Some(1_778_709_681_897_818_005),
            snapshot_total_size_bytes: Some(272_586_987),
            state: ArtifactState::Durable,
            temp_path: None,
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(HASH.to_string()),
            updated_at: "2026-04-10T12:00:00Z".to_string(),
        }],
    }
}

#[test]
fn valid_journal_passes_validation() {
    let journal = valid_journal();

    journal.validate().expect("journal should validate");
}

#[test]
fn download_journal_unknown_field_fails_deserialize() {
    let mut value = serde_json::to_value(valid_journal()).expect("serialize journal");
    value["unexpected_field"] = serde_json::Value::Bool(true);

    let err = serde_json::from_value::<DownloadJournal>(value).expect_err("unknown field rejects");

    assert!(err.is_data());
}

#[test]
fn download_journal_requires_current_topology_and_metrics_fields() {
    for field in [
        "discovery_topology_hash",
        "pre_snapshot_topology_hash",
        "operation_metrics",
    ] {
        let mut value = serde_json::to_value(valid_journal()).expect("serialize journal");
        value.as_object_mut().expect("journal object").remove(field);

        let err = serde_json::from_value::<DownloadJournal>(value)
            .expect_err("current journal field must be present");

        assert!(err.is_data());
    }
}

#[test]
fn download_journal_requires_exact_current_optional_fields() {
    for field in [
        "snapshot_taken_at_timestamp",
        "snapshot_total_size_bytes",
        "temp_path",
        "checksum",
    ] {
        let mut value = serde_json::to_value(valid_journal()).expect("serialize journal");
        value["artifacts"][0]
            .as_object_mut()
            .expect("artifact journal object")
            .remove(field);

        let err = serde_json::from_value::<DownloadJournal>(value)
            .expect_err("current artifact journal field must be present");
        assert!(err.is_data());
    }
}

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

#[test]
fn artifact_states_reject_fields_owned_by_other_transitions() {
    let mut created = valid_journal();
    created.artifacts[0].state = ArtifactState::Created;
    created.artifacts[0].checksum = None;
    created.artifacts[0].temp_path = Some("artifacts/root.tmp".to_string());
    let error = created
        .validate()
        .expect_err("created artifact cannot claim a temp path");
    std::assert_matches!(
        error,
        JournalValidationError::UnexpectedField {
            field: "artifacts[].temp_path",
            state: ArtifactState::Created,
        }
    );

    let mut downloaded = valid_journal();
    downloaded.artifacts[0].state = ArtifactState::Downloaded;
    downloaded.artifacts[0].temp_path = Some("artifacts/root.tmp".to_string());
    let error = downloaded
        .validate()
        .expect_err("downloaded artifact cannot claim a checksum");
    std::assert_matches!(
        error,
        JournalValidationError::UnexpectedField {
            field: "artifacts[].checksum",
            state: ArtifactState::Downloaded,
        }
    );

    let mut durable = valid_journal();
    durable.artifacts[0].temp_path = Some("artifacts/root.tmp".to_string());
    let error = durable
        .validate()
        .expect_err("durable artifact cannot retain a temp path");
    std::assert_matches!(
        error,
        JournalValidationError::UnexpectedField {
            field: "artifacts[].temp_path",
            state: ArtifactState::Durable,
        }
    );
}

#[test]
fn journal_state_names_match_their_rust_variants_without_serde_rules() {
    assert_eq!(
        serde_json::to_value(ArtifactState::ChecksumVerified).expect("serialize artifact state"),
        serde_json::json!("ChecksumVerified")
    );
    assert_eq!(
        serde_json::to_value(ResumeAction::VerifyChecksum).expect("serialize resume action"),
        serde_json::json!("VerifyChecksum")
    );
}

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
    assert_eq!(report.discovery_topology_hash, HASH);
    assert_eq!(report.pre_snapshot_topology_hash, HASH);
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

#[test]
fn state_transitions_are_monotonic() {
    let mut entry = valid_journal().artifacts.remove(0);

    let err = entry
        .advance_to(
            ArtifactState::Downloaded,
            "2026-04-10T12:01:00Z".to_string(),
        )
        .expect_err("durable cannot move back to downloaded");

    std::assert_matches!(err, JournalValidationError::InvalidStateTransition { .. });
}

#[test]
fn durable_artifact_requires_checksum() {
    let mut journal = valid_journal();
    journal.artifacts[0].checksum = None;

    let err = journal
        .validate()
        .expect_err("durable artifact without checksum should fail");

    std::assert_matches!(err, JournalValidationError::EmptyField(_));
}

#[test]
fn duplicate_artifacts_fail_validation() {
    let mut journal = valid_journal();
    journal.artifacts.push(journal.artifacts[0].clone());

    let err = journal
        .validate()
        .expect_err("duplicate artifact should fail");

    std::assert_matches!(err, JournalValidationError::DuplicateArtifact { .. });
}

#[test]
fn artifact_paths_must_stay_relative_to_backup_root() {
    for path in ["../outside", "/tmp/outside"] {
        let mut journal = valid_journal();
        journal.artifacts[0].artifact_path = path.to_string();

        let err = journal
            .validate()
            .expect_err("escaping artifact path should fail");

        std::assert_matches!(err, JournalValidationError::InvalidArtifactPath { .. });
    }
}

#[test]
fn journal_round_trips_through_json() {
    let journal = valid_journal();

    let encoded = serde_json::to_string(&journal).expect("serialize journal");
    let decoded: DownloadJournal = serde_json::from_str(&encoded).expect("deserialize journal");

    decoded.validate().expect("decoded journal should validate");
}
