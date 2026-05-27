use serde::{Deserialize, Serialize};

///
/// DeploymentBackupManifest
///

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
}

///
/// SourceMetadata
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceMetadata {
    pub environment: String,
    pub root_canister: String,
}

///
/// ConsistencySection
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConsistencySection {
    pub backup_units: Vec<BackupUnit>,
}

///
/// BackupUnit
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BackupUnit {
    pub unit_id: String,
    pub kind: BackupUnitKind,
    pub roles: Vec<String>,
}

///
/// BackupUnitKind
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

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityMode {
    Fixed,
    Relocatable,
}

///
/// SourceSnapshot
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct VerificationPlan {
    pub deployment_checks: Vec<VerificationCheck>,
    pub member_checks: Vec<MemberVerificationChecks>,
}

///
/// MemberVerificationChecks
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemberVerificationChecks {
    pub role: String,
    pub checks: Vec<VerificationCheck>,
}

///
/// VerificationCheck
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerificationCheck {
    pub kind: String,
    pub roles: Vec<String>,
}
