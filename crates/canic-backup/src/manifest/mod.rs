use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, str::FromStr};
use thiserror::Error as ThisError;

const SUPPORTED_MANIFEST_VERSION: u16 = 1;
const SHA256_ALGORITHM: &str = "sha256";

///
/// FleetBackupManifest
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FleetBackupManifest {
    pub manifest_version: u16,
    pub backup_id: String,
    pub created_at: String,
    pub tool: ToolMetadata,
    pub source: SourceMetadata,
    pub consistency: ConsistencySection,
    pub fleet: FleetSection,
    pub verification: VerificationPlan,
}

impl FleetBackupManifest {
    /// Validate the manifest-level contract before backup finalization or restore planning.
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_manifest_version(self.manifest_version)?;
        validate_nonempty("backup_id", &self.backup_id)?;
        validate_nonempty("created_at", &self.created_at)?;
        self.tool.validate()?;
        self.source.validate()?;
        self.consistency.validate()?;
        self.fleet.validate()?;
        self.verification.validate()?;
        Ok(())
    }
}

///
/// ToolMetadata
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
}

impl ToolMetadata {
    /// Validate that the manifest names the tool that produced it.
    pub(crate) fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("tool.name", &self.name)?;
        validate_nonempty("tool.version", &self.version)
    }
}

///
/// SourceMetadata
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceMetadata {
    pub environment: String,
    pub root_canister: String,
}

impl SourceMetadata {
    /// Validate the source environment and root canister identity.
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("source.environment", &self.environment)?;
        validate_principal("source.root_canister", &self.root_canister)
    }
}

///
/// ConsistencySection
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConsistencySection {
    pub mode: ConsistencyMode,
    pub backup_units: Vec<BackupUnit>,
}

impl ConsistencySection {
    /// Validate consistency mode and every declared backup unit.
    fn validate(&self) -> Result<(), ManifestValidationError> {
        if self.backup_units.is_empty() {
            return Err(ManifestValidationError::EmptyCollection(
                "consistency.backup_units",
            ));
        }

        for unit in &self.backup_units {
            unit.validate(&self.mode)?;
        }

        Ok(())
    }
}

///
/// ConsistencyMode
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConsistencyMode {
    CrashConsistent,
    QuiescedUnit,
}

///
/// BackupUnit
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BackupUnit {
    pub unit_id: String,
    pub kind: BackupUnitKind,
    pub roles: Vec<String>,
    pub consistency_reason: Option<String>,
    pub dependency_closure: Vec<String>,
    pub topology_validation: String,
    pub quiescence_strategy: Option<String>,
}

impl BackupUnit {
    /// Validate the declared unit boundary and quiescence metadata.
    fn validate(&self, mode: &ConsistencyMode) -> Result<(), ManifestValidationError> {
        validate_nonempty("consistency.backup_units[].unit_id", &self.unit_id)?;
        validate_nonempty(
            "consistency.backup_units[].topology_validation",
            &self.topology_validation,
        )?;

        if self.roles.is_empty() {
            return Err(ManifestValidationError::EmptyCollection(
                "consistency.backup_units[].roles",
            ));
        }

        for role in &self.roles {
            validate_nonempty("consistency.backup_units[].roles[]", role)?;
        }

        if matches!(self.kind, BackupUnitKind::Flat) {
            validate_required_option(
                "consistency.backup_units[].consistency_reason",
                self.consistency_reason.as_deref(),
            )?;
        }

        if matches!(mode, ConsistencyMode::QuiescedUnit) {
            validate_required_option(
                "consistency.backup_units[].quiescence_strategy",
                self.quiescence_strategy.as_deref(),
            )?;
        }

        Ok(())
    }
}

///
/// BackupUnitKind
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupUnitKind {
    WholeFleet,
    ControlPlaneSubset,
    SubtreeRooted,
    Flat,
}

///
/// FleetSection
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FleetSection {
    pub topology_hash_algorithm: String,
    pub topology_hash_input: String,
    pub discovery_topology_hash: String,
    pub pre_snapshot_topology_hash: String,
    pub topology_hash: String,
    pub members: Vec<FleetMember>,
}

impl FleetSection {
    /// Validate topology hash invariants and member uniqueness.
    pub(crate) fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty(
            "fleet.topology_hash_algorithm",
            &self.topology_hash_algorithm,
        )?;
        if self.topology_hash_algorithm != SHA256_ALGORITHM {
            return Err(ManifestValidationError::UnsupportedHashAlgorithm(
                self.topology_hash_algorithm.clone(),
            ));
        }

        validate_nonempty("fleet.topology_hash_input", &self.topology_hash_input)?;
        validate_hash(
            "fleet.discovery_topology_hash",
            &self.discovery_topology_hash,
        )?;
        validate_hash(
            "fleet.pre_snapshot_topology_hash",
            &self.pre_snapshot_topology_hash,
        )?;
        validate_hash("fleet.topology_hash", &self.topology_hash)?;

        if self.discovery_topology_hash != self.pre_snapshot_topology_hash {
            return Err(ManifestValidationError::TopologyHashMismatch {
                discovery: self.discovery_topology_hash.clone(),
                pre_snapshot: self.pre_snapshot_topology_hash.clone(),
            });
        }

        if self.topology_hash != self.discovery_topology_hash {
            return Err(ManifestValidationError::AcceptedTopologyHashMismatch {
                accepted: self.topology_hash.clone(),
                discovery: self.discovery_topology_hash.clone(),
            });
        }

        if self.members.is_empty() {
            return Err(ManifestValidationError::EmptyCollection("fleet.members"));
        }

        let mut canister_ids = BTreeSet::new();
        for member in &self.members {
            member.validate()?;
            if !canister_ids.insert(member.canister_id.clone()) {
                return Err(ManifestValidationError::DuplicateCanisterId(
                    member.canister_id.clone(),
                ));
            }
        }

        Ok(())
    }
}

///
/// FleetMember
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FleetMember {
    pub role: String,
    pub canister_id: String,
    pub parent_canister_id: Option<String>,
    pub subnet_canister_id: Option<String>,
    pub controller_hint: Option<String>,
    pub identity_mode: IdentityMode,
    pub restore_group: u16,
    pub verification_class: String,
    pub verification_checks: Vec<VerificationCheck>,
    pub source_snapshot: SourceSnapshot,
}

impl FleetMember {
    /// Validate one restore member projection from the manifest.
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("fleet.members[].role", &self.role)?;
        validate_principal("fleet.members[].canister_id", &self.canister_id)?;
        validate_optional_principal(
            "fleet.members[].parent_canister_id",
            self.parent_canister_id.as_deref(),
        )?;
        validate_optional_principal(
            "fleet.members[].subnet_canister_id",
            self.subnet_canister_id.as_deref(),
        )?;
        validate_optional_principal(
            "fleet.members[].controller_hint",
            self.controller_hint.as_deref(),
        )?;
        validate_nonempty(
            "fleet.members[].verification_class",
            &self.verification_class,
        )?;

        if self.verification_checks.is_empty() {
            return Err(ManifestValidationError::MissingMemberVerificationChecks(
                self.canister_id.clone(),
            ));
        }

        for check in &self.verification_checks {
            check.validate()?;
        }

        self.source_snapshot.validate()
    }
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceSnapshot {
    pub snapshot_id: String,
    pub module_hash: Option<String>,
    pub wasm_hash: Option<String>,
    pub code_version: Option<String>,
    pub artifact_path: String,
    pub checksum_algorithm: String,
}

impl SourceSnapshot {
    /// Validate source snapshot provenance and artifact checksum metadata.
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty(
            "fleet.members[].source_snapshot.snapshot_id",
            &self.snapshot_id,
        )?;
        validate_optional_nonempty(
            "fleet.members[].source_snapshot.module_hash",
            self.module_hash.as_deref(),
        )?;
        validate_optional_nonempty(
            "fleet.members[].source_snapshot.wasm_hash",
            self.wasm_hash.as_deref(),
        )?;
        validate_optional_nonempty(
            "fleet.members[].source_snapshot.code_version",
            self.code_version.as_deref(),
        )?;
        validate_nonempty(
            "fleet.members[].source_snapshot.artifact_path",
            &self.artifact_path,
        )?;
        validate_nonempty(
            "fleet.members[].source_snapshot.checksum_algorithm",
            &self.checksum_algorithm,
        )?;
        if self.checksum_algorithm != SHA256_ALGORITHM {
            return Err(ManifestValidationError::UnsupportedHashAlgorithm(
                self.checksum_algorithm.clone(),
            ));
        }
        Ok(())
    }
}

///
/// VerificationPlan
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct VerificationPlan {
    pub fleet_checks: Vec<VerificationCheck>,
    pub member_checks: Vec<MemberVerificationChecks>,
}

impl VerificationPlan {
    /// Validate all declarative verification checks.
    fn validate(&self) -> Result<(), ManifestValidationError> {
        for check in &self.fleet_checks {
            check.validate()?;
        }
        for member in &self.member_checks {
            member.validate()?;
        }
        Ok(())
    }
}

///
/// MemberVerificationChecks
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemberVerificationChecks {
    pub role: String,
    pub checks: Vec<VerificationCheck>,
}

impl MemberVerificationChecks {
    /// Validate one role-scoped verification check group.
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("verification.member_checks[].role", &self.role)?;
        if self.checks.is_empty() {
            return Err(ManifestValidationError::EmptyCollection(
                "verification.member_checks[].checks",
            ));
        }
        for check in &self.checks {
            check.validate()?;
        }
        Ok(())
    }
}

///
/// VerificationCheck
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VerificationCheck {
    pub kind: String,
    pub method: Option<String>,
    pub roles: Vec<String>,
}

impl VerificationCheck {
    /// Validate one concrete verification check.
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("verification.check.kind", &self.kind)?;
        validate_optional_nonempty("verification.check.method", self.method.as_deref())?;
        for role in &self.roles {
            validate_nonempty("verification.check.roles[]", role)?;
        }
        Ok(())
    }
}

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

    #[error("topology hash mismatch between discovery {discovery} and pre-snapshot {pre_snapshot}")]
    TopologyHashMismatch {
        discovery: String,
        pre_snapshot: String,
    },

    #[error("accepted topology hash {accepted} does not match discovery hash {discovery}")]
    AcceptedTopologyHashMismatch { accepted: String, discovery: String },

    #[error("duplicate canister id {0}")]
    DuplicateCanisterId(String),

    #[error("fleet member {0} has no concrete verification checks")]
    MissingMemberVerificationChecks(String),
}

// Validate the manifest format version before checking nested fields.
const fn validate_manifest_version(version: u16) -> Result<(), ManifestValidationError> {
    if version == SUPPORTED_MANIFEST_VERSION {
        Ok(())
    } else {
        Err(ManifestValidationError::UnsupportedManifestVersion(version))
    }
}

// Validate required string fields after trimming whitespace.
fn validate_nonempty(field: &'static str, value: &str) -> Result<(), ManifestValidationError> {
    if value.trim().is_empty() {
        Err(ManifestValidationError::EmptyField(field))
    } else {
        Ok(())
    }
}

// Validate optional string fields only when present.
fn validate_optional_nonempty(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ManifestValidationError> {
    if let Some(value) = value {
        validate_nonempty(field, value)?;
    }
    Ok(())
}

// Validate required string fields that are represented as optional manifest fields.
fn validate_required_option(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ManifestValidationError> {
    match value {
        Some(value) => validate_nonempty(field, value),
        None => Err(ManifestValidationError::EmptyField(field)),
    }
}

// Validate textual principal fields used in JSON manifests.
fn validate_principal(field: &'static str, value: &str) -> Result<(), ManifestValidationError> {
    validate_nonempty(field, value)?;
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| ManifestValidationError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

// Validate optional textual principal fields used in JSON manifests.
fn validate_optional_principal(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ManifestValidationError> {
    if let Some(value) = value {
        validate_principal(field, value)?;
    }
    Ok(())
}

// Validate SHA-256 hex values used for topology and artifact compatibility.
fn validate_hash(field: &'static str, value: &str) -> Result<(), ManifestValidationError> {
    const SHA256_HEX_LEN: usize = 64;
    validate_nonempty(field, value)?;
    if value.len() == SHA256_HEX_LEN && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ManifestValidationError::InvalidHash(field))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Build one valid manifest for validation tests.
    fn valid_manifest() -> FleetBackupManifest {
        FleetBackupManifest {
            manifest_version: 1,
            backup_id: "fbk_test_001".to_string(),
            created_at: "2026-04-10T12:00:00Z".to_string(),
            tool: ToolMetadata {
                name: "canic".to_string(),
                version: "v1".to_string(),
            },
            source: SourceMetadata {
                environment: "local".to_string(),
                root_canister: ROOT.to_string(),
            },
            consistency: ConsistencySection {
                mode: ConsistencyMode::QuiescedUnit,
                backup_units: vec![BackupUnit {
                    unit_id: "core".to_string(),
                    kind: BackupUnitKind::Flat,
                    roles: vec!["root".to_string(), "app".to_string()],
                    consistency_reason: Some("root and app state are coordinated".to_string()),
                    dependency_closure: vec!["root".to_string(), "app".to_string()],
                    topology_validation: "operator-declared-flat".to_string(),
                    quiescence_strategy: Some("standard-canic-hooks@v1".to_string()),
                }],
            },
            fleet: FleetSection {
                topology_hash_algorithm: "sha256".to_string(),
                topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
                discovery_topology_hash: HASH.to_string(),
                pre_snapshot_topology_hash: HASH.to_string(),
                topology_hash: HASH.to_string(),
                members: vec![FleetMember {
                    role: "root".to_string(),
                    canister_id: ROOT.to_string(),
                    parent_canister_id: None,
                    subnet_canister_id: Some(CHILD.to_string()),
                    controller_hint: Some(ROOT.to_string()),
                    identity_mode: IdentityMode::Fixed,
                    restore_group: 1,
                    verification_class: "basic".to_string(),
                    verification_checks: vec![VerificationCheck {
                        kind: "call".to_string(),
                        method: Some("canic_ready".to_string()),
                        roles: Vec::new(),
                    }],
                    source_snapshot: SourceSnapshot {
                        snapshot_id: "snap-1".to_string(),
                        module_hash: Some(HASH.to_string()),
                        wasm_hash: Some(HASH.to_string()),
                        code_version: Some("v0.30.0".to_string()),
                        artifact_path: "artifacts/root".to_string(),
                        checksum_algorithm: "sha256".to_string(),
                    },
                }],
            },
            verification: VerificationPlan {
                fleet_checks: vec![VerificationCheck {
                    kind: "root_ready".to_string(),
                    method: None,
                    roles: Vec::new(),
                }],
                member_checks: Vec::new(),
            },
        }
    }

    #[test]
    fn valid_manifest_passes_validation() {
        let manifest = valid_manifest();

        manifest.validate().expect("manifest should validate");
    }

    #[test]
    fn topology_hash_mismatch_fails_validation() {
        let mut manifest = valid_manifest();
        manifest.fleet.pre_snapshot_topology_hash =
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();

        let err = manifest.validate().expect_err("mismatch should fail");

        assert!(matches!(
            err,
            ManifestValidationError::TopologyHashMismatch { .. }
        ));
    }

    #[test]
    fn missing_member_verification_checks_fail_validation() {
        let mut manifest = valid_manifest();
        manifest.fleet.members[0].verification_checks.clear();

        let err = manifest
            .validate()
            .expect_err("missing member checks should fail");

        assert!(matches!(
            err,
            ManifestValidationError::MissingMemberVerificationChecks(_)
        ));
    }

    #[test]
    fn quiesced_unit_requires_quiescence_strategy() {
        let mut manifest = valid_manifest();
        manifest.consistency.backup_units[0].quiescence_strategy = None;

        let err = manifest
            .validate()
            .expect_err("missing quiescence strategy should fail");

        assert!(matches!(err, ManifestValidationError::EmptyField(_)));
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let manifest = valid_manifest();

        let encoded = serde_json::to_string(&manifest).expect("serialize manifest");
        let decoded: FleetBackupManifest =
            serde_json::from_str(&encoded).expect("deserialize manifest");

        decoded
            .validate()
            .expect("decoded manifest should validate");
    }
}
