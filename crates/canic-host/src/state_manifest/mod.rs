//! Module: state_manifest
//!
//! Responsibility: build host-side state manifest and audit reports from
//! Rust-authored Canic state declarations.
//! Does not own: stable-memory inspection, migration execution, CLI parsing, or
//! runtime introspection.
//! Boundary: consumes passive declaration metadata from `canic-core` and emits
//! diagnostic-only reports.

mod resolution;

pub use resolution::{StateManifestResolution, resolve_project_state_manifest};

use crate::role_contract::finding_detail;
use canic_core::{
    role_contract::RoleContractFinding,
    state_contract::{
        MigrationPolicy, ReservedMemoryManifest, STATE_MANIFEST_SCHEMA_VERSION,
        StateDomainManifest, StateManifest, StateMigrationManifest, StateRoleManifest,
        StateStorage,
    },
};
use serde::Serialize;
use std::collections::BTreeMap;

pub const STATE_AUDIT_COMMAND: &str = "canic state audit";
pub const STATE_MANIFEST_COMMAND: &str = "canic state manifest";
pub const STATE_AUDIT_SCHEMA_VERSION: u16 = 2;

const SCOPE_PROJECT: StateAuditScope = StateAuditScope::Project;
const SCOPE_ROLE: StateAuditScope = StateAuditScope::Role;
const SOURCE_STATE_MANIFEST: StateAuditSource = StateAuditSource::StateManifest;
const CATEGORY_MANIFEST: StateAuditCategory = StateAuditCategory::Manifest;
const CATEGORY_SCHEMA_VERSION: StateAuditCategory = StateAuditCategory::SchemaVersion;
const CATEGORY_MEMORY_ID: StateAuditCategory = StateAuditCategory::MemoryId;
const CATEGORY_MIGRATION: StateAuditCategory = StateAuditCategory::Migration;
const CATEGORY_SNAPSHOT: StateAuditCategory = StateAuditCategory::Snapshot;
const CATEGORY_NAMING: StateAuditCategory = StateAuditCategory::Naming;
const CATEGORY_LIFECYCLE: StateAuditCategory = StateAuditCategory::Lifecycle;
const CATEGORY_INVARIANT: StateAuditCategory = StateAuditCategory::Invariant;
const CATEGORY_TEST_COVERAGE: StateAuditCategory = StateAuditCategory::TestCoverage;

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
    let mut checks = audit_checks(
        &manifest,
        if contract_errors.is_empty() {
            role
        } else {
            None
        },
    );
    checks.extend(contract_errors.iter().map(role_contract_check));
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

fn role_contract_check(finding: &RoleContractFinding) -> StateAuditCheck {
    fail(
        CATEGORY_MANIFEST,
        finding.code(),
        "role_contract",
        finding_detail(finding),
        "repair the role package contract, then rerun canic state audit",
    )
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
        checks.extend(memory_id_checks(&role.canister_role, &role.state));
        checks.extend(role_state_checks(&role.canister_role, &role.state));
        checks.extend(reserved_memory_checks(
            &role.canister_role,
            &role.state,
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

fn memory_id_checks(role: &str, domains: &[StateDomainManifest]) -> Vec<StateAuditCheck> {
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

    checks
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

fn reserved_memory_checks(
    role: &str,
    domains: &[StateDomainManifest],
    reserved: &[ReservedMemoryManifest],
) -> Vec<StateAuditCheck> {
    let active_by_id = active_memory_ids(domains);
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
                    "memory id {} is reserved for {} but is not yet modeled as an active state domain",
                    entry.memory_id, entry.label
                ),
                "model this reservation as a precise state domain when the state shape is known",
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
    category: StateAuditCategory,
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
    category: StateAuditCategory,
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
    category: StateAuditCategory,
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
    use crate::role_contract::materialize_state_manifest;
    use canic_core::{
        ids::CanisterRole,
        role_contract::{
            AllocationOwner, BuiltInRoleKind, CanicFeatureKey, ResolvedRoleContract,
            ResolvedStateAllocation, SelectionProvenance, StateAllocationKey,
            allocation::allocation_definition,
        },
        state_contract::{MigrationPolicy, StateRoleManifest},
    };
    use std::{collections::BTreeSet, path::PathBuf};

    fn test_state_manifest(role: Option<&str>) -> StateManifest {
        let contracts = match role {
            Some("root") | None => vec![test_contract(
                "root",
                None,
                &[
                    StateAllocationKey::CoreRuntimeTopology,
                    StateAllocationKey::CoreRootAppRegistry,
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
            ("fleets/test/canic.toml", "user_hub", vec![49, 53, 54, 56]),
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
                11, 12, 13, 15, 16, 17, 18, 20, 30, 34, 39, 40, 41, 42, 80, 81, 82, 83, 85,
            ]
        );
        assert_eq!(
            manifest.roles[0]
                .reserved_memory
                .iter()
                .map(|entry| entry.memory_id)
                .collect::<Vec<_>>(),
            vec![29, 31, 32]
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
    fn reserved_memory_ids_warn_until_modeled() {
        let report = build_state_audit_report(Some("root"));

        assert!(report.checks.iter().any(|check| {
            check.code == "reserved_memory_id_declared" && check.status == StateAuditStatus::Warn
        }));
    }

    #[test]
    fn active_domain_reclaiming_reserved_memory_id_fails() {
        let mut manifest = test_state_manifest(Some("root"));
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
