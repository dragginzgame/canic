use candid::Principal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
use thiserror::Error as ThisError;

const SUPPORTED_MANIFEST_VERSION: u16 = 1;
const SHA256_ALGORITHM: &str = "sha256";
const DESIGN_V1: &str = "design";
const TOPOLOGY_HASH_INPUT_V1: &str = "sorted(pid,parent_pid,role,module_hash)";

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
        validate_consistency_against_fleet(&self.consistency, &self.fleet)?;
        validate_verification_against_fleet(&self.verification, &self.fleet)?;
        Ok(())
    }

    /// Build a design-conformance report for operator preflight checks.
    #[must_use]
    pub fn design_conformance_report(&self) -> ManifestDesignConformanceReport {
        ManifestDesignConformanceReport::from_manifest(self)
    }
}

///
/// ManifestDesignConformanceReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManifestDesignConformanceReport {
    pub design_version: String,
    pub design_v1_ready: bool,
    pub topology: TopologyConformance,
    pub backup_units: BackupUnitConformance,
    pub quiescence: QuiescenceConformance,
    pub verification: VerificationConformance,
    pub identity: IdentityConformance,
    pub snapshot_provenance: SnapshotProvenanceConformance,
    pub restore_order: RestoreOrderConformance,
}

impl ManifestDesignConformanceReport {
    /// Build one report from an already-loaded manifest.
    fn from_manifest(manifest: &FleetBackupManifest) -> Self {
        let topology = TopologyConformance::from_manifest(manifest);
        let backup_units = BackupUnitConformance::from_manifest(manifest);
        let quiescence = QuiescenceConformance::from_manifest(manifest);
        let verification = VerificationConformance::from_manifest(manifest);
        let identity = IdentityConformance::from_manifest(manifest);
        let snapshot_provenance = SnapshotProvenanceConformance::from_manifest(manifest);
        let restore_order = RestoreOrderConformance::from_manifest(manifest);
        let design_v1_ready = topology.design_v1_ready
            && backup_units.design_v1_ready
            && quiescence.design_v1_ready
            && verification.design_v1_ready
            && snapshot_provenance.design_v1_ready
            && restore_order.design_v1_ready;

        Self {
            design_version: DESIGN_V1.to_string(),
            design_v1_ready,
            topology,
            backup_units,
            quiescence,
            verification,
            identity,
            snapshot_provenance,
            restore_order,
        }
    }
}

/// Build the manifest validation summary emitted by CLI and preflight workflows.
#[must_use]
pub fn manifest_validation_summary(manifest: &FleetBackupManifest) -> serde_json::Value {
    let design_conformance = manifest.design_conformance_report();

    json!({
        "status": "valid",
        "backup_id": manifest.backup_id,
        "members": manifest.fleet.members.len(),
        "backup_unit_count": manifest.consistency.backup_units.len(),
        "consistency_mode": consistency_mode_name(&manifest.consistency.mode),
        "topology_hash": manifest.fleet.topology_hash,
        "topology_hash_algorithm": manifest.fleet.topology_hash_algorithm,
        "topology_hash_input": manifest.fleet.topology_hash_input,
        "topology_validation_status": "validated",
        "design_conformance": design_conformance,
        "backup_unit_kinds": backup_unit_kind_counts(manifest),
        "backup_units": manifest
            .consistency
            .backup_units
            .iter()
            .map(|unit| json!({
                "unit_id": unit.unit_id,
                "kind": backup_unit_kind_name(&unit.kind),
                "role_count": unit.roles.len(),
                "dependency_count": unit.dependency_closure.len(),
                "topology_validation": unit.topology_validation,
            }))
            .collect::<Vec<_>>(),
    })
}

// Count backup units by stable serialized kind name.
fn backup_unit_kind_counts(manifest: &FleetBackupManifest) -> serde_json::Value {
    let mut whole_fleet = 0;
    let mut control_plane_subset = 0;
    let mut subtree_rooted = 0;
    let mut flat = 0;
    for unit in &manifest.consistency.backup_units {
        match &unit.kind {
            BackupUnitKind::WholeFleet => whole_fleet += 1,
            BackupUnitKind::ControlPlaneSubset => control_plane_subset += 1,
            BackupUnitKind::SubtreeRooted => subtree_rooted += 1,
            BackupUnitKind::Flat => flat += 1,
        }
    }

    json!({
        "whole_fleet": whole_fleet,
        "control_plane_subset": control_plane_subset,
        "subtree_rooted": subtree_rooted,
        "flat": flat,
    })
}

// Return the stable serialized name for a consistency mode.
pub(crate) const fn consistency_mode_name(mode: &ConsistencyMode) -> &'static str {
    match mode {
        ConsistencyMode::CrashConsistent => "crash-consistent",
        ConsistencyMode::QuiescedUnit => "quiesced-unit",
    }
}

// Return the stable serialized name for a backup unit kind.
pub(crate) const fn backup_unit_kind_name(kind: &BackupUnitKind) -> &'static str {
    match kind {
        BackupUnitKind::WholeFleet => "whole-fleet",
        BackupUnitKind::ControlPlaneSubset => "control-plane-subset",
        BackupUnitKind::SubtreeRooted => "subtree-rooted",
        BackupUnitKind::Flat => "flat",
    }
}

///
/// TopologyConformance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct TopologyConformance {
    pub design_v1_ready: bool,
    pub algorithm_sha256: bool,
    pub canonical_input: bool,
    pub discovery_matches_pre_snapshot: bool,
    pub accepted_matches_discovery: bool,
}

impl TopologyConformance {
    /// Summarize topology hash stability and canonical input metadata.
    fn from_manifest(manifest: &FleetBackupManifest) -> Self {
        let algorithm_sha256 = manifest.fleet.topology_hash_algorithm == SHA256_ALGORITHM;
        let canonical_input = manifest.fleet.topology_hash_input == TOPOLOGY_HASH_INPUT_V1;
        let discovery_matches_pre_snapshot =
            manifest.fleet.discovery_topology_hash == manifest.fleet.pre_snapshot_topology_hash;
        let accepted_matches_discovery =
            manifest.fleet.topology_hash == manifest.fleet.discovery_topology_hash;
        let design_v1_ready = algorithm_sha256
            && canonical_input
            && discovery_matches_pre_snapshot
            && accepted_matches_discovery;

        Self {
            design_v1_ready,
            algorithm_sha256,
            canonical_input,
            discovery_matches_pre_snapshot,
            accepted_matches_discovery,
        }
    }
}

///
/// BackupUnitConformance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct BackupUnitConformance {
    pub design_v1_ready: bool,
    pub unit_count: usize,
    pub all_units_have_roles: bool,
    pub all_units_have_topology_validation: bool,
    pub all_roles_covered: bool,
    pub flat_units: usize,
    pub flat_units_with_reason: usize,
    pub subtree_units: usize,
    pub subtree_units_declared_closed: usize,
}

impl BackupUnitConformance {
    /// Summarize backup-unit boundary metadata required by the design.
    fn from_manifest(manifest: &FleetBackupManifest) -> Self {
        let unit_count = manifest.consistency.backup_units.len();
        let all_units_have_roles = manifest
            .consistency
            .backup_units
            .iter()
            .all(|unit| !unit.roles.is_empty());
        let all_units_have_topology_validation = manifest
            .consistency
            .backup_units
            .iter()
            .all(|unit| !unit.topology_validation.trim().is_empty());
        let all_roles_covered = all_fleet_roles_covered(manifest);
        let flat_units = manifest
            .consistency
            .backup_units
            .iter()
            .filter(|unit| matches!(unit.kind, BackupUnitKind::Flat))
            .count();
        let flat_units_with_reason = manifest
            .consistency
            .backup_units
            .iter()
            .filter(|unit| matches!(unit.kind, BackupUnitKind::Flat))
            .filter(|unit| {
                unit.consistency_reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty())
            })
            .count();
        let subtree_units = manifest
            .consistency
            .backup_units
            .iter()
            .filter(|unit| matches!(unit.kind, BackupUnitKind::SubtreeRooted))
            .count();
        let subtree_units_declared_closed = manifest
            .consistency
            .backup_units
            .iter()
            .filter(|unit| matches!(unit.kind, BackupUnitKind::SubtreeRooted))
            .filter(|unit| unit.topology_validation == "subtree-closed")
            .count();
        let design_v1_ready = unit_count > 0
            && all_units_have_roles
            && all_units_have_topology_validation
            && all_roles_covered
            && flat_units == flat_units_with_reason
            && subtree_units == subtree_units_declared_closed;

        Self {
            design_v1_ready,
            unit_count,
            all_units_have_roles,
            all_units_have_topology_validation,
            all_roles_covered,
            flat_units,
            flat_units_with_reason,
            subtree_units,
            subtree_units_declared_closed,
        }
    }
}

///
/// QuiescenceConformance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct QuiescenceConformance {
    pub design_v1_ready: bool,
    pub mode: ConsistencyMode,
    pub quiescence_required: bool,
    pub unit_count: usize,
    pub units_with_strategy: usize,
    pub all_required_units_have_strategy: bool,
}

impl QuiescenceConformance {
    /// Summarize the explicit quiescence boundary for the backup.
    fn from_manifest(manifest: &FleetBackupManifest) -> Self {
        let quiescence_required =
            matches!(manifest.consistency.mode, ConsistencyMode::QuiescedUnit);
        let unit_count = manifest.consistency.backup_units.len();
        let units_with_strategy = manifest
            .consistency
            .backup_units
            .iter()
            .filter(|unit| {
                unit.quiescence_strategy
                    .as_deref()
                    .is_some_and(|strategy| !strategy.trim().is_empty())
            })
            .count();
        let all_required_units_have_strategy =
            !quiescence_required || units_with_strategy == unit_count;
        let design_v1_ready = all_required_units_have_strategy;

        Self {
            design_v1_ready,
            mode: manifest.consistency.mode.clone(),
            quiescence_required,
            unit_count,
            units_with_strategy,
            all_required_units_have_strategy,
        }
    }
}

///
/// VerificationConformance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerificationConformance {
    pub design_v1_ready: bool,
    pub member_count: usize,
    pub members_with_checks: usize,
    pub all_members_have_checks: bool,
    pub fleet_check_count: usize,
    pub role_check_group_count: usize,
}

impl VerificationConformance {
    /// Summarize concrete verification coverage for restored members.
    fn from_manifest(manifest: &FleetBackupManifest) -> Self {
        let member_count = manifest.fleet.members.len();
        let members_with_checks = manifest
            .fleet
            .members
            .iter()
            .filter(|member| !member.verification_checks.is_empty())
            .count();
        let all_members_have_checks = member_count == members_with_checks;
        let fleet_check_count = manifest.verification.fleet_checks.len();
        let role_check_group_count = manifest.verification.member_checks.len();
        let design_v1_ready = member_count > 0 && all_members_have_checks;

        Self {
            design_v1_ready,
            member_count,
            members_with_checks,
            all_members_have_checks,
            fleet_check_count,
            role_check_group_count,
        }
    }
}

///
/// IdentityConformance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IdentityConformance {
    pub fixed_members: usize,
    pub relocatable_members: usize,
}

impl IdentityConformance {
    /// Summarize restore identity modes carried by fleet members.
    fn from_manifest(manifest: &FleetBackupManifest) -> Self {
        let fixed_members = manifest
            .fleet
            .members
            .iter()
            .filter(|member| matches!(member.identity_mode, IdentityMode::Fixed))
            .count();
        let relocatable_members = manifest
            .fleet
            .members
            .iter()
            .filter(|member| matches!(member.identity_mode, IdentityMode::Relocatable))
            .count();

        Self {
            fixed_members,
            relocatable_members,
        }
    }
}

///
/// SnapshotProvenanceConformance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SnapshotProvenanceConformance {
    pub design_v1_ready: bool,
    pub member_count: usize,
    pub members_with_snapshot_id: usize,
    pub members_with_checksum: usize,
    pub members_with_module_hash: usize,
    pub members_with_wasm_hash: usize,
    pub members_with_code_version: usize,
    pub all_members_have_snapshot_id: bool,
    pub all_members_have_checksum: bool,
}

impl SnapshotProvenanceConformance {
    /// Summarize snapshot artifact provenance completeness.
    fn from_manifest(manifest: &FleetBackupManifest) -> Self {
        let member_count = manifest.fleet.members.len();
        let members_with_snapshot_id = manifest
            .fleet
            .members
            .iter()
            .filter(|member| !member.source_snapshot.snapshot_id.trim().is_empty())
            .count();
        let members_with_checksum = manifest
            .fleet
            .members
            .iter()
            .filter(|member| member.source_snapshot.checksum.is_some())
            .count();
        let members_with_module_hash = manifest
            .fleet
            .members
            .iter()
            .filter(|member| member.source_snapshot.module_hash.is_some())
            .count();
        let members_with_wasm_hash = manifest
            .fleet
            .members
            .iter()
            .filter(|member| member.source_snapshot.wasm_hash.is_some())
            .count();
        let members_with_code_version = manifest
            .fleet
            .members
            .iter()
            .filter(|member| member.source_snapshot.code_version.is_some())
            .count();
        let all_members_have_snapshot_id = member_count == members_with_snapshot_id;
        let all_members_have_checksum = member_count == members_with_checksum;
        let design_v1_ready =
            member_count > 0 && all_members_have_snapshot_id && all_members_have_checksum;

        Self {
            design_v1_ready,
            member_count,
            members_with_snapshot_id,
            members_with_checksum,
            members_with_module_hash,
            members_with_wasm_hash,
            members_with_code_version,
            all_members_have_snapshot_id,
            all_members_have_checksum,
        }
    }
}

///
/// RestoreOrderConformance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreOrderConformance {
    pub design_v1_ready: bool,
    pub parent_relationships: usize,
    pub parent_group_violations: Vec<RestoreGroupViolation>,
}

impl RestoreOrderConformance {
    /// Summarize whether restore groups preserve parent-before-child ordering.
    fn from_manifest(manifest: &FleetBackupManifest) -> Self {
        let members_by_id = manifest
            .fleet
            .members
            .iter()
            .map(|member| (member.canister_id.as_str(), member))
            .collect::<BTreeMap<_, _>>();
        let mut parent_relationships = 0;
        let mut parent_group_violations = Vec::new();

        for member in &manifest.fleet.members {
            let Some(parent_id) = member.parent_canister_id.as_deref() else {
                continue;
            };
            let Some(parent) = members_by_id.get(parent_id) else {
                continue;
            };
            parent_relationships += 1;
            if parent.restore_group > member.restore_group {
                parent_group_violations.push(RestoreGroupViolation {
                    parent_canister_id: parent.canister_id.clone(),
                    child_canister_id: member.canister_id.clone(),
                    parent_restore_group: parent.restore_group,
                    child_restore_group: member.restore_group,
                });
            }
        }

        let design_v1_ready = parent_group_violations.is_empty();

        Self {
            design_v1_ready,
            parent_relationships,
            parent_group_violations,
        }
    }
}

///
/// RestoreGroupViolation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreGroupViolation {
    pub parent_canister_id: String,
    pub child_canister_id: String,
    pub parent_restore_group: u16,
    pub child_restore_group: u16,
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

        let mut unit_ids = BTreeSet::new();
        for unit in &self.backup_units {
            unit.validate(&self.mode)?;
            if !unit_ids.insert(unit.unit_id.clone()) {
                return Err(ManifestValidationError::DuplicateBackupUnitId(
                    unit.unit_id.clone(),
                ));
            }
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
        validate_unique_values("consistency.backup_units[].roles[]", &self.roles, |role| {
            ManifestValidationError::DuplicateBackupUnitRole {
                unit_id: self.unit_id.clone(),
                role: role.to_string(),
            }
        })?;

        for dependency in &self.dependency_closure {
            validate_nonempty(
                "consistency.backup_units[].dependency_closure[]",
                dependency,
            )?;
        }
        validate_unique_values(
            "consistency.backup_units[].dependency_closure[]",
            &self.dependency_closure,
            |dependency| ManifestValidationError::DuplicateBackupUnitDependency {
                unit_id: self.unit_id.clone(),
                dependency: dependency.to_string(),
            },
        )?;

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceSnapshot {
    pub snapshot_id: String,
    pub module_hash: Option<String>,
    pub wasm_hash: Option<String>,
    pub code_version: Option<String>,
    pub artifact_path: String,
    pub checksum_algorithm: String,
    #[serde(default)]
    pub checksum: Option<String>,
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
        validate_optional_hash(
            "fleet.members[].source_snapshot.checksum",
            self.checksum.as_deref(),
        )?;
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
        validate_unique_values("verification.check.roles[]", &self.roles, |role| {
            ManifestValidationError::DuplicateVerificationCheckRole {
                kind: self.kind.clone(),
                role: role.to_string(),
            }
        })?;
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

    #[error("duplicate backup unit id {0}")]
    DuplicateBackupUnitId(String),

    #[error("backup unit {unit_id} repeats role {role}")]
    DuplicateBackupUnitRole { unit_id: String, role: String },

    #[error("backup unit {unit_id} repeats dependency {dependency}")]
    DuplicateBackupUnitDependency { unit_id: String, dependency: String },

    #[error("fleet member {0} has no concrete verification checks")]
    MissingMemberVerificationChecks(String),

    #[error("backup unit {unit_id} references unknown role {role}")]
    UnknownBackupUnitRole { unit_id: String, role: String },

    #[error("backup unit {unit_id} references unknown dependency {dependency}")]
    UnknownBackupUnitDependency { unit_id: String, dependency: String },

    #[error("fleet role {role} is not covered by any backup unit")]
    BackupUnitCoverageMissingRole { role: String },

    #[error("verification plan references unknown role {role}")]
    UnknownVerificationRole { role: String },

    #[error("duplicate member verification role {0}")]
    DuplicateMemberVerificationRole(String),

    #[error("verification check {kind} repeats role {role}")]
    DuplicateVerificationCheckRole { kind: String, role: String },

    #[error("whole-fleet backup unit {unit_id} omits fleet role {role}")]
    WholeFleetUnitMissingRole { unit_id: String, role: String },

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

// Return whether every fleet role is included in at least one backup unit.
fn all_fleet_roles_covered(manifest: &FleetBackupManifest) -> bool {
    let fleet_roles = manifest
        .fleet
        .members
        .iter()
        .map(|member| member.role.as_str())
        .collect::<BTreeSet<_>>();
    let covered_roles = manifest
        .consistency
        .backup_units
        .iter()
        .flat_map(|unit| unit.roles.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();

    fleet_roles.iter().all(|role| covered_roles.contains(role))
}

// Validate cross-section backup unit references after local section checks pass.
fn validate_consistency_against_fleet(
    consistency: &ConsistencySection,
    fleet: &FleetSection,
) -> Result<(), ManifestValidationError> {
    let fleet_roles = fleet
        .members
        .iter()
        .map(|member| member.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut covered_roles = BTreeSet::new();

    for unit in &consistency.backup_units {
        for role in &unit.roles {
            if !fleet_roles.contains(role.as_str()) {
                return Err(ManifestValidationError::UnknownBackupUnitRole {
                    unit_id: unit.unit_id.clone(),
                    role: role.clone(),
                });
            }
            covered_roles.insert(role.as_str());
        }

        for dependency in &unit.dependency_closure {
            if !fleet_roles.contains(dependency.as_str()) {
                return Err(ManifestValidationError::UnknownBackupUnitDependency {
                    unit_id: unit.unit_id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }

        validate_backup_unit_topology(unit, fleet, &fleet_roles)?;
    }

    for role in &fleet_roles {
        if !covered_roles.contains(role) {
            return Err(ManifestValidationError::BackupUnitCoverageMissingRole {
                role: (*role).to_string(),
            });
        }
    }

    Ok(())
}

// Validate verification role references after fleet roles are known.
fn validate_verification_against_fleet(
    verification: &VerificationPlan,
    fleet: &FleetSection,
) -> Result<(), ManifestValidationError> {
    let fleet_roles = fleet
        .members
        .iter()
        .map(|member| member.role.as_str())
        .collect::<BTreeSet<_>>();

    for check in &verification.fleet_checks {
        validate_verification_check_roles(check, &fleet_roles)?;
    }

    for member in &fleet.members {
        for check in &member.verification_checks {
            validate_verification_check_roles(check, &fleet_roles)?;
        }
    }

    let mut member_check_roles = BTreeSet::new();
    for member in &verification.member_checks {
        if !fleet_roles.contains(member.role.as_str()) {
            return Err(ManifestValidationError::UnknownVerificationRole {
                role: member.role.clone(),
            });
        }
        if !member_check_roles.insert(member.role.as_str()) {
            return Err(ManifestValidationError::DuplicateMemberVerificationRole(
                member.role.clone(),
            ));
        }
        for check in &member.checks {
            validate_verification_check_roles(check, &fleet_roles)?;
        }
    }

    Ok(())
}

// Validate every role filter in one verification check.
fn validate_verification_check_roles(
    check: &VerificationCheck,
    fleet_roles: &BTreeSet<&str>,
) -> Result<(), ManifestValidationError> {
    for role in &check.roles {
        if !fleet_roles.contains(role.as_str()) {
            return Err(ManifestValidationError::UnknownVerificationRole { role: role.clone() });
        }
    }

    Ok(())
}

// Validate backup unit topology promises against manifest membership.
fn validate_backup_unit_topology(
    unit: &BackupUnit,
    fleet: &FleetSection,
    fleet_roles: &BTreeSet<&str>,
) -> Result<(), ManifestValidationError> {
    match &unit.kind {
        BackupUnitKind::WholeFleet => validate_whole_fleet_unit(unit, fleet_roles),
        BackupUnitKind::SubtreeRooted => validate_subtree_unit(unit, fleet),
        BackupUnitKind::ControlPlaneSubset | BackupUnitKind::Flat => Ok(()),
    }
}

// Ensure whole-fleet units cover every role declared by the fleet.
fn validate_whole_fleet_unit(
    unit: &BackupUnit,
    fleet_roles: &BTreeSet<&str>,
) -> Result<(), ManifestValidationError> {
    let unit_roles = unit
        .roles
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for role in fleet_roles {
        if !unit_roles.contains(role) {
            return Err(ManifestValidationError::WholeFleetUnitMissingRole {
                unit_id: unit.unit_id.clone(),
                role: (*role).to_string(),
            });
        }
    }

    Ok(())
}

// Ensure subtree units are connected and closed under descendants.
fn validate_subtree_unit(
    unit: &BackupUnit,
    fleet: &FleetSection,
) -> Result<(), ManifestValidationError> {
    let unit_roles = unit
        .roles
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let members_by_id = fleet
        .members
        .iter()
        .map(|member| (member.canister_id.as_str(), member))
        .collect::<BTreeMap<_, _>>();
    let unit_member_ids = fleet
        .members
        .iter()
        .filter(|member| unit_roles.contains(member.role.as_str()))
        .map(|member| member.canister_id.as_str())
        .collect::<BTreeSet<_>>();

    let root_count = fleet
        .members
        .iter()
        .filter(|member| unit_member_ids.contains(member.canister_id.as_str()))
        .filter(|member| {
            member
                .parent_canister_id
                .as_deref()
                .is_none_or(|parent| !unit_member_ids.contains(parent))
        })
        .count();
    if root_count != 1 {
        return Err(ManifestValidationError::SubtreeBackupUnitNotConnected {
            unit_id: unit.unit_id.clone(),
        });
    }

    for member in &fleet.members {
        if unit_member_ids.contains(member.canister_id.as_str()) {
            continue;
        }

        if let Some(parent) = first_unit_ancestor(member, &members_by_id, &unit_member_ids) {
            return Err(
                ManifestValidationError::SubtreeBackupUnitMissingDescendant {
                    unit_id: unit.unit_id.clone(),
                    parent: parent.to_string(),
                    descendant: member.canister_id.clone(),
                },
            );
        }
    }

    Ok(())
}

// Return the nearest selected ancestor for a member outside a subtree unit.
fn first_unit_ancestor<'a>(
    member: &'a FleetMember,
    members_by_id: &BTreeMap<&'a str, &'a FleetMember>,
    unit_member_ids: &BTreeSet<&'a str>,
) -> Option<&'a str> {
    let mut visited = BTreeSet::new();
    let mut parent = member.parent_canister_id.as_deref();
    while let Some(parent_id) = parent {
        if unit_member_ids.contains(parent_id) {
            return Some(parent_id);
        }
        if !visited.insert(parent_id) {
            return None;
        }
        parent = members_by_id
            .get(parent_id)
            .and_then(|ancestor| ancestor.parent_canister_id.as_deref());
    }

    None
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

// Validate that a string list does not repeat values.
fn validate_unique_values<F>(
    field: &'static str,
    values: &[String],
    error: F,
) -> Result<(), ManifestValidationError>
where
    F: Fn(&str) -> ManifestValidationError,
{
    let mut seen = BTreeSet::new();
    for value in values {
        validate_nonempty(field, value)?;
        if !seen.insert(value.as_str()) {
            return Err(error(value));
        }
    }

    Ok(())
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

// Validate optional SHA-256 hex values only when present.
fn validate_optional_hash(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ManifestValidationError> {
    if let Some(value) = value {
        validate_hash(field, value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
