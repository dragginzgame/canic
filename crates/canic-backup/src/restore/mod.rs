use crate::manifest::{FleetBackupManifest, FleetMember, IdentityMode, ManifestValidationError};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
use thiserror::Error as ThisError;

///
/// RestoreMapping
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RestoreMapping {
    pub members: Vec<RestoreMappingEntry>,
}

impl RestoreMapping {
    /// Resolve the target canister for one source member.
    fn target_for(&self, source_canister: &str) -> Option<&str> {
        self.members
            .iter()
            .find(|entry| entry.source_canister == source_canister)
            .map(|entry| entry.target_canister.as_str())
    }
}

///
/// RestoreMappingEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RestoreMappingEntry {
    pub source_canister: String,
    pub target_canister: String,
}

///
/// RestorePlan
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePlan {
    pub phases: Vec<RestorePhase>,
}

impl RestorePlan {
    /// Return all planned members in execution order.
    #[must_use]
    pub fn ordered_members(&self) -> Vec<&RestorePlanMember> {
        self.phases
            .iter()
            .flat_map(|phase| phase.members.iter())
            .collect()
    }
}

///
/// RestorePhase
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePhase {
    pub restore_group: u16,
    pub members: Vec<RestorePlanMember>,
}

///
/// RestorePlanMember
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePlanMember {
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    pub parent_source_canister: Option<String>,
    pub restore_group: u16,
}

///
/// RestorePlanner
///

pub struct RestorePlanner;

impl RestorePlanner {
    /// Build a no-mutation restore plan from the manifest and optional target mapping.
    pub fn plan(
        manifest: &FleetBackupManifest,
        mapping: Option<&RestoreMapping>,
    ) -> Result<RestorePlan, RestorePlanError> {
        manifest.validate()?;
        if let Some(mapping) = mapping {
            validate_mapping(mapping)?;
        }

        let members = resolve_members(manifest, mapping)?;
        let phases = group_and_order_members(members)?;

        Ok(RestorePlan { phases })
    }
}

///
/// RestorePlanError
///

#[derive(Debug, ThisError)]
pub enum RestorePlanError {
    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("mapping contains duplicate source canister {0}")]
    DuplicateMappingSource(String),

    #[error("mapping contains duplicate target canister {0}")]
    DuplicateMappingTarget(String),

    #[error("mapping is missing source canister {0}")]
    MissingMappingSource(String),

    #[error("fixed-identity member {source_canister} cannot be mapped to {target_canister}")]
    FixedIdentityRemap {
        source_canister: String,
        target_canister: String,
    },

    #[error("restore plan contains duplicate target canister {0}")]
    DuplicatePlanTarget(String),

    #[error("restore group {0} contains a parent cycle or unresolved dependency")]
    RestoreOrderCycle(u16),
}

// Validate a user-supplied restore mapping before applying it to the manifest.
fn validate_mapping(mapping: &RestoreMapping) -> Result<(), RestorePlanError> {
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();

    for entry in &mapping.members {
        validate_principal("mapping.members[].source_canister", &entry.source_canister)?;
        validate_principal("mapping.members[].target_canister", &entry.target_canister)?;

        if !sources.insert(entry.source_canister.clone()) {
            return Err(RestorePlanError::DuplicateMappingSource(
                entry.source_canister.clone(),
            ));
        }

        if !targets.insert(entry.target_canister.clone()) {
            return Err(RestorePlanError::DuplicateMappingTarget(
                entry.target_canister.clone(),
            ));
        }
    }

    Ok(())
}

// Resolve source manifest members into target restore members.
fn resolve_members(
    manifest: &FleetBackupManifest,
    mapping: Option<&RestoreMapping>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    let mut plan_members = Vec::with_capacity(manifest.fleet.members.len());
    let mut targets = BTreeSet::new();

    for member in &manifest.fleet.members {
        let target = resolve_target(member, mapping)?;
        if !targets.insert(target.clone()) {
            return Err(RestorePlanError::DuplicatePlanTarget(target));
        }

        plan_members.push(RestorePlanMember {
            source_canister: member.canister_id.clone(),
            target_canister: target,
            role: member.role.clone(),
            parent_source_canister: member.parent_canister_id.clone(),
            restore_group: member.restore_group,
        });
    }

    Ok(plan_members)
}

// Resolve one member's target canister, enforcing identity continuity.
fn resolve_target(
    member: &FleetMember,
    mapping: Option<&RestoreMapping>,
) -> Result<String, RestorePlanError> {
    let target = match mapping {
        Some(mapping) => mapping
            .target_for(&member.canister_id)
            .ok_or_else(|| RestorePlanError::MissingMappingSource(member.canister_id.clone()))?
            .to_string(),
        None => member.canister_id.clone(),
    };

    if matches!(member.identity_mode, IdentityMode::Fixed) && target != member.canister_id {
        return Err(RestorePlanError::FixedIdentityRemap {
            source_canister: member.canister_id.clone(),
            target_canister: target,
        });
    }

    Ok(target)
}

// Group members and apply parent-before-child ordering inside each group.
fn group_and_order_members(
    members: Vec<RestorePlanMember>,
) -> Result<Vec<RestorePhase>, RestorePlanError> {
    let mut groups = BTreeMap::<u16, Vec<RestorePlanMember>>::new();
    for member in members {
        groups.entry(member.restore_group).or_default().push(member);
    }

    groups
        .into_iter()
        .map(|(restore_group, members)| {
            let members = order_group(restore_group, members)?;
            Ok(RestorePhase {
                restore_group,
                members,
            })
        })
        .collect()
}

// Topologically order one group using manifest parent relationships.
fn order_group(
    restore_group: u16,
    members: Vec<RestorePlanMember>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    let mut remaining = members;
    let group_sources = remaining
        .iter()
        .map(|member| member.source_canister.clone())
        .collect::<BTreeSet<_>>();
    let mut emitted = BTreeSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let Some(index) = remaining
            .iter()
            .position(|member| parent_satisfied(member, &group_sources, &emitted))
        else {
            return Err(RestorePlanError::RestoreOrderCycle(restore_group));
        };

        let member = remaining.remove(index);
        emitted.insert(member.source_canister.clone());
        ordered.push(member);
    }

    Ok(ordered)
}

// Determine whether a member's in-group parent has already been emitted.
fn parent_satisfied(
    member: &RestorePlanMember,
    group_sources: &BTreeSet<String>,
    emitted: &BTreeSet<String>,
) -> bool {
    match &member.parent_source_canister {
        Some(parent) if group_sources.contains(parent) => emitted.contains(parent),
        _ => true,
    }
}

// Validate textual principal fields used in mappings.
fn validate_principal(field: &'static str, value: &str) -> Result<(), RestorePlanError> {
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| RestorePlanError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetSection,
        SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck, VerificationPlan,
    };

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const TARGET: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Build one valid manifest with a parent and child in the same restore group.
    fn valid_manifest(identity_mode: IdentityMode) -> FleetBackupManifest {
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
                mode: ConsistencyMode::CrashConsistent,
                backup_units: vec![BackupUnit {
                    unit_id: "whole-fleet".to_string(),
                    kind: BackupUnitKind::WholeFleet,
                    roles: vec!["root".to_string(), "app".to_string()],
                    consistency_reason: None,
                    dependency_closure: Vec::new(),
                    topology_validation: "subtree-closed".to_string(),
                    quiescence_strategy: None,
                }],
            },
            fleet: FleetSection {
                topology_hash_algorithm: "sha256".to_string(),
                topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
                discovery_topology_hash: HASH.to_string(),
                pre_snapshot_topology_hash: HASH.to_string(),
                topology_hash: HASH.to_string(),
                members: vec![
                    fleet_member("app", CHILD, Some(ROOT), identity_mode, 1),
                    fleet_member("root", ROOT, None, IdentityMode::Fixed, 1),
                ],
            },
            verification: VerificationPlan {
                fleet_checks: Vec::new(),
                member_checks: Vec::new(),
            },
        }
    }

    // Build one manifest member for restore planning tests.
    fn fleet_member(
        role: &str,
        canister_id: &str,
        parent_canister_id: Option<&str>,
        identity_mode: IdentityMode,
        restore_group: u16,
    ) -> FleetMember {
        FleetMember {
            role: role.to_string(),
            canister_id: canister_id.to_string(),
            parent_canister_id: parent_canister_id.map(str::to_string),
            subnet_canister_id: None,
            controller_hint: Some(ROOT.to_string()),
            identity_mode,
            restore_group,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "call".to_string(),
                method: Some("canic_ready".to_string()),
                roles: Vec::new(),
            }],
            source_snapshot: SourceSnapshot {
                snapshot_id: format!("snap-{role}"),
                module_hash: Some(HASH.to_string()),
                wasm_hash: Some(HASH.to_string()),
                code_version: Some("v0.30.0".to_string()),
                artifact_path: format!("artifacts/{role}"),
                checksum_algorithm: "sha256".to_string(),
            },
        }
    }

    // Ensure in-place restore planning sorts parent before child.
    #[test]
    fn in_place_plan_orders_parent_before_child() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let ordered = plan.ordered_members();

        assert_eq!(ordered[0].source_canister, ROOT);
        assert_eq!(ordered[1].source_canister, CHILD);
    }

    // Ensure fixed identities cannot be remapped.
    #[test]
    fn fixed_identity_member_cannot_be_remapped() {
        let manifest = valid_manifest(IdentityMode::Fixed);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
            ],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("fixed member remap should fail");

        assert!(matches!(err, RestorePlanError::FixedIdentityRemap { .. }));
    }

    // Ensure relocatable identities may be mapped when all members are covered.
    #[test]
    fn relocatable_member_can_be_mapped() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
            ],
        };

        let plan = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");
        let child = plan
            .ordered_members()
            .into_iter()
            .find(|member| member.source_canister == CHILD)
            .expect("child member should be planned");

        assert_eq!(child.target_canister, TARGET);
    }

    // Ensure mapped restores must cover every source member.
    #[test]
    fn mapped_restore_requires_complete_mapping() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            }],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("incomplete mapping should fail");

        assert!(matches!(err, RestorePlanError::MissingMappingSource(_)));
    }

    // Ensure duplicate target mappings fail before a plan is produced.
    #[test]
    fn duplicate_mapping_targets_fail_validation() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: ROOT.to_string(),
                },
            ],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("duplicate targets should fail");

        assert!(matches!(err, RestorePlanError::DuplicateMappingTarget(_)));
    }
}
