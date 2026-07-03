//! Module: state_manifest
//!
//! Responsibility: build host-side state manifest and audit reports from
//! Rust-authored Canic state declarations.
//! Does not own: stable-memory inspection, migration execution, CLI parsing, or
//! runtime introspection.
//! Boundary: consumes passive declaration metadata from `canic-core` and emits
//! diagnostic-only reports.

use canic_core::state_contract::{
    MigrationPolicy, StateDomainManifest, StateManifest, StateMigrationManifest, StateStorage,
    canic_state_manifest_for_role,
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
    canic_state_manifest_for_role(role)
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
    if manifest.roles.is_empty() {
        return role_filter
            .map(role_not_found_check)
            .into_iter()
            .collect::<Vec<_>>();
    }

    let mut checks = Vec::new();
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
        checks.extend(memory_id_checks(&role.canister_role, &role.state));
        checks.extend(role_state_checks(&role.canister_role, &role.state));
        checks.extend(removed_state_checks(
            &role.canister_role,
            &role.removed_state,
        ));
    }
    checks
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

    let duplicates = by_id
        .iter()
        .filter(|(_, domains)| domains.len() > 1)
        .collect::<Vec<_>>();
    if duplicates.is_empty() {
        return vec![pass(
            CATEGORY_MEMORY_ID,
            "memory_id_unique",
            role,
            format!("all stable-memory domains for {role} use unique memory IDs"),
        )];
    }

    duplicates
        .into_iter()
        .map(|(memory_id, domains)| {
            fail(
                CATEGORY_MEMORY_ID,
                "memory_id_duplicate",
                &format!("{role}/memory_id/{memory_id}"),
                format!("memory id {memory_id} is used by {}", domains.join(", ")),
                "assign a unique memory id or add an explicit migration design",
            )
        })
        .collect()
}

fn role_state_checks(role: &str, domains: &[StateDomainManifest]) -> Vec<StateAuditCheck> {
    let mut checks = Vec::new();
    for domain in domains {
        checks.extend(schema_checks(role, domain));
        checks.extend(storage_checks(role, domain));
        checks.extend(naming_checks(role, domain));
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

fn migration_checks(role: &str, domain: &StateDomainManifest) -> Vec<StateAuditCheck> {
    let subject = domain_subject(role, domain);
    if domain.min_supported_version >= domain.version {
        return vec![pass(
            CATEGORY_MIGRATION,
            "migration_available",
            &subject,
            "no older supported schema version requires migration".to_string(),
        )];
    }

    match domain.migration_policy {
        MigrationPolicy::Migrate => migration_path_checks(role, domain),
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

fn removed_state_checks(
    role: &str,
    removed: &[canic_core::state_contract::RemovedStateManifest],
) -> Vec<StateAuditCheck> {
    removed
        .iter()
        .map(|entry| {
            let subject = format!("{role}/{}", entry.domain);
            if entry.disposition.trim().is_empty() {
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
            }
        })
        .collect()
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
    fn builtin_report_is_warning_only_for_snapshot_names() {
        let report = build_state_audit_report(Some("root"));

        assert_eq!(report.status, StateAuditStatus::Warn);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.code == "snapshot_name_invalid")
        );
        assert!(
            report
                .checks
                .iter()
                .all(|check| check.status != StateAuditStatus::Fail)
        );
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
