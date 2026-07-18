//! Module: manifest::types
//!
//! Responsibility: define serialized backup manifest data contracts.
//! Does not own: validation, discovery, snapshot capture, or restore actions.
//! Boundary: stable JSON shapes shared by backup creation and restore flows.

use serde::{Deserialize, Serialize};

///
/// DeploymentBackupManifest
///
/// Top-level deployment backup manifest persisted with a backup bundle.
/// Owned by backup manifest contracts and consumed by restore planning.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentBackupManifest {
    pub manifest_version: u16,
    pub backup_id: String,
    pub created_at: String,
    pub tool: ToolMetadata,
    pub source: SourceMetadata,
    pub consistency: ConsistencySection,
    pub deployment: DeploymentSection,
    pub verification: VerificationPlan,
}

///
/// ToolMetadata
///
/// Tool identity recorded with one generated backup manifest.
/// Owned by backup manifest contracts and written during backup creation.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
}

///
/// SourceMetadata
///
/// Source network identity recorded for a backup bundle.
/// Owned by backup manifest contracts and used by restore validation.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMetadata {
    pub network: String,
    pub root_canister: String,
}

///
/// ConsistencySection
///
/// Backup unit grouping used to validate deployment consistency.
/// Owned by backup manifest contracts and checked before restore planning.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsistencySection {
    pub backup_units: Vec<BackupUnit>,
}

///
/// BackupUnit
///
/// Role grouping that must be captured and restored as one consistency unit.
/// Owned by backup manifest contracts and validated against deployment roles.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupUnit {
    pub unit_id: String,
    pub kind: BackupUnitKind,
    pub roles: Vec<String>,
}

///
/// BackupUnitKind
///
/// Consistency grouping mode for a backup unit.
/// Owned by backup manifest contracts and interpreted by validators.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupUnitKind {
    Single,
    Subtree,
}

///
/// DeploymentSection
///
/// Captured deployment topology and member list for one backup.
/// Owned by backup manifest contracts and consumed by restore planning.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentSection {
    pub topology_hash_algorithm: String,
    pub topology_hash_input: String,
    pub discovery_topology_hash: String,
    pub pre_snapshot_topology_hash: String,
    pub topology_hash: String,
    pub members: Vec<DeploymentMember>,
}

///
/// DeploymentMember
///
/// One canister member captured in a deployment backup manifest.
/// Owned by backup manifest contracts and mapped into restore plan members.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentMember {
    pub role: String,
    pub canister_id: String,
    pub parent_canister_id: Option<String>,
    pub subnet_canister_id: Option<String>,
    pub controller_hint: Option<String>,
    pub identity_mode: IdentityMode,
    pub verification_checks: Vec<VerificationCheck>,
    pub source_snapshot: SourceSnapshot,
}

///
/// IdentityMode
///
/// Restore identity policy for one deployment member.
/// Owned by backup manifest contracts and enforced during restore planning.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityMode {
    Fixed,
    Relocatable,
}

///
/// SourceSnapshot
///
/// Snapshot artifact metadata for one deployment member.
/// Owned by backup manifest contracts and validated before restore execution.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshot {
    pub snapshot_id: String,
    pub module_hash: Option<String>,
    pub code_version: Option<String>,
    pub artifact_path: String,
    pub checksum_algorithm: String,
    #[serde(default)]
    pub checksum: Option<String>,
}

///
/// VerificationPlan
///
/// Deployment and member verification checks required for a backup bundle.
/// Owned by backup manifest contracts and consumed by restore validation.
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlan {
    pub deployment_checks: Vec<VerificationCheck>,
    pub member_checks: Vec<MemberVerificationChecks>,
}

///
/// MemberVerificationChecks
///
/// Verification checks scoped to one deployment role.
/// Owned by backup manifest contracts and validated against deployment members.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemberVerificationChecks {
    pub role: String,
    pub checks: Vec<VerificationCheck>,
}

///
/// VerificationCheck
///
/// Named verification check and the deployment roles it covers.
/// Owned by backup manifest contracts and interpreted by validators.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationCheck {
    pub kind: String,
    pub roles: Vec<String>,
}

/// Stable manifest label for the supported canister-status verification.
pub const VERIFICATION_KIND_STATUS: &str = "status";
