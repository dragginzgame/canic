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
mod tests {
    use super::*;
    use candid::Principal;

    const ROOT: Principal = Principal::from_slice(&[]);
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Build a deterministic non-root principal for discovery tests.
    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    // Ensure discovery projections produce valid manifest fleet sections.
    #[test]
    fn discovery_projects_to_valid_fleet_section() {
        let fleet = DiscoveredFleet {
            topology_records: vec![
                topology_record(ROOT, None, "root"),
                topology_record(p(2), Some(ROOT), "app"),
            ],
            members: vec![
                discovered_member("root", ROOT.to_string(), None),
                discovered_member("app", p(2).to_string(), Some(ROOT.to_string())),
            ],
        };

        let section = fleet
            .into_fleet_section()
            .expect("discovery should project");

        section.validate().expect("fleet section should validate");
        assert_eq!(section.discovery_topology_hash, section.topology_hash);
        assert_eq!(section.members.len(), 2);
    }

    // Ensure duplicate canisters are rejected before manifest projection.
    #[test]
    fn discovery_rejects_duplicate_canisters() {
        let fleet = DiscoveredFleet {
            topology_records: vec![topology_record(ROOT, None, "root")],
            members: vec![
                discovered_member("root", ROOT.to_string(), None),
                discovered_member("root", ROOT.to_string(), None),
            ],
        };

        let err = fleet
            .into_fleet_section()
            .expect_err("duplicate canisters should fail");

        assert!(matches!(err, DiscoveryError::DuplicateCanisterId(_)));
    }

    // Ensure discovery requires concrete member verification.
    #[test]
    fn discovery_requires_verification_checks() {
        let mut member = discovered_member("root", ROOT.to_string(), None);
        member.verification_checks.clear();
        let fleet = DiscoveredFleet {
            topology_records: vec![topology_record(ROOT, None, "root")],
            members: vec![member],
        };

        let err = fleet
            .into_fleet_section()
            .expect_err("missing verification should fail");

        assert!(matches!(err, DiscoveryError::MissingVerificationChecks(_)));
    }

    // Build one topology record for tests.
    fn topology_record(
        pid: Principal,
        parent_pid: Option<Principal>,
        role: &str,
    ) -> TopologyRecord {
        TopologyRecord {
            pid,
            parent_pid,
            role: role.to_string(),
            module_hash: None,
        }
    }

    // Build one discovered member for tests.
    fn discovered_member(
        role: &str,
        canister_id: String,
        parent_canister_id: Option<String>,
    ) -> DiscoveredMember {
        DiscoveredMember {
            role: role.to_string(),
            canister_id,
            parent_canister_id,
            subnet_canister_id: None,
            controller_hint: Some(ROOT.to_string()),
            identity_mode: IdentityMode::Fixed,
            restore_group: 1,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "call".to_string(),
                method: Some("canic_ready".to_string()),
                roles: Vec::new(),
            }],
            snapshot_plan: SnapshotPlan {
                snapshot_id: format!("snap-{role}"),
                module_hash: Some(HASH.to_string()),
                wasm_hash: Some(HASH.to_string()),
                code_version: Some("v0.30.0".to_string()),
                artifact_path: format!("artifacts/{role}"),
                checksum_algorithm: "sha256".to_string(),
                checksum: Some(HASH.to_string()),
            },
        }
    }
}
