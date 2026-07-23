//! Module: state_manifest
//!
//! Responsibility: build host-side state manifest and audit reports from
//! Rust-authored Canic state declarations.
//! Does not own: stable-memory inspection, migration execution, CLI parsing, or
//! runtime introspection.
//! Boundary: consumes passive declaration metadata from `canic-core` and emits
//! diagnostic-only reports.

mod aggregation;
mod audit;
mod resolution;

pub use resolution::{StateManifestResolution, resolve_project_state_manifest};

use canic_core::state_contract::{STATE_MANIFEST_SCHEMA_VERSION, StateManifest};
use serde::Serialize;

pub const STATE_AUDIT_COMMAND: &str = "canic state audit";
pub const STATE_MANIFEST_COMMAND: &str = "canic state manifest";
pub const STATE_AUDIT_SCHEMA_VERSION: u16 = 1;

const SCOPE_PROJECT: StateAuditScope = StateAuditScope::Project;
const SCOPE_ROLE: StateAuditScope = StateAuditScope::Role;

///
/// StateAuditReport
///
/// Diagnostic report for declared state domains.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StateAuditReport {
    pub schema_version: u16,
    pub command: &'static str,
    pub scope: StateAuditScope,
    pub role: Option<String>,
    pub status: StateAuditStatus,
    pub manifest: StateManifest,
    pub checks: Vec<StateAuditCheck>,
    pub next_actions: Vec<String>,
}

///
/// StateAuditCheck
///
/// Stable check row emitted by `canic state audit`.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StateAuditCheck {
    pub category: StateAuditCategory,
    pub code: &'static str,
    pub status: StateAuditStatus,
    pub severity: StateAuditSeverity,
    pub subject: String,
    pub detail: String,
    pub next: Option<String>,
    pub source: StateAuditSource,
}

///
/// StateAuditScope
///
/// Stable audit scope emitted by `canic state audit`.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAuditScope {
    Project,
    Role,
}

impl StateAuditScope {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Role => "role",
        }
    }
}

///
/// StateAuditCategory
///
/// Stable category label emitted by state-audit checks.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAuditCategory {
    Invariant,
    Lifecycle,
    Manifest,
    MemoryId,
    Migration,
    Naming,
    SchemaVersion,
    Snapshot,
    TestCoverage,
}

impl StateAuditCategory {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Invariant => "invariant",
            Self::Lifecycle => "lifecycle",
            Self::Manifest => "manifest",
            Self::MemoryId => "memory_id",
            Self::Migration => "migration",
            Self::Naming => "naming",
            Self::SchemaVersion => "schema_version",
            Self::Snapshot => "snapshot",
            Self::TestCoverage => "test_coverage",
        }
    }
}

///
/// StateAuditSource
///
/// Stable source-attribution label emitted by state-audit checks.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAuditSource {
    StateManifest,
}

impl StateAuditSource {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::StateManifest => "state_manifest",
        }
    }
}

///
/// StateAuditStatus
///
/// Stable audit status for reports and checks.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAuditStatus {
    Pass,
    Warn,
    Fail,
    NotEvaluated,
}

impl StateAuditStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::NotEvaluated => "not_evaluated",
        }
    }
}

///
/// StateAuditSeverity
///
/// Stable severity framing for audit checks.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAuditSeverity {
    Info,
    Warning,
    Blocked,
    Unsupported,
}

#[must_use]
pub fn build_state_audit_report(
    resolution: &StateManifestResolution,
    role: Option<&str>,
) -> StateAuditReport {
    let (manifest, contract_errors) = match resolution {
        StateManifestResolution::Resolved { manifest, .. } => (manifest.clone(), Vec::new()),
        StateManifestResolution::Rejected { errors } => (
            StateManifest {
                schema_version: STATE_MANIFEST_SCHEMA_VERSION,
                roles: Vec::new(),
            },
            errors.clone(),
        ),
    };
    let mut checks = audit::audit_checks(
        &manifest,
        if contract_errors.is_empty() {
            role
        } else {
            None
        },
    );
    checks.extend(contract_errors.iter().map(audit::role_contract_check));
    aggregation::sort_checks(&mut checks);

    let status = aggregation::aggregate_status(&checks);
    let mut next_actions = aggregation::next_actions(status, role);
    next_actions.sort();
    next_actions.dedup();

    StateAuditReport {
        schema_version: STATE_AUDIT_SCHEMA_VERSION,
        command: STATE_AUDIT_COMMAND,
        scope: if role.is_some() {
            SCOPE_ROLE
        } else {
            SCOPE_PROJECT
        },
        role: role.map(ToString::to_string),
        status,
        manifest,
        checks,
        next_actions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role_contract::materialize_state_manifest;
    use canic_core::{
        ids::CanisterRole,
        role_contract::{
            AllocationOwner, BuiltInRoleKind, CanicFeatureKey, ResolvedRoleContract,
            ResolvedStateAllocation, RoleContractFinding, SelectionProvenance, StateAllocationKey,
            allocation::allocation_definition,
        },
        state_contract::{
            MigrationPolicy, ReservedMemoryManifest, StateDomainManifest, StateMigrationManifest,
            StateRoleManifest, StateStorage,
        },
    };
    use std::{collections::BTreeSet, path::PathBuf};

    use super::audit::audit_checks;

    fn test_state_manifest(role: Option<&str>) -> StateManifest {
        let contracts = match role {
            Some("root") | None => vec![test_contract(
                "root",
                None,
                &[
                    StateAllocationKey::CoreRuntimeTopology,
                    StateAllocationKey::CoreRuntimeEnvironment,
                    StateAllocationKey::CoreAuthState,
                    StateAllocationKey::CoreReplayReceipts,
                    StateAllocationKey::CoreRuntimeObservability,
                    StateAllocationKey::CoreRuntimeIntent,
                    StateAllocationKey::CanisterPool,
                    StateAllocationKey::TemplateManifests,
                    StateAllocationKey::TemplateChunkSets,
                    StateAllocationKey::TemplateChunkRefs,
                    StateAllocationKey::TemplateChunkPayloads,
                    StateAllocationKey::ControlPlaneSubnetState,
                ],
            )],
            Some("wasm_store") => vec![test_contract(
                "wasm_store",
                Some(BuiltInRoleKind::WasmStore),
                &[
                    StateAllocationKey::TemplateManifests,
                    StateAllocationKey::TemplateChunkSets,
                    StateAllocationKey::TemplateChunkRefs,
                    StateAllocationKey::TemplateChunkPayloads,
                    StateAllocationKey::WasmStoreGcState,
                ],
            )],
            Some(_) => Vec::new(),
        };
        materialize_state_manifest(&contracts).expect("test manifest")
    }

    fn test_contract(
        role: &str,
        built_in: Option<BuiltInRoleKind>,
        keys: &[StateAllocationKey],
    ) -> ResolvedRoleContract {
        let allocations = keys
            .iter()
            .map(|key| {
                let definition = allocation_definition(*key).expect("allocation definition");
                ResolvedStateAllocation {
                    key: *key,
                    owner: definition.owner,
                    memory_ids: definition.memory_ids.to_vec(),
                    selected_by: BTreeSet::from([if let Some(built_in) = built_in {
                        SelectionProvenance::BuiltInRole(built_in)
                    } else if definition.owner == AllocationOwner::CanicControlPlane {
                        SelectionProvenance::EffectiveFeature(CanicFeatureKey::ControlPlane)
                    } else {
                        SelectionProvenance::Capability(
                            canic_core::role_contract::RoleCapabilityKey::Root,
                        )
                    }]),
                }
            })
            .collect();
        ResolvedRoleContract {
            role: CanisterRole::owned(role.to_string()),
            built_in,
            capabilities: BTreeSet::new(),
            required_features: BTreeSet::new(),
            effective_features: BTreeSet::new(),
            allocations,
        }
    }

    fn build_state_audit_report(role: Option<&str>) -> StateAuditReport {
        let resolution = StateManifestResolution::Resolved {
            manifest: test_state_manifest(role),
            contracts: Vec::new(),
        };
        super::build_state_audit_report(&resolution, role)
    }

    #[test]
    fn state_audit_status_owns_serialized_labels() {
        assert_eq!(StateAuditStatus::Pass.label(), "pass");
        assert_eq!(StateAuditStatus::Warn.label(), "warn");
        assert_eq!(StateAuditStatus::Fail.label(), "fail");
        assert_eq!(StateAuditStatus::NotEvaluated.label(), "not_evaluated");
    }

    #[test]
    fn builtin_report_passes_when_every_active_memory_id_is_modeled() {
        let report = build_state_audit_report(Some("root"));

        assert_eq!(report.status, StateAuditStatus::Pass);
        assert!(report.checks.iter().any(|check| {
            check.code == "state_manifest_schema_version_supported"
                && check.status == StateAuditStatus::Pass
        }));
        assert!(
            report
                .checks
                .iter()
                .all(|check| check.code != "reserved_memory_id_declared")
        );
        assert!(
            report
                .checks
                .iter()
                .all(|check| check.code != "snapshot_name_invalid")
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.code == "reserved_export_import_ok")
        );
        assert!(
            report
                .checks
                .iter()
                .all(|check| check.status != StateAuditStatus::Fail)
        );
    }

    #[test]
    fn builtin_manifest_merges_control_plane_state_by_role() {
        let manifest = test_state_manifest(Some("root"));
        let role = manifest.roles.first().expect("root role");

        assert!(role.state.iter().any(|domain| {
            domain.domain == "template_manifests"
                && domain.owner == "canic-control-plane"
                && domain.memory_id == Some(80)
        }));
        assert!(
            role.state
                .iter()
                .any(|domain| { domain.domain == "auth_state" && domain.owner == "canic-core" })
        );
    }

    #[test]
    fn wasm_store_role_audits_every_owned_memory_domain_cleanly() {
        let report = build_state_audit_report(Some("wasm_store"));

        assert_eq!(report.status, StateAuditStatus::Pass);
        let role = report
            .manifest
            .roles
            .iter()
            .find(|role| role.canister_role == "wasm_store")
            .expect("wasm_store role");
        let domains = role
            .state
            .iter()
            .map(|domain| domain.domain.as_str())
            .collect::<Vec<_>>();

        for expected in [
            "template_manifests",
            "template_chunk_sets",
            "template_chunk_refs",
            "template_chunk_payloads",
            "wasm_store_gc_state",
        ] {
            assert!(domains.contains(&expected));
        }
        assert_eq!(domains.len(), 5);
    }

    #[test]
    fn complete_descriptor_registry_satisfies_state_audit_metadata_contract() {
        let keys = canic_core::role_contract::allocation::allocation_definitions()
            .iter()
            .map(|definition| definition.key)
            .collect::<Vec<_>>();
        let manifest = materialize_state_manifest(&[test_contract("catalog", None, &keys)])
            .expect("complete descriptor manifest");
        let checks = audit_checks(&manifest, Some("catalog"));

        assert!(
            checks
                .iter()
                .all(|check| check.status != StateAuditStatus::Fail),
            "complete descriptor registry must satisfy audit metadata: {checks:#?}"
        );
    }

    #[test]
    fn exact_blob_role_resolution_materializes_blob_allocations() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let config = workspace.join("canisters/test/blob_storage_probe/canic.toml");
        let resolution = resolve_project_state_manifest(&workspace, &[config], Some("test"));
        let StateManifestResolution::Resolved {
            manifest,
            contracts,
        } = resolution
        else {
            panic!("blob role contract should resolve")
        };

        assert_eq!(contracts.len(), 1);
        let role = manifest.roles.first().expect("blob role manifest");
        assert_eq!(role.canister_role, "test");
        assert_eq!(
            role.state
                .iter()
                .filter_map(|domain| domain.memory_id)
                .filter(|memory_id| (62..=65).contains(memory_id))
                .collect::<Vec<_>>(),
            vec![63, 65, 64, 62]
        );
        assert!(role.state.iter().all(|domain| domain.owner == "canic-core"));
    }

    #[test]
    fn placement_roles_materialize_exact_placement_state() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

        for (config_path, role, expected_ids) in [
            ("apps/test/canic.toml", "user_hub", vec![49, 53, 54, 56]),
            (
                "canisters/audit/scaling_probe/canic.toml",
                "scale_hub",
                vec![49, 52],
            ),
            (
                "canisters/test/project_hub_stub/canic.toml",
                "project_hub",
                vec![49, 55],
            ),
        ] {
            let config = workspace.join(config_path);
            let resolution = resolve_project_state_manifest(&workspace, &[config], Some(role));
            let StateManifestResolution::Resolved { manifest, .. } = resolution else {
                panic!("{role} role contract should resolve");
            };
            let mut actual_ids = manifest.roles[0]
                .state
                .iter()
                .filter_map(|domain| domain.memory_id)
                .filter(|memory_id| (49..=56).contains(memory_id))
                .collect::<Vec<_>>();
            actual_ids.sort_unstable();

            assert_eq!(actual_ids, expected_ids, "unexpected state for {role}");
        }
    }

    #[test]
    fn exact_built_in_resolution_materializes_runtime_template_and_gc_allocations() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let resolution = resolve_project_state_manifest(&workspace, &[], Some("wasm_store"));
        let StateManifestResolution::Resolved {
            manifest,
            contracts,
        } = resolution
        else {
            panic!("built-in wasm_store contract should resolve")
        };

        assert_eq!(contracts.len(), 1);
        let mut ids = manifest.roles[0]
            .state
            .iter()
            .filter_map(|domain| domain.memory_id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(
            ids,
            vec![
                11, 12, 13, 15, 16, 18, 20, 29, 30, 34, 35, 39, 40, 41, 42, 43, 44, 45, 46, 47, 80,
                81, 82, 83, 85,
            ]
        );
        assert_eq!(
            manifest.roles[0]
                .reserved_memory
                .iter()
                .map(|entry| entry.memory_id)
                .collect::<Vec<_>>(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn unknown_role_resolution_returns_no_manifest() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let config = workspace.join("canisters/audit/root_probe/canic.toml");
        let resolution = resolve_project_state_manifest(&workspace, &[config], Some("missing"));

        assert!(matches!(
            resolution,
            StateManifestResolution::Rejected { errors }
                if errors.iter().any(|finding| matches!(finding, RoleContractFinding::RoleUnknown { .. }))
        ));
    }

    #[test]
    fn unsupported_manifest_schema_version_fails() {
        let mut manifest = test_state_manifest(Some("root"));
        manifest.schema_version = STATE_MANIFEST_SCHEMA_VERSION + 1;

        let checks = audit_checks(&manifest, Some("root"));

        assert!(checks.iter().any(|check| {
            check.code == "state_manifest_schema_version_unsupported"
                && check.status == StateAuditStatus::Fail
        }));
    }

    #[test]
    fn duplicate_state_role_fails() {
        let mut manifest = test_state_manifest(Some("root"));
        let duplicate = manifest.roles[0].clone();
        manifest.roles.push(duplicate);

        let checks = audit_checks(&manifest, None);

        assert!(checks.iter().any(|check| {
            check.code == "state_role_duplicate" && check.status == StateAuditStatus::Fail
        }));
    }

    #[test]
    fn duplicate_memory_id_fails_within_role() {
        let mut manifest = test_state_manifest(Some("root"));
        let role = manifest.roles.first_mut().expect("root role");
        role.state[1].memory_id = role.state[0].memory_id;

        let checks = audit_checks(&manifest, Some("root"));
        assert!(
            checks
                .iter()
                .any(|check| check.code == "memory_id_duplicate"
                    && check.status == StateAuditStatus::Fail)
        );
    }

    #[test]
    fn duplicate_state_domain_fails_within_role() {
        let mut manifest = test_state_manifest(Some("root"));
        let role = manifest.roles.first_mut().expect("root role");
        let mut duplicate = role.state[0].clone();
        duplicate.memory_id = Some(250);
        role.state.push(duplicate);

        let checks = audit_checks(&manifest, Some("root"));

        assert!(
            checks
                .iter()
                .any(|check| check.code == "state_domain_duplicate"
                    && check.status == StateAuditStatus::Fail)
        );
    }

    #[test]
    fn active_cycle_tracker_memory_is_not_reported_as_reserved() {
        let report = build_state_audit_report(Some("root"));

        assert!(
            report
                .checks
                .iter()
                .all(|check| check.code != "reserved_memory_id_declared")
        );
    }

    #[test]
    fn active_domain_reclaiming_reserved_memory_id_fails() {
        let mut manifest = test_state_manifest(Some("root"));
        let role = manifest.roles.first_mut().expect("root role");
        let reserved_id = 250;
        role.reserved_memory.push(ReservedMemoryManifest {
            label: "future_state".to_string(),
            memory_id: reserved_id,
            owner: "canic-core".to_string(),
            reason: "synthetic collision fixture".to_string(),
        });
        role.state[0].memory_id = Some(reserved_id);

        let checks = audit_checks(&manifest, Some("root"));

        assert!(
            checks
                .iter()
                .any(|check| check.code == "reserved_memory_id_collision"
                    && check.status == StateAuditStatus::Fail)
        );
    }

    #[test]
    fn storage_not_applicable_is_explicit_metadata() {
        let manifest = StateManifest {
            schema_version: STATE_MANIFEST_SCHEMA_VERSION,
            roles: vec![StateRoleManifest {
                canister_role: "root".to_string(),
                state: vec![StateDomainManifest {
                    domain: "external_authority".to_string(),
                    version: 1,
                    storage: StateStorage::NotApplicable,
                    memory_id: None,
                    owner: "canic-core".to_string(),
                    record: "ExternalAuthorityRecord".to_string(),
                    snapshot: "ExternalAuthorityData".to_string(),
                    min_supported_version: 1,
                    migration_policy: MigrationPolicy::NotApplicable,
                    restore_order: Some(10),
                    post_upgrade_invariant: Some("external_authority_invariants".to_string()),
                    migrations: Vec::new(),
                }],
                reserved_memory: Vec::new(),
            }],
        };

        let checks = audit_checks(&manifest, None);

        assert!(checks.iter().any(|check| {
            check.code == "state_domain_storage_not_applicable"
                && check.status == StateAuditStatus::Pass
        }));
        assert!(
            checks
                .iter()
                .all(|check| check.code != "state_domain_missing_memory_id")
        );
    }

    #[test]
    fn invalid_support_window_fails() {
        let manifest = StateManifest {
            schema_version: STATE_MANIFEST_SCHEMA_VERSION,
            roles: vec![StateRoleManifest {
                canister_role: "root".to_string(),
                state: vec![StateDomainManifest {
                    domain: "auth_sessions".to_string(),
                    version: 2,
                    storage: StateStorage::StableMemory,
                    memory_id: Some(19),
                    owner: "canic-core".to_string(),
                    record: "AuthSessionRecord".to_string(),
                    snapshot: "AuthSessionsData".to_string(),
                    min_supported_version: 3,
                    migration_policy: MigrationPolicy::NewDomain,
                    restore_order: Some(10),
                    post_upgrade_invariant: Some("auth_sessions_invariants".to_string()),
                    migrations: Vec::new(),
                }],
                reserved_memory: Vec::new(),
            }],
        };

        let checks = audit_checks(&manifest, None);

        assert!(checks.iter().any(|check| {
            check.code == "state_domain_invalid_support_window"
                && check.status == StateAuditStatus::Fail
        }));
        assert!(
            checks
                .iter()
                .all(|check| check.code != "migration_available")
        );
    }

    #[test]
    fn duplicate_migration_declaration_fails() {
        let manifest = StateManifest {
            schema_version: STATE_MANIFEST_SCHEMA_VERSION,
            roles: vec![StateRoleManifest {
                canister_role: "root".to_string(),
                state: vec![StateDomainManifest {
                    domain: "auth_sessions".to_string(),
                    version: 3,
                    storage: StateStorage::StableMemory,
                    memory_id: Some(19),
                    owner: "canic-core".to_string(),
                    record: "AuthSessionRecord".to_string(),
                    snapshot: "AuthSessionsData".to_string(),
                    min_supported_version: 2,
                    migration_policy: MigrationPolicy::Migrate,
                    restore_order: Some(10),
                    post_upgrade_invariant: Some("auth_sessions_invariants".to_string()),
                    migrations: vec![
                        StateMigrationManifest {
                            from: 2,
                            to: 3,
                            kind: "function".to_string(),
                            name: Some("migrate_auth_sessions_v2_to_v3".to_string()),
                            test: Some(
                                "auth_sessions_v2_to_v3_upgrade_preserves_sessions".to_string(),
                            ),
                        },
                        StateMigrationManifest {
                            from: 2,
                            to: 3,
                            kind: "function".to_string(),
                            name: Some("migrate_auth_sessions_v2_to_v3_again".to_string()),
                            test: Some(
                                "auth_sessions_v2_to_v3_upgrade_preserves_sessions".to_string(),
                            ),
                        },
                    ],
                }],
                reserved_memory: Vec::new(),
            }],
        };

        let checks = audit_checks(&manifest, None);

        assert!(checks.iter().any(|check| {
            check.code == "migration_declaration_duplicate"
                && check.status == StateAuditStatus::Fail
        }));
    }

    #[test]
    fn invalid_migration_declaration_fails() {
        let manifest = StateManifest {
            schema_version: STATE_MANIFEST_SCHEMA_VERSION,
            roles: vec![StateRoleManifest {
                canister_role: "root".to_string(),
                state: vec![StateDomainManifest {
                    domain: "auth_sessions".to_string(),
                    version: 4,
                    storage: StateStorage::StableMemory,
                    memory_id: Some(19),
                    owner: "canic-core".to_string(),
                    record: "AuthSessionRecord".to_string(),
                    snapshot: "AuthSessionsData".to_string(),
                    min_supported_version: 2,
                    migration_policy: MigrationPolicy::Migrate,
                    restore_order: Some(10),
                    post_upgrade_invariant: Some("auth_sessions_invariants".to_string()),
                    migrations: vec![StateMigrationManifest {
                        from: 2,
                        to: 4,
                        kind: "function".to_string(),
                        name: Some("migrate_auth_sessions_v2_to_v4".to_string()),
                        test: Some("auth_sessions_v2_to_v4_upgrade_preserves_sessions".to_string()),
                    }],
                }],
                reserved_memory: Vec::new(),
            }],
        };

        let checks = audit_checks(&manifest, None);

        assert!(checks.iter().any(|check| {
            check.code == "migration_declaration_invalid" && check.status == StateAuditStatus::Fail
        }));
    }

    #[test]
    fn missing_migration_test_warns_separately_from_missing_migration() {
        let manifest = StateManifest {
            schema_version: STATE_MANIFEST_SCHEMA_VERSION,
            roles: vec![StateRoleManifest {
                canister_role: "root".to_string(),
                state: vec![StateDomainManifest {
                    domain: "auth_sessions".to_string(),
                    version: 3,
                    storage: StateStorage::StableMemory,
                    memory_id: Some(19),
                    owner: "canic-core".to_string(),
                    record: "AuthSessionRecord".to_string(),
                    snapshot: "AuthSessionsData".to_string(),
                    min_supported_version: 2,
                    migration_policy: MigrationPolicy::Migrate,
                    restore_order: Some(10),
                    post_upgrade_invariant: Some("auth_sessions_invariants".to_string()),
                    migrations: vec![StateMigrationManifest {
                        from: 2,
                        to: 3,
                        kind: "function".to_string(),
                        name: Some("migrate_auth_sessions_v2_to_v3".to_string()),
                        test: None,
                    }],
                }],
                reserved_memory: Vec::new(),
            }],
        };
        let checks = audit_checks(&manifest, None);

        assert!(
            checks
                .iter()
                .any(|check| check.code == "upgrade_test_missing"
                    && check.status == StateAuditStatus::Warn)
        );
        assert!(checks.iter().all(|check| check.code != "migration_missing"));
    }

    #[test]
    fn unknown_filtered_role_fails() {
        let report = build_state_audit_report(Some("missing"));

        assert_eq!(report.status, StateAuditStatus::Fail);
        assert_eq!(report.checks[0].code, "state_role_missing");
    }
}
