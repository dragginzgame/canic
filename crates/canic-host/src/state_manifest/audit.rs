//! Module: state_manifest::audit
//!
//! Responsibility: construct deterministic state-manifest audit checks from
//! resolved manifest and role-contract evidence.
//! Does not own: package resolution, report aggregation, next actions, or
//! rendering.
//! Boundary: consumes passive manifest declarations and emits typed audit
//! checks without side effects.

use super::{StateAuditCategory, StateAuditCheck, StateAuditSeverity, StateAuditStatus};
use crate::role_contract::finding_detail;
use canic_core::{
    role_contract::RoleContractFinding,
    state_contract::{
        ReservedMemoryManifest, STATE_MANIFEST_SCHEMA_VERSION, StateDomainManifest, StateManifest,
        StateRoleManifest, StateStorage,
    },
};
use std::collections::BTreeMap;

const SOURCE_STATE_MANIFEST: super::StateAuditSource = super::StateAuditSource::StateManifest;
const CATEGORY_MANIFEST: StateAuditCategory = StateAuditCategory::Manifest;
const CATEGORY_SCHEMA_VERSION: StateAuditCategory = StateAuditCategory::SchemaVersion;
const CATEGORY_MEMORY_ID: StateAuditCategory = StateAuditCategory::MemoryId;
const CATEGORY_SNAPSHOT: StateAuditCategory = StateAuditCategory::Snapshot;
const CATEGORY_NAMING: StateAuditCategory = StateAuditCategory::Naming;
const CATEGORY_LIFECYCLE: StateAuditCategory = StateAuditCategory::Lifecycle;
const CATEGORY_INVARIANT: StateAuditCategory = StateAuditCategory::Invariant;

pub(super) fn role_contract_check(finding: &RoleContractFinding) -> StateAuditCheck {
    fail(
        CATEGORY_MANIFEST,
        finding.code(),
        "role_contract",
        finding_detail(finding),
        "repair the role package contract, then rerun canic state audit",
    )
}

pub(super) fn audit_checks(
    manifest: &StateManifest,
    role_filter: Option<&str>,
) -> Vec<StateAuditCheck> {
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
                "assign one unique owner to each memory id",
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
            "introduce canonical *Data snapshot types before relying on this domain for snapshot or restore audits",
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
            "declare lifecycle restore order for same-release restore",
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
                "declare one owner for the memory id",
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
