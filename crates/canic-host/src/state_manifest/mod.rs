//! Module: state_manifest
//!
//! Responsibility: build host-side state manifest and audit reports from
//! Rust-authored Canic state declarations.
//! Does not own: stable-memory inspection, migration execution, CLI parsing, or
//! runtime introspection.
//! Boundary: consumes passive declaration metadata from `canic-core` and emits
//! diagnostic-only reports.

use canic_control_plane::state_contract::canic_control_plane_state_manifest;
use canic_core::state_contract::{
    MigrationPolicy, RemovedStateManifest, ReservedMemoryManifest, STATE_MANIFEST_SCHEMA_VERSION,
    StateDomainManifest, StateManifest, StateMigrationManifest, StateRoleManifest, StateStorage,
    canic_state_manifest,
};
use serde::Serialize;
use std::collections::BTreeMap;

pub const STATE_AUDIT_COMMAND: &str = "canic state audit";
pub const STATE_MANIFEST_COMMAND: &str = "canic state manifest";
pub const STATE_AUDIT_SCHEMA_VERSION: u16 = 1;

const SCOPE_PROJECT: &str = "project";
const SCOPE_ROLE: &str = "role";
const SOURCE_STATE_MANIFEST: &str = "state_manifest";
const CATEGORY_MANIFEST: &str = "manifest";
const CATEGORY_SCHEMA_VERSION: &str = "schema_version";
const CATEGORY_MEMORY_ID: &str = "memory_id";
const CATEGORY_MIGRATION: &str = "migration";
const CATEGORY_REMOVED_STATE: &str = "removed_state";
const CATEGORY_SNAPSHOT: &str = "snapshot";
const CATEGORY_NAMING: &str = "naming";
const CATEGORY_LIFECYCLE: &str = "lifecycle";
const CATEGORY_INVARIANT: &str = "invariant";
const CATEGORY_TEST_COVERAGE: &str = "test_coverage";

///
/// StateAuditReport
///
/// Diagnostic report for declared state domains.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StateAuditReport {
    pub schema_version: u16,
    pub command: &'static str,
    pub scope: &'static str,
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
    pub category: &'static str,
    pub code: &'static str,
    pub status: StateAuditStatus,
    pub severity: StateAuditSeverity,
    pub subject: String,
    pub detail: String,
    pub next: Option<String>,
    pub source: &'static str,
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
pub fn declared_state_manifest(role: Option<&str>) -> StateManifest {
    let mut manifest = merge_manifests(vec![
        canic_state_manifest(),
        canic_control_plane_state_manifest(),
    ]);
    if let Some(role) = role {
        manifest
            .roles
            .retain(|entry| entry.canister_role.as_str() == role);
    }
    manifest
}

fn merge_manifests(manifests: Vec<StateManifest>) -> StateManifest {
    let mut by_role = BTreeMap::<String, StateRoleManifest>::new();

    for manifest in manifests {
        for role in manifest.roles {
            let entry = by_role
                .entry(role.canister_role.clone())
                .or_insert_with(|| StateRoleManifest {
                    canister_role: role.canister_role.clone(),
                    state: Vec::new(),
                    removed_state: Vec::new(),
                    reserved_memory: Vec::new(),
                });
            entry.state.extend(role.state);
            entry.removed_state.extend(role.removed_state);
            entry.reserved_memory.extend(role.reserved_memory);
        }
    }

    let mut roles = by_role.into_values().collect::<Vec<_>>();
    sort_roles(&mut roles);
    StateManifest {
        schema_version: STATE_MANIFEST_SCHEMA_VERSION,
        roles,
    }
}

fn sort_roles(roles: &mut [StateRoleManifest]) {
    roles.sort_by(|left, right| left.canister_role.cmp(&right.canister_role));
    for role in roles {
        role.state
            .sort_by(|left, right| left.domain.cmp(&right.domain));
        role.removed_state
            .sort_by(|left, right| left.domain.cmp(&right.domain));
        role.reserved_memory
            .sort_by_key(|reservation| reservation.memory_id);
        for domain in &mut role.state {
            domain
                .migrations
                .sort_by_key(|migration| (migration.from, migration.to));
        }
    }
}

#[must_use]
pub fn build_state_audit_report(role: Option<&str>) -> StateAuditReport {
    let manifest = declared_state_manifest(role);
    let mut checks = audit_checks(&manifest, role);
    sort_checks(&mut checks);

    let status = aggregate_status(&checks);
    let mut next_actions = next_actions(status, role);
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

fn audit_checks(manifest: &StateManifest, role_filter: Option<&str>) -> Vec<StateAuditCheck> {
    let mut checks = manifest_schema_checks(manifest);

    if manifest.roles.is_empty() {
        if let Some(check) = role_filter.map(role_not_found_check) {
            checks.push(check);
        }
        return checks;
    }

    checks.extend(role_identity_checks(&manifest.roles));

    for role in &manifest.roles {
        checks.push(pass(
            CATEGORY_MANIFEST,
            "state_domain_registered",
            &role.canister_role,
            format!(
                "{} state domain(s) registered for {}",
                role.state.len(),
                role.canister_role
            ),
        ));
        checks.extend(domain_identity_checks(&role.canister_role, &role.state));
        checks.extend(memory_id_checks(
            &role.canister_role,
            &role.state,
            &role.removed_state,
        ));
        checks.extend(role_state_checks(&role.canister_role, &role.state));
        checks.extend(removed_state_checks(
            &role.canister_role,
            &role.removed_state,
        ));
        checks.extend(reserved_memory_checks(
            &role.canister_role,
            &role.state,
            &role.removed_state,
            &role.reserved_memory,
        ));
    }
    checks
}

fn role_identity_checks(roles: &[StateRoleManifest]) -> Vec<StateAuditCheck> {
    let mut by_role = BTreeMap::<&str, usize>::new();
    for role in roles {
        *by_role.entry(&role.canister_role).or_default() += 1;
    }

    by_role
        .into_iter()
        .filter(|&(_, count)| count > 1)
        .map(|(role, count)| {
            fail(
                CATEGORY_MANIFEST,
                "state_role_duplicate",
                role,
                format!("state role {role} is declared {count} times"),
                "declare each canister role once in the state manifest",
            )
        })
        .collect()
}

fn role_not_found_check(role: &str) -> StateAuditCheck {
    fail(
        CATEGORY_MANIFEST,
        "state_role_missing",
        role,
        format!("no state manifest role named {role} is declared"),
        "choose a declared role or omit --role to audit all declared roles",
    )
}

fn manifest_schema_checks(manifest: &StateManifest) -> Vec<StateAuditCheck> {
    if manifest.schema_version == STATE_MANIFEST_SCHEMA_VERSION {
        vec![pass(
            CATEGORY_SCHEMA_VERSION,
            "state_manifest_schema_version_supported",
            "state_manifest",
            format!(
                "manifest schema version {} is supported",
                manifest.schema_version
            ),
        )]
    } else {
        vec![fail(
            CATEGORY_SCHEMA_VERSION,
            "state_manifest_schema_version_unsupported",
            "state_manifest",
            format!(
                "manifest schema version {} is not supported by schema version {}",
                manifest.schema_version, STATE_MANIFEST_SCHEMA_VERSION
            ),
            "regenerate the manifest from current Rust state declarations",
        )]
    }
}

fn domain_identity_checks(role: &str, domains: &[StateDomainManifest]) -> Vec<StateAuditCheck> {
    let mut by_domain = BTreeMap::<&str, usize>::new();
    for domain in domains {
        *by_domain.entry(&domain.domain).or_default() += 1;
    }

    by_domain
        .into_iter()
        .filter(|&(_, count)| count > 1)
        .map(|(domain, count)| {
            fail(
                CATEGORY_MANIFEST,
                "state_domain_duplicate",
                &format!("{role}/{domain}"),
                format!("state domain {domain} is declared {count} times for role {role}"),
                "declare each state domain once per canister role",
            )
        })
        .collect()
}

fn memory_id_checks(
    role: &str,
    domains: &[StateDomainManifest],
    removed: &[RemovedStateManifest],
) -> Vec<StateAuditCheck> {
    let mut by_id = BTreeMap::<u8, Vec<&str>>::new();
    for domain in domains
        .iter()
        .filter(|domain| domain.storage == StateStorage::StableMemory)
    {
        if let Some(memory_id) = domain.memory_id {
            by_id.entry(memory_id).or_default().push(&domain.domain);
        }
    }

    let mut checks = Vec::new();
    let duplicates = by_id
        .iter()
        .filter(|(_, domains)| domains.len() > 1)
        .collect::<Vec<_>>();
    if duplicates.is_empty() {
        checks.push(pass(
            CATEGORY_MEMORY_ID,
            "memory_id_unique",
            role,
            format!("all stable-memory domains for {role} use unique memory IDs"),
        ));
    } else {
        checks.extend(duplicates.into_iter().map(|(memory_id, domains)| {
            fail(
                CATEGORY_MEMORY_ID,
                "memory_id_duplicate",
                &format!("{role}/memory_id/{memory_id}"),
                format!("memory id {memory_id} is used by {}", domains.join(", ")),
                "assign a unique memory id or add an explicit migration design",
            )
        }));
    }

    checks.extend(removed_memory_id_checks(role, &by_id, removed));
    checks
}

fn removed_memory_id_checks(
    role: &str,
    active_by_id: &BTreeMap<u8, Vec<&str>>,
    removed: &[RemovedStateManifest],
) -> Vec<StateAuditCheck> {
    removed
        .iter()
        .filter_map(|entry| {
            let memory_id = entry.memory_id?;
            let subject = format!("{role}/memory_id/{memory_id}");
            if let Some(active_domains) = active_by_id.get(&memory_id) {
                Some(fail(
                    CATEGORY_REMOVED_STATE,
                    "removed_state_memory_id_reclaimed",
                    &subject,
                    format!(
                        "removed state {} reserved memory id {memory_id}, but active domain(s) {} use it",
                        entry.domain,
                        active_domains.join(", ")
                    ),
                    "keep retired memory ids reserved or add an explicit migration design",
                ))
            } else {
                Some(pass(
                    CATEGORY_REMOVED_STATE,
                    "removed_state_memory_id_reserved",
                    &subject,
                    format!(
                        "removed state {} keeps retired memory id {memory_id} reserved",
                        entry.domain
                    ),
                ))
            }
        })
        .collect()
}

fn role_state_checks(role: &str, domains: &[StateDomainManifest]) -> Vec<StateAuditCheck> {
    let mut checks = Vec::new();
    for domain in domains {
        checks.extend(schema_checks(role, domain));
        checks.extend(storage_checks(role, domain));
        checks.extend(naming_checks(role, domain));
        checks.extend(export_import_contract_checks(role, domain));
        checks.extend(migration_checks(role, domain));
        checks.extend(lifecycle_checks(role, domain));
    }
    checks
}

fn schema_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let subject = domain_subject(role, domain);
    if domain.version == 0 {
        vec![fail(
            CATEGORY_SCHEMA_VERSION,
            "state_domain_missing_version",
            &subject,
            "state domain does not declare a positive schema version".to_string(),
            "declare the current schema version for this state domain",
        )]
    } else {
        vec![pass(
            CATEGORY_SCHEMA_VERSION,
            "state_domain_registered",
            &subject,
            format!("declares schema version {}", domain.version),
        )]
    }
}

fn storage_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let subject = domain_subject(role, domain);
    match (domain.storage, domain.memory_id) {
        (StateStorage::StableMemory, Some(memory_id)) => vec![pass(
            CATEGORY_MEMORY_ID,
            "memory_id_unique",
            &subject,
            format!("stable-memory domain declares memory id {memory_id}"),
        )],
        (StateStorage::StableMemory, None) => vec![fail(
            CATEGORY_MEMORY_ID,
            "state_domain_missing_memory_id",
            &subject,
            "stable-memory domain does not declare a memory id".to_string(),
            "declare the stable-memory id owned by this domain",
        )],
        (StateStorage::HeapOnly, None) => vec![pass(
            CATEGORY_MEMORY_ID,
            "state_domain_declares_no_stable_memory",
            &subject,
            "heap-only domain explicitly declares no stable memory".to_string(),
        )],
        (StateStorage::HeapOnly, Some(memory_id)) => vec![warn(
            CATEGORY_MEMORY_ID,
            "state_domain_declares_no_stable_memory",
            &subject,
            format!("heap-only domain also declares memory id {memory_id}"),
            "remove the memory id or change storage to stable_memory",
        )],
        (StateStorage::NotApplicable, None) => vec![pass(
            CATEGORY_MEMORY_ID,
            "state_domain_storage_not_applicable",
            &subject,
            "domain explicitly declares storage is not applicable".to_string(),
        )],
        (StateStorage::NotApplicable, Some(memory_id)) => vec![warn(
            CATEGORY_MEMORY_ID,
            "state_domain_storage_not_applicable",
            &subject,
            format!("storage-not-applicable domain also declares memory id {memory_id}"),
            "remove the memory id or choose a concrete storage substrate",
        )],
    }
}

fn naming_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let subject = domain_subject(role, domain);
    let record_check = if domain.record.ends_with("Record") {
        pass(
            CATEGORY_NAMING,
            "record_name_valid",
            &subject,
            format!(
                "record type {} follows the Record suffix convention",
                domain.record
            ),
        )
    } else {
        warn(
            CATEGORY_NAMING,
            "record_name_invalid",
            &subject,
            format!("record type {} does not end with Record", domain.record),
            "rename persisted records to use the Record suffix when safe",
        )
    };
    let snapshot_check = if domain.snapshot.ends_with("Data") {
        pass(
            CATEGORY_SNAPSHOT,
            "snapshot_name_valid",
            &subject,
            format!(
                "snapshot type {} follows the Data suffix convention",
                domain.snapshot
            ),
        )
    } else {
        warn(
            CATEGORY_SNAPSHOT,
            "snapshot_name_invalid",
            &subject,
            format!("snapshot type {} does not end with Data", domain.snapshot),
            "introduce canonical *Data snapshot types before relying on this domain for migration audits",
        )
    };

    vec![record_check, snapshot_check]
}

fn export_import_contract_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let subject = domain_subject(role, domain);
    if domain.snapshot.ends_with("Data") {
        vec![pass(
            CATEGORY_SNAPSHOT,
            "reserved_export_import_ok",
            &subject,
            format!(
                "snapshot type {} is a canonical Data shape for export/import boundaries",
                domain.snapshot
            ),
        )]
    } else {
        vec![warn(
            CATEGORY_SNAPSHOT,
            "reserved_export_import_violation",
            &subject,
            format!(
                "snapshot type {} is not a canonical Data shape for export/import boundaries",
                domain.snapshot
            ),
            "reserve export/import for canonical *Data snapshot shapes",
        )]
    }
}

fn migration_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let subject = domain_subject(role, domain);
    if domain.min_supported_version == 0 || domain.min_supported_version > domain.version {
        return vec![fail(
            CATEGORY_MIGRATION,
            "state_domain_invalid_support_window",
            &subject,
            format!(
                "min_supported_version {} is not valid for current version {}",
                domain.min_supported_version, domain.version
            ),
            "set min_supported_version to a positive version less than or equal to the current version",
        )];
    }

    if domain.min_supported_version == domain.version {
        return vec![pass(
            CATEGORY_MIGRATION,
            "migration_available",
            &subject,
            "no older supported schema version requires migration".to_string(),
        )];
    }

    match domain.migration_policy {
        MigrationPolicy::Migrate => {
            let mut checks = migration_declaration_checks(role, domain);
            checks.extend(migration_path_checks(role, domain));
            checks
        }
        MigrationPolicy::ManualMigrationRequired => vec![warn(
            CATEGORY_MIGRATION,
            "manual_migration_required_declared",
            &subject,
            format!(
                "manual migration is declared for supported versions {} through {}",
                domain.min_supported_version,
                domain.version.saturating_sub(1)
            ),
            "treat manual migration as a release gate when the old version is in production support",
        )],
        MigrationPolicy::DiscardDeclared => vec![pass(
            CATEGORY_MIGRATION,
            "migration_unsupported_declared",
            &subject,
            "old supported state is declared as discarded by policy".to_string(),
        )],
        MigrationPolicy::NotApplicable | MigrationPolicy::NewDomain => vec![fail(
            CATEGORY_MIGRATION,
            "migration_missing",
            &subject,
            "supported old versions exist but no migration policy can handle them".to_string(),
            "declare migration, manual migration, discard, or hard-cut min_supported_version",
        )],
    }
}

fn migration_declaration_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let mut checks = Vec::new();
    let mut by_edge = BTreeMap::<(u32, u32), usize>::new();

    for migration in &domain.migrations {
        *by_edge.entry((migration.from, migration.to)).or_default() += 1;
        let expected_next = migration.from.checked_add(1);
        if migration.from == 0
            || migration.to == 0
            || migration.from >= migration.to
            || expected_next != Some(migration.to)
            || migration.from < domain.min_supported_version
            || migration.to > domain.version
        {
            let subject = format!(
                "{}/{domain} v{} -> v{}",
                role,
                migration.from,
                migration.to,
                domain = domain.domain
            );
            checks.push(fail(
                CATEGORY_MIGRATION,
                "migration_declaration_invalid",
                &subject,
                format!(
                    "declared migration edge v{} -> v{} is outside the supported window {}..={}",
                    migration.from, migration.to, domain.min_supported_version, domain.version
                ),
                "declare only one-step migrations inside the supported version window",
            ));
        }
    }

    checks.extend(by_edge.into_iter().filter(|&(_, count)| count > 1).map(
        |((from, to), count)| {
            let subject = format!("{}/{domain} v{from} -> v{to}", role, domain = domain.domain);
            fail(
                CATEGORY_MIGRATION,
                "migration_declaration_duplicate",
                &subject,
                format!("migration edge v{from} -> v{to} is declared {count} times"),
                "declare each migration edge once",
            )
        },
    ));
    checks
}

fn migration_path_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let mut checks = Vec::new();
    for from in domain.min_supported_version..domain.version {
        let to = from + 1;
        let subject = format!("{}/{domain} v{from} -> v{to}", role, domain = domain.domain);
        match migration_for(domain, from, to) {
            Some(migration) => checks.push(migration_available_check(&subject, migration)),
            None => checks.push(fail(
                CATEGORY_MIGRATION,
                "migration_missing",
                &subject,
                format!("no declared migration covers v{from} -> v{to}"),
                "declare migration coverage or hard-cut min_supported_version",
            )),
        }
    }
    checks
}

fn migration_available_check(subject: &str, migration: &StateMigrationManifest) -> StateAuditCheck {
    if migration.test.is_some() {
        pass(
            CATEGORY_TEST_COVERAGE,
            "upgrade_test_declared",
            subject,
            format!(
                "migration {} declares upgrade test coverage",
                migration_label(migration)
            ),
        )
    } else {
        warn(
            CATEGORY_TEST_COVERAGE,
            "upgrade_test_missing",
            subject,
            format!(
                "migration {} has no declared upgrade test",
                migration_label(migration)
            ),
            "declare upgrade test coverage or hard-cut min_supported_version",
        )
    }
}

fn migration_for(
    domain: &StateDomainManifest,
    from: u32,
    to: u32,
) -> Option<&StateMigrationManifest> {
    domain
        .migrations
        .iter()
        .find(|migration| migration.from == from && migration.to == to)
}

fn lifecycle_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let subject = domain_subject(role, domain);
    let restore_check = if domain.restore_order.is_some() {
        pass(
            CATEGORY_LIFECYCLE,
            "restore_order_declared",
            &subject,
            "restore order is declared".to_string(),
        )
    } else {
        warn(
            CATEGORY_LIFECYCLE,
            "restore_order_missing",
            &subject,
            "restore order is not declared".to_string(),
            "declare lifecycle restore order before broad upgrade gating",
        )
    };
    let invariant_check = if domain.post_upgrade_invariant.is_some() {
        pass(
            CATEGORY_INVARIANT,
            "post_upgrade_invariant_declared",
            &subject,
            "post-upgrade invariant declaration is present".to_string(),
        )
    } else {
        warn(
            CATEGORY_INVARIANT,
            "post_upgrade_invariant_missing",
            &subject,
            "post-upgrade invariant declaration is missing".to_string(),
            "declare invariant checks or document why this domain is not applicable",
        )
    };

    vec![restore_check, invariant_check]
}

fn removed_state_checks(role: &str, removed: &[RemovedStateManifest]) -> Vec<StateAuditCheck> {
    removed
        .iter()
        .flat_map(|entry| {
            let subject = format!("{role}/{}", entry.domain);
            let mut checks = Vec::new();
            checks.push(if entry.disposition.trim().is_empty() {
                fail(
                    CATEGORY_REMOVED_STATE,
                    "removed_state_disposition_missing",
                    &subject,
                    "removed state does not declare a disposition".to_string(),
                    "declare whether the state is migrated, discarded, or manually handled",
                )
            } else {
                pass(
                    CATEGORY_REMOVED_STATE,
                    "removed_state_disposition_declared",
                    &subject,
                    format!("removed state disposition declared: {}", entry.disposition),
                )
            });
            checks.push(if entry.reason.trim().is_empty() {
                warn(
                    CATEGORY_REMOVED_STATE,
                    "removed_state_reason_missing",
                    &subject,
                    "removed state disposition does not declare a reason".to_string(),
                    "document why the removed state can be migrated, discarded, or manually handled",
                )
            } else {
                pass(
                    CATEGORY_REMOVED_STATE,
                    "removed_state_reason_declared",
                    &subject,
                    format!("removed state reason declared: {}", entry.reason),
                )
            });
            checks.push(
                if entry.test.as_ref().is_some_and(|test| !test.trim().is_empty()) {
                    pass(
                        CATEGORY_TEST_COVERAGE,
                        "removed_state_test_declared",
                        &subject,
                        "removed state declares upgrade test coverage".to_string(),
                    )
                } else {
                    warn(
                        CATEGORY_TEST_COVERAGE,
                        "removed_state_test_missing",
                        &subject,
                        "removed state has no declared upgrade test coverage".to_string(),
                        "declare upgrade test coverage for the removed-state disposition",
                    )
                },
            );
            checks
        })
        .collect()
}

fn reserved_memory_checks(
    role: &str,
    domains: &[StateDomainManifest],
    removed: &[RemovedStateManifest],
    reserved: &[ReservedMemoryManifest],
) -> Vec<StateAuditCheck> {
    let active_by_id = active_memory_ids(domains);
    let removed_by_id = removed_memory_ids(removed);
    let mut reserved_by_id = BTreeMap::<u8, Vec<&str>>::new();
    for entry in reserved {
        reserved_by_id
            .entry(entry.memory_id)
            .or_default()
            .push(&entry.label);
    }

    let mut checks = Vec::new();
    for entry in reserved {
        let subject = format!("{role}/memory_id/{}", entry.memory_id);
        if let Some(active_domains) = active_by_id.get(&entry.memory_id) {
            checks.push(fail(
                CATEGORY_MEMORY_ID,
                "reserved_memory_id_collision",
                &subject,
                format!(
                    "reserved memory id {} for {} is used by active domain(s) {}",
                    entry.memory_id,
                    entry.label,
                    active_domains.join(", ")
                ),
                "declare one owner for the memory id or add an explicit migration design",
            ));
        } else if let Some(removed_domains) = removed_by_id.get(&entry.memory_id) {
            checks.push(fail(
                CATEGORY_MEMORY_ID,
                "reserved_memory_id_collision",
                &subject,
                format!(
                    "reserved memory id {} for {} is already declared by removed state {}",
                    entry.memory_id,
                    entry.label,
                    removed_domains.join(", ")
                ),
                "keep the memory id in exactly one manifest section",
            ));
        } else if reserved_by_id
            .get(&entry.memory_id)
            .is_some_and(|labels| labels.len() > 1)
        {
            checks.push(fail(
                CATEGORY_MEMORY_ID,
                "reserved_memory_id_duplicate",
                &subject,
                format!(
                    "reserved memory id {} is listed for {}",
                    entry.memory_id,
                    reserved_by_id
                        .get(&entry.memory_id)
                        .map(|labels| labels.join(", "))
                        .unwrap_or_default()
                ),
                "reserve each memory id once",
            ));
        } else {
            checks.push(warn(
                CATEGORY_MEMORY_ID,
                "reserved_memory_id_declared",
                &subject,
                format!(
                    "memory id {} is reserved for {} but is not yet modeled as an active or removed state domain",
                    entry.memory_id, entry.label
                ),
                "model this reservation as a precise state domain or removed-state disposition when the state shape is known",
            ));
        }
    }
    checks
}

fn active_memory_ids(domains: &[StateDomainManifest]) -> BTreeMap<u8, Vec<&str>> {
    let mut by_id = BTreeMap::<u8, Vec<&str>>::new();
    for domain in domains
        .iter()
        .filter(|domain| domain.storage == StateStorage::StableMemory)
    {
        if let Some(memory_id) = domain.memory_id {
            by_id.entry(memory_id).or_default().push(&domain.domain);
        }
    }
    by_id
}

fn removed_memory_ids(removed: &[RemovedStateManifest]) -> BTreeMap<u8, Vec<&str>> {
    let mut by_id = BTreeMap::<u8, Vec<&str>>::new();
    for entry in removed {
        if let Some(memory_id) = entry.memory_id {
            by_id.entry(memory_id).or_default().push(&entry.domain);
        }
    }
    by_id
}

fn domain_subject(role: &str, domain: &StateDomainManifest) -> String {
    format!("{role}/{}", domain.domain)
}

fn migration_label(migration: &StateMigrationManifest) -> String {
    migration
        .name
        .clone()
        .unwrap_or_else(|| format!("{}->{}", migration.from, migration.to))
}

fn pass(
    category: &'static str,
    code: &'static str,
    subject: &str,
    detail: String,
) -> StateAuditCheck {
    StateAuditCheck {
        category,
        code,
        status: StateAuditStatus::Pass,
        severity: StateAuditSeverity::Info,
        subject: subject.to_string(),
        detail,
        next: None,
        source: SOURCE_STATE_MANIFEST,
    }
}

fn warn(
    category: &'static str,
    code: &'static str,
    subject: &str,
    detail: String,
    next: &str,
) -> StateAuditCheck {
    StateAuditCheck {
        category,
        code,
        status: StateAuditStatus::Warn,
        severity: StateAuditSeverity::Warning,
        subject: subject.to_string(),
        detail,
        next: Some(next.to_string()),
        source: SOURCE_STATE_MANIFEST,
    }
}

fn fail(
    category: &'static str,
    code: &'static str,
    subject: &str,
    detail: String,
    next: &str,
) -> StateAuditCheck {
    StateAuditCheck {
        category,
        code,
        status: StateAuditStatus::Fail,
        severity: StateAuditSeverity::Blocked,
        subject: subject.to_string(),
        detail,
        next: Some(next.to_string()),
        source: SOURCE_STATE_MANIFEST,
    }
}

fn aggregate_status(checks: &[StateAuditCheck]) -> StateAuditStatus {
    if checks.is_empty() {
        return StateAuditStatus::NotEvaluated;
    }
    if checks
        .iter()
        .any(|check| check.status == StateAuditStatus::Fail)
    {
        return StateAuditStatus::Fail;
    }
    if checks
        .iter()
        .any(|check| check.status == StateAuditStatus::Warn)
    {
        return StateAuditStatus::Warn;
    }
    StateAuditStatus::Pass
}

fn next_actions(status: StateAuditStatus, role: Option<&str>) -> Vec<String> {
    let scope = role.unwrap_or("project");
    match status {
        StateAuditStatus::Pass => vec![format!(
            "state metadata declarations for {scope} have no blocking findings"
        )],
        StateAuditStatus::Warn => vec![format!(
            "review warning checks before using {scope} state metadata as an upgrade gate"
        )],
        StateAuditStatus::Fail => vec![format!(
            "fix failing state metadata checks before upgrade or release gating for {scope}"
        )],
        StateAuditStatus::NotEvaluated => vec![format!(
            "declare state metadata before auditing {scope} upgrade safety"
        )],
    }
}

fn sort_checks(checks: &mut [StateAuditCheck]) {
    checks.sort_by(|left, right| {
        (
            status_rank(left.status),
            left.category,
            left.code,
            left.subject.as_str(),
        )
            .cmp(&(
                status_rank(right.status),
                right.category,
                right.code,
                right.subject.as_str(),
            ))
    });
}

const fn status_rank(status: StateAuditStatus) -> u8 {
    match status {
        StateAuditStatus::Fail => 0,
        StateAuditStatus::Warn => 1,
        StateAuditStatus::NotEvaluated => 2,
        StateAuditStatus::Pass => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::state_contract::{MigrationPolicy, StateRoleManifest};

    #[test]
    fn builtin_report_is_warning_only_for_reserved_memory() {
        let report = build_state_audit_report(Some("root"));

        assert_eq!(report.status, StateAuditStatus::Warn);
        assert!(report.checks.iter().any(|check| {
            check.code == "state_manifest_schema_version_supported"
                && check.status == StateAuditStatus::Pass
        }));
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.code == "reserved_memory_id_declared")
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
        let manifest = declared_state_manifest(Some("root"));
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
    fn wasm_store_role_audits_cleanly() {
        let report = build_state_audit_report(Some("wasm_store"));

        assert_eq!(report.status, StateAuditStatus::Pass);
        assert!(report.manifest.roles.iter().any(|role| {
            role.canister_role == "wasm_store"
                && role
                    .state
                    .iter()
                    .any(|domain| domain.domain == "wasm_store_gc_state")
        }));
    }

    #[test]
    fn unsupported_manifest_schema_version_fails() {
        let mut manifest = declared_state_manifest(Some("root"));
        manifest.schema_version = STATE_MANIFEST_SCHEMA_VERSION + 1;

        let checks = audit_checks(&manifest, Some("root"));

        assert!(checks.iter().any(|check| {
            check.code == "state_manifest_schema_version_unsupported"
                && check.status == StateAuditStatus::Fail
        }));
    }

    #[test]
    fn duplicate_state_role_fails() {
        let mut manifest = declared_state_manifest(Some("root"));
        let duplicate = manifest.roles[0].clone();
        manifest.roles.push(duplicate);

        let checks = audit_checks(&manifest, None);

        assert!(checks.iter().any(|check| {
            check.code == "state_role_duplicate" && check.status == StateAuditStatus::Fail
        }));
    }

    #[test]
    fn duplicate_memory_id_fails_within_role() {
        let mut manifest = declared_state_manifest(Some("root"));
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
        let mut manifest = declared_state_manifest(Some("root"));
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
    fn active_domain_reclaiming_removed_memory_id_fails() {
        let mut manifest = declared_state_manifest(Some("root"));
        let role = manifest.roles.first_mut().expect("root role");
        let retired_id = role
            .removed_state
            .first()
            .and_then(|entry| entry.memory_id)
            .expect("retired memory id");
        role.state[0].memory_id = Some(retired_id);

        let checks = audit_checks(&manifest, Some("root"));

        assert!(
            checks
                .iter()
                .any(|check| check.code == "removed_state_memory_id_reclaimed"
                    && check.status == StateAuditStatus::Fail)
        );
    }

    #[test]
    fn reserved_memory_ids_warn_until_modeled() {
        let report = build_state_audit_report(Some("root"));

        assert!(report.checks.iter().any(|check| {
            check.code == "reserved_memory_id_declared" && check.status == StateAuditStatus::Warn
        }));
    }

    #[test]
    fn active_domain_reclaiming_reserved_memory_id_fails() {
        let mut manifest = declared_state_manifest(Some("root"));
        let role = manifest.roles.first_mut().expect("root role");
        let reserved_id = role
            .reserved_memory
            .first()
            .map(|entry| entry.memory_id)
            .expect("reserved memory id");
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
            schema_version: 1,
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
                removed_state: Vec::new(),
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
            schema_version: 1,
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
                removed_state: Vec::new(),
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
            schema_version: 1,
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
                removed_state: Vec::new(),
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
            schema_version: 1,
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
                removed_state: Vec::new(),
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
            schema_version: 1,
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
                removed_state: Vec::new(),
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
    fn removed_state_missing_reason_and_test_warn() {
        let manifest = StateManifest {
            schema_version: 1,
            roles: vec![StateRoleManifest {
                canister_role: "root".to_string(),
                state: Vec::new(),
                removed_state: vec![RemovedStateManifest {
                    domain: "legacy_cache".to_string(),
                    last_version: 1,
                    removed_in_version: 2,
                    memory_id: Some(99),
                    disposition: "discarded".to_string(),
                    reason: String::new(),
                    test: None,
                }],
                reserved_memory: Vec::new(),
            }],
        };

        let checks = audit_checks(&manifest, None);

        assert!(checks.iter().any(|check| {
            check.code == "removed_state_reason_missing" && check.status == StateAuditStatus::Warn
        }));
        assert!(checks.iter().any(|check| {
            check.code == "removed_state_test_missing" && check.status == StateAuditStatus::Warn
        }));
    }

    #[test]
    fn unknown_filtered_role_fails() {
        let report = build_state_audit_report(Some("missing"));

        assert_eq!(report.status, StateAuditStatus::Fail);
        assert_eq!(report.checks[0].code, "state_role_missing");
    }
}
