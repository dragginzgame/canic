use thiserror::Error as ThisError;

///
/// ManifestValidationError
///

#[derive(Debug, ThisError)]
pub enum ManifestValidationError {
    #[error("unsupported manifest version {0}")]
    UnsupportedManifestVersion(u16),

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

    #[error("unsupported verification kind {0}")]
    UnsupportedVerificationKind(String),

    #[error("topology hash mismatch between discovery {discovery} and pre-snapshot {pre_snapshot}")]
    TopologyHashMismatch {
        discovery: String,
        pre_snapshot: String,
    },

    #[error("accepted topology hash {accepted} does not match discovery hash {discovery}")]
    AcceptedTopologyHashMismatch { accepted: String, discovery: String },

    #[error("duplicate canister id {0}")]
    DuplicateCanisterId(String),

    #[error("duplicate backup unit id {0}")]
    DuplicateBackupUnitId(String),

    #[error("backup unit {unit_id} repeats role {role}")]
    DuplicateBackupUnitRole { unit_id: String, role: String },

    #[error("fleet member {0} has no concrete verification checks")]
    MissingMemberVerificationChecks(String),

    #[error("backup unit {unit_id} references unknown role {role}")]
    UnknownBackupUnitRole { unit_id: String, role: String },

    #[error("fleet role {role} is not covered by any backup unit")]
    BackupUnitCoverageMissingRole { role: String },

    #[error("verification plan references unknown role {role}")]
    UnknownVerificationRole { role: String },

    #[error("duplicate member verification role {0}")]
    DuplicateMemberVerificationRole(String),

    #[error("verification check {kind} repeats role {role}")]
    DuplicateVerificationCheckRole { kind: String, role: String },

    #[error("subtree backup unit {unit_id} is not connected")]
    SubtreeBackupUnitNotConnected { unit_id: String },

    #[error(
        "subtree backup unit {unit_id} includes parent {parent} but omits descendant {descendant}"
    )]
    SubtreeBackupUnitMissingDescendant {
        unit_id: String,
        parent: String,
        descendant: String,
    },
}
