//! Module: manifest::validation::sections
//!
//! Responsibility: validate individual backup manifest sections.
//! Does not own: cross-section binding, persistence, or restore execution.
//! Boundary: attaches validation methods to manifest section types.

use crate::manifest::validation::{
    SHA256_ALGORITHM,
    scalar::{
        validate_hash, validate_nonempty, validate_optional_hash, validate_optional_nonempty,
        validate_optional_principal, validate_principal, validate_unique_values,
    },
};
use crate::manifest::{
    BackupUnit, ConsistencySection, DeploymentMember, DeploymentSection, ManifestValidationError,
    MemberVerificationChecks, SourceMetadata, SourceSnapshot, ToolMetadata,
    VERIFICATION_KIND_STATUS, VerificationCheck, VerificationPlan,
};

use std::collections::BTreeSet;

impl ToolMetadata {
    pub(super) fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("tool.name", &self.name)?;
        validate_nonempty("tool.version", &self.version)
    }
}

impl SourceMetadata {
    pub(super) fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("source.network", &self.network)?;
        validate_principal("source.root_canister", &self.root_canister)
    }
}

impl ConsistencySection {
    pub(super) fn validate(&self) -> Result<(), ManifestValidationError> {
        if self.backup_units.is_empty() {
            return Err(ManifestValidationError::EmptyCollection(
                "consistency.backup_units",
            ));
        }

        let mut unit_ids = BTreeSet::new();
        for unit in &self.backup_units {
            unit.validate()?;
            if !unit_ids.insert(unit.unit_id.clone()) {
                return Err(ManifestValidationError::DuplicateBackupUnitId(
                    unit.unit_id.clone(),
                ));
            }
        }

        Ok(())
    }
}

impl BackupUnit {
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("consistency.backup_units[].unit_id", &self.unit_id)?;

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

        Ok(())
    }
}

impl DeploymentSection {
    pub(crate) fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty(
            "deployment.topology_hash_algorithm",
            &self.topology_hash_algorithm,
        )?;
        if self.topology_hash_algorithm != SHA256_ALGORITHM {
            return Err(ManifestValidationError::UnsupportedHashAlgorithm(
                self.topology_hash_algorithm.clone(),
            ));
        }

        validate_nonempty("deployment.topology_hash_input", &self.topology_hash_input)?;
        validate_hash(
            "deployment.discovery_topology_hash",
            &self.discovery_topology_hash,
        )?;
        validate_hash(
            "deployment.pre_snapshot_topology_hash",
            &self.pre_snapshot_topology_hash,
        )?;
        validate_hash("deployment.topology_hash", &self.topology_hash)?;

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
            return Err(ManifestValidationError::EmptyCollection(
                "deployment.members",
            ));
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

impl DeploymentMember {
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("deployment.members[].role", &self.role)?;
        validate_principal("deployment.members[].canister_id", &self.canister_id)?;
        validate_optional_principal(
            "deployment.members[].parent_canister_id",
            self.parent_canister_id.as_deref(),
        )?;
        validate_optional_principal(
            "deployment.members[].subnet_canister_id",
            self.subnet_canister_id.as_deref(),
        )?;
        validate_optional_principal(
            "deployment.members[].controller_hint",
            self.controller_hint.as_deref(),
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

impl SourceSnapshot {
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty(
            "deployment.members[].source_snapshot.snapshot_id",
            &self.snapshot_id,
        )?;
        validate_optional_nonempty(
            "deployment.members[].source_snapshot.module_hash",
            self.module_hash.as_deref(),
        )?;
        validate_optional_nonempty(
            "deployment.members[].source_snapshot.code_version",
            self.code_version.as_deref(),
        )?;
        validate_nonempty(
            "deployment.members[].source_snapshot.artifact_path",
            &self.artifact_path,
        )?;
        validate_nonempty(
            "deployment.members[].source_snapshot.checksum_algorithm",
            &self.checksum_algorithm,
        )?;
        if self.checksum_algorithm != SHA256_ALGORITHM {
            return Err(ManifestValidationError::UnsupportedHashAlgorithm(
                self.checksum_algorithm.clone(),
            ));
        }
        validate_optional_hash(
            "deployment.members[].source_snapshot.checksum",
            self.checksum.as_deref(),
        )?;
        Ok(())
    }
}

impl VerificationPlan {
    pub(super) fn validate(&self) -> Result<(), ManifestValidationError> {
        for check in &self.deployment_checks {
            check.validate()?;
        }
        for member in &self.member_checks {
            member.validate()?;
        }
        Ok(())
    }
}

impl MemberVerificationChecks {
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

impl VerificationCheck {
    fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_nonempty("verification.check.kind", &self.kind)?;
        if self.kind != VERIFICATION_KIND_STATUS {
            return Err(ManifestValidationError::UnsupportedVerificationKind(
                self.kind.clone(),
            ));
        }
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
