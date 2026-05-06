use crate::{
    manifest::{FleetMember, FleetSection, IdentityMode, SourceSnapshot, VerificationCheck},
    topology::{TopologyHasher, TopologyRecord},
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// DiscoveredFleet
///

#[derive(Clone, Debug)]
pub struct DiscoveredFleet {
    pub topology_records: Vec<TopologyRecord>,
    pub members: Vec<DiscoveredMember>,
}

impl DiscoveredFleet {
    /// Convert discovered topology and member policy into a manifest fleet section.
    pub fn into_fleet_section(self) -> Result<FleetSection, DiscoveryError> {
        validate_discovered_members(&self.members)?;

        let topology_hash = TopologyHasher::hash(&self.topology_records);
        let members = self
            .members
            .into_iter()
            .map(DiscoveredMember::into_fleet_member)
            .collect();

        Ok(FleetSection {
            topology_hash_algorithm: topology_hash.algorithm,
            topology_hash_input: topology_hash.input,
            discovery_topology_hash: topology_hash.hash.clone(),
            pre_snapshot_topology_hash: topology_hash.hash.clone(),
            topology_hash: topology_hash.hash,
            members,
        })
    }
}

///
/// DiscoveredMember
///

#[derive(Clone, Debug)]
pub struct DiscoveredMember {
    pub role: String,
    pub canister_id: String,
    pub parent_canister_id: Option<String>,
    pub subnet_canister_id: Option<String>,
    pub controller_hint: Option<String>,
    pub identity_mode: IdentityMode,
    pub restore_group: u16,
    pub verification_class: String,
    pub verification_checks: Vec<VerificationCheck>,
    pub snapshot_plan: SnapshotPlan,
}

impl DiscoveredMember {
    /// Project this discovery member into the manifest restore contract.
    fn into_fleet_member(self) -> FleetMember {
        FleetMember {
            role: self.role,
            canister_id: self.canister_id,
            parent_canister_id: self.parent_canister_id,
            subnet_canister_id: self.subnet_canister_id,
            controller_hint: self.controller_hint,
            identity_mode: self.identity_mode,
            restore_group: self.restore_group,
            verification_class: self.verification_class,
            verification_checks: self.verification_checks,
            source_snapshot: SourceSnapshot {
                snapshot_id: self.snapshot_plan.snapshot_id,
                module_hash: self.snapshot_plan.module_hash,
                wasm_hash: self.snapshot_plan.wasm_hash,
                code_version: self.snapshot_plan.code_version,
                artifact_path: self.snapshot_plan.artifact_path,
                checksum_algorithm: self.snapshot_plan.checksum_algorithm,
                checksum: self.snapshot_plan.checksum,
            },
        }
    }
}

///
/// SnapshotPlan
///

#[derive(Clone, Debug)]
pub struct SnapshotPlan {
    pub snapshot_id: String,
    pub module_hash: Option<String>,
    pub wasm_hash: Option<String>,
    pub code_version: Option<String>,
    pub artifact_path: String,
    pub checksum_algorithm: String,
    pub checksum: Option<String>,
}

///
/// DiscoveryError
///

#[derive(Debug, ThisError)]
pub enum DiscoveryError {
    #[error("discovered fleet has no members")]
    EmptyFleet,

    #[error("duplicate discovered canister id {0}")]
    DuplicateCanisterId(String),

    #[error("discovered member {0} has no verification checks")]
    MissingVerificationChecks(String),
}

// Validate discovery output before building a manifest projection.
fn validate_discovered_members(members: &[DiscoveredMember]) -> Result<(), DiscoveryError> {
    if members.is_empty() {
        return Err(DiscoveryError::EmptyFleet);
    }

    let mut canister_ids = BTreeSet::new();
    for member in members {
        if !canister_ids.insert(member.canister_id.clone()) {
            return Err(DiscoveryError::DuplicateCanisterId(
                member.canister_id.clone(),
            ));
        }
        if member.verification_checks.is_empty() {
            return Err(DiscoveryError::MissingVerificationChecks(
                member.canister_id.clone(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
