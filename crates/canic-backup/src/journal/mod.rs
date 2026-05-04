use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, str::FromStr};
use thiserror::Error as ThisError;

const SUPPORTED_JOURNAL_VERSION: u16 = 1;
const SHA256_ALGORITHM: &str = "sha256";

///
/// DownloadJournal
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DownloadJournal {
    pub journal_version: u16,
    pub backup_id: String,
    #[serde(default)]
    pub discovery_topology_hash: Option<String>,
    #[serde(default)]
    pub pre_snapshot_topology_hash: Option<String>,
    pub artifacts: Vec<ArtifactJournalEntry>,
}

impl DownloadJournal {
    /// Validate resumable artifact state for one backup run.
    pub fn validate(&self) -> Result<(), JournalValidationError> {
        validate_journal_version(self.journal_version)?;
        validate_nonempty("backup_id", &self.backup_id)?;
        validate_optional_hash(
            "discovery_topology_hash",
            self.discovery_topology_hash.as_deref(),
        )?;
        validate_optional_hash(
            "pre_snapshot_topology_hash",
            self.pre_snapshot_topology_hash.as_deref(),
        )?;

        if self.artifacts.is_empty() {
            return Err(JournalValidationError::EmptyCollection("artifacts"));
        }

        let mut keys = BTreeSet::new();
        for artifact in &self.artifacts {
            artifact.validate()?;
            let key = (artifact.canister_id.clone(), artifact.snapshot_id.clone());
            if !keys.insert(key) {
                return Err(JournalValidationError::DuplicateArtifact {
                    canister_id: artifact.canister_id.clone(),
                    snapshot_id: artifact.snapshot_id.clone(),
                });
            }
        }

        Ok(())
    }

    /// Build a resumability report from the current journal state.
    #[must_use]
    pub fn resume_report(&self) -> JournalResumeReport {
        let mut counts = JournalStateCounts::default();
        let mut artifacts = Vec::with_capacity(self.artifacts.len());

        for artifact in &self.artifacts {
            counts.record(artifact.state, artifact.resume_action());
            artifacts.push(ArtifactResumeReport {
                canister_id: artifact.canister_id.clone(),
                snapshot_id: artifact.snapshot_id.clone(),
                state: artifact.state,
                resume_action: artifact.resume_action(),
                artifact_path: artifact.artifact_path.clone(),
                temp_path: artifact.temp_path.clone(),
                updated_at: artifact.updated_at.clone(),
            });
        }

        JournalResumeReport {
            backup_id: self.backup_id.clone(),
            discovery_topology_hash: self.discovery_topology_hash.clone(),
            pre_snapshot_topology_hash: self.pre_snapshot_topology_hash.clone(),
            total_artifacts: self.artifacts.len(),
            is_complete: counts.skip == self.artifacts.len(),
            pending_artifacts: self.artifacts.len() - counts.skip,
            counts,
            artifacts,
        }
    }
}

///
/// ArtifactJournalEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArtifactJournalEntry {
    pub canister_id: String,
    pub snapshot_id: String,
    pub state: ArtifactState,
    pub temp_path: Option<String>,
    pub artifact_path: String,
    pub checksum_algorithm: String,
    pub checksum: Option<String>,
    pub updated_at: String,
}

impl ArtifactJournalEntry {
    /// Return the idempotent action needed to resume this artifact.
    #[must_use]
    pub const fn resume_action(&self) -> ResumeAction {
        match self.state {
            ArtifactState::Created => ResumeAction::Download,
            ArtifactState::Downloaded => ResumeAction::VerifyChecksum,
            ArtifactState::ChecksumVerified => ResumeAction::Finalize,
            ArtifactState::Durable => ResumeAction::Skip,
        }
    }

    /// Advance this artifact to a later journal state.
    pub fn advance_to(
        &mut self,
        next_state: ArtifactState,
        updated_at: String,
    ) -> Result<(), JournalValidationError> {
        if !self.state.can_advance_to(next_state) {
            return Err(JournalValidationError::InvalidStateTransition {
                from: self.state,
                to: next_state,
            });
        }

        self.state = next_state;
        self.updated_at = updated_at;
        Ok(())
    }

    /// Validate one artifact's resumable state.
    fn validate(&self) -> Result<(), JournalValidationError> {
        validate_principal("artifacts[].canister_id", &self.canister_id)?;
        validate_nonempty("artifacts[].snapshot_id", &self.snapshot_id)?;
        validate_nonempty("artifacts[].artifact_path", &self.artifact_path)?;
        validate_nonempty("artifacts[].checksum_algorithm", &self.checksum_algorithm)?;
        validate_nonempty("artifacts[].updated_at", &self.updated_at)?;

        if self.checksum_algorithm != SHA256_ALGORITHM {
            return Err(JournalValidationError::UnsupportedHashAlgorithm(
                self.checksum_algorithm.clone(),
            ));
        }

        if matches!(
            self.state,
            ArtifactState::Downloaded | ArtifactState::ChecksumVerified
        ) {
            validate_required_option("artifacts[].temp_path", self.temp_path.as_deref())?;
        }

        if matches!(
            self.state,
            ArtifactState::ChecksumVerified | ArtifactState::Durable
        ) {
            validate_required_hash("artifacts[].checksum", self.checksum.as_deref())?;
        }

        Ok(())
    }
}

///
/// ArtifactState
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum ArtifactState {
    Created,
    Downloaded,
    ChecksumVerified,
    Durable,
}

impl ArtifactState {
    /// Return whether this state can advance monotonically to `next`.
    #[must_use]
    pub const fn can_advance_to(self, next: Self) -> bool {
        self.as_order() <= next.as_order()
    }

    /// Return the stable ordering used by the journal state machine.
    const fn as_order(self) -> u8 {
        match self {
            Self::Created => 0,
            Self::Downloaded => 1,
            Self::ChecksumVerified => 2,
            Self::Durable => 3,
        }
    }
}

///
/// ResumeAction
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum ResumeAction {
    Download,
    VerifyChecksum,
    Finalize,
    Skip,
}

///
/// JournalResumeReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JournalResumeReport {
    pub backup_id: String,
    pub discovery_topology_hash: Option<String>,
    pub pre_snapshot_topology_hash: Option<String>,
    pub total_artifacts: usize,
    pub is_complete: bool,
    pub pending_artifacts: usize,
    pub counts: JournalStateCounts,
    pub artifacts: Vec<ArtifactResumeReport>,
}

///
/// JournalStateCounts
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct JournalStateCounts {
    pub created: usize,
    pub downloaded: usize,
    pub checksum_verified: usize,
    pub durable: usize,
    pub download: usize,
    pub verify_checksum: usize,
    pub finalize: usize,
    pub skip: usize,
}

impl JournalStateCounts {
    // Record one artifact's state and next idempotent resume action.
    const fn record(&mut self, state: ArtifactState, action: ResumeAction) {
        match state {
            ArtifactState::Created => self.created += 1,
            ArtifactState::Downloaded => self.downloaded += 1,
            ArtifactState::ChecksumVerified => self.checksum_verified += 1,
            ArtifactState::Durable => self.durable += 1,
        }

        match action {
            ResumeAction::Download => self.download += 1,
            ResumeAction::VerifyChecksum => self.verify_checksum += 1,
            ResumeAction::Finalize => self.finalize += 1,
            ResumeAction::Skip => self.skip += 1,
        }
    }
}

///
/// ArtifactResumeReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactResumeReport {
    pub canister_id: String,
    pub snapshot_id: String,
    pub state: ArtifactState,
    pub resume_action: ResumeAction,
    pub artifact_path: String,
    pub temp_path: Option<String>,
    pub updated_at: String,
}

///
/// JournalValidationError
///

#[derive(Debug, ThisError)]
pub enum JournalValidationError {
    #[error("unsupported journal version {0}")]
    UnsupportedJournalVersion(u16),

    #[error("field {0} must not be empty")]
    EmptyField(&'static str),

    #[error("collection {0} must not be empty")]
    EmptyCollection(&'static str),

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("field {0} must be a non-empty sha256 hex string")]
    InvalidHash(&'static str),

    #[error("unsupported hash algorithm {0}")]
    UnsupportedHashAlgorithm(String),

    #[error("duplicate artifact entry for canister {canister_id} snapshot {snapshot_id}")]
    DuplicateArtifact {
        canister_id: String,
        snapshot_id: String,
    },

    #[error("invalid journal transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: ArtifactState,
        to: ArtifactState,
    },
}

// Validate the journal format version before checking nested entries.
const fn validate_journal_version(version: u16) -> Result<(), JournalValidationError> {
    if version == SUPPORTED_JOURNAL_VERSION {
        Ok(())
    } else {
        Err(JournalValidationError::UnsupportedJournalVersion(version))
    }
}

// Validate required string fields after trimming whitespace.
fn validate_nonempty(field: &'static str, value: &str) -> Result<(), JournalValidationError> {
    if value.trim().is_empty() {
        Err(JournalValidationError::EmptyField(field))
    } else {
        Ok(())
    }
}

// Validate required string fields represented as optional journal fields.
fn validate_required_option(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), JournalValidationError> {
    match value {
        Some(value) => validate_nonempty(field, value),
        None => Err(JournalValidationError::EmptyField(field)),
    }
}

// Validate textual principal fields used in JSON journals.
fn validate_principal(field: &'static str, value: &str) -> Result<(), JournalValidationError> {
    validate_nonempty(field, value)?;
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| JournalValidationError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

// Validate required SHA-256 hex fields represented as optional journal fields.
fn validate_required_hash(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), JournalValidationError> {
    match value {
        Some(value) => validate_hash(field, value),
        None => Err(JournalValidationError::EmptyField(field)),
    }
}

// Validate optional SHA-256 hex fields when present.
fn validate_optional_hash(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), JournalValidationError> {
    if let Some(value) = value {
        validate_hash(field, value)?;
    }
    Ok(())
}

// Validate SHA-256 hex values used for downloaded artifacts.
fn validate_hash(field: &'static str, value: &str) -> Result<(), JournalValidationError> {
    const SHA256_HEX_LEN: usize = 64;
    validate_nonempty(field, value)?;
    if value.len() == SHA256_HEX_LEN && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(JournalValidationError::InvalidHash(field))
    }
}

#[cfg(test)]
mod tests {
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
}
