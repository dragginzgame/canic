use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionMaterializationIdentityReportV1,
    PromotionMaterializationOutputGroupV1, PromotionReadinessStatusV1,
    RolePromotionMaterializationIdentityV1, SafetyFindingV1, SafetySeverityV1,
};
use std::collections::BTreeSet;

use super::super::digest::promotion_materialization_identity_report_digest;
use super::super::ensure::{
    ensure_materialization_report_field, ensure_materialization_report_sha256,
};
use super::super::error::PromotionMaterializationIdentityReportError;
use super::super::identity::{
    materialization_output_key_for_group, materialization_output_key_for_role,
};

pub fn validate_promotion_materialization_identity_report(
    report: &PromotionMaterializationIdentityReportV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionMaterializationIdentityReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_materialization_report_field("report_id", &report.report_id)?;
    ensure_materialization_report_sha256(
        "materialization_identity_report_digest",
        &report.materialization_identity_report_digest,
    )?;
    ensure_materialization_report_status_matches_blockers(report)?;
    ensure_unique_materialization_report_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_materialization_identity(role)?;
    }
    validate_materialization_output_groups(&report.roles, &report.output_groups)?;
    let expected_blockers = Vec::<SafetyFindingV1>::new();
    if report.blockers != expected_blockers {
        return Err(PromotionMaterializationIdentityReportError::BlockerMismatch);
    }
    validate_materialization_report_blockers(&report.blockers)?;
    if report.materialization_identity_report_digest
        != promotion_materialization_identity_report_digest(report)
    {
        return Err(
            PromotionMaterializationIdentityReportError::LinkageMismatch {
                field: "materialization_identity_report_digest",
            },
        );
    }
    Ok(())
}

fn validate_materialization_output_groups(
    roles: &[RolePromotionMaterializationIdentityV1],
    groups: &[PromotionMaterializationOutputGroupV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut grouped_roles = BTreeSet::new();
    let mut group_keys = BTreeSet::new();
    for group in groups {
        validate_materialization_output_group(group)?;
        if !group_keys.insert(group.output_identity_key.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::DuplicateOutputGroup {
                    output_identity_key: group.output_identity_key.clone(),
                },
            );
        }
        if group.roles.is_empty() {
            return Err(
                PromotionMaterializationIdentityReportError::EmptyOutputGroup {
                    output_identity_key: group.output_identity_key.clone(),
                },
            );
        }
        for role in &group.roles {
            if !role_names.contains(role.as_str()) {
                return Err(
                    PromotionMaterializationIdentityReportError::UnknownGroupedRole {
                        role: role.clone(),
                    },
                );
            }
            if !grouped_roles.insert(role.as_str()) {
                return Err(
                    PromotionMaterializationIdentityReportError::DuplicateGroupedRole {
                        role: role.clone(),
                    },
                );
            }
            let role_identity = roles
                .iter()
                .find(|candidate| candidate.role == *role)
                .expect("known role should be present");
            let expected = materialization_output_key_for_role(role_identity);
            if expected != group.output_identity_key {
                return Err(
                    PromotionMaterializationIdentityReportError::OutputGroupRoleMismatch {
                        role: role.clone(),
                        expected,
                        found: group.output_identity_key.clone(),
                    },
                );
            }
        }
    }
    for role in roles {
        if !grouped_roles.contains(role.role.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::MissingGroupedRole {
                    role: role.role.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_materialization_output_group(
    group: &PromotionMaterializationOutputGroupV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field(
        "output_group.output_identity_key",
        &group.output_identity_key,
    )?;
    ensure_materialization_report_sha256("output_group.wasm_sha256", &group.wasm_sha256)?;
    ensure_materialization_report_sha256("output_group.wasm_gz_sha256", &group.wasm_gz_sha256)?;
    ensure_materialization_report_sha256(
        "output_group.installed_module_hash",
        &group.installed_module_hash,
    )?;
    ensure_materialization_report_sha256("output_group.candid_sha256", &group.candid_sha256)?;
    let expected = materialization_output_key_for_group(group);
    if expected != group.output_identity_key {
        return Err(
            PromotionMaterializationIdentityReportError::OutputGroupKeyMismatch {
                expected,
                found: group.output_identity_key.clone(),
            },
        );
    }
    Ok(())
}

const fn ensure_materialization_report_status_matches_blockers(
    report: &PromotionMaterializationIdentityReportV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionMaterializationIdentityReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_materialization_report_roles(
    roles: &[RolePromotionMaterializationIdentityV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    let mut seen_roles = BTreeSet::new();
    let mut seen_evidence = BTreeSet::new();
    for role in roles {
        if !seen_roles.insert(role.role.as_str()) {
            return Err(PromotionMaterializationIdentityReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
        if !seen_evidence.insert(role.evidence_id.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::DuplicateEvidence {
                    evidence_id: role.evidence_id.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_role_materialization_identity(
    role: &RolePromotionMaterializationIdentityV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field("role", &role.role)?;
    ensure_materialization_report_field("evidence_id", &role.evidence_id)?;
    ensure_materialization_report_sha256(
        "materialization_evidence_digest",
        &role.materialization_evidence_digest,
    )?;
    ensure_materialization_report_field("recipe_id", &role.recipe_id)?;
    ensure_materialization_report_field(
        "materialization_input_id",
        &role.materialization_input_id,
    )?;
    ensure_materialization_report_field(
        "materialization_result_id",
        &role.materialization_result_id,
    )?;
    ensure_materialization_report_sha256(
        "materialization_input_digest",
        &role.materialization_input_digest,
    )?;
    ensure_materialization_report_sha256(
        "canonical_embedded_config_sha256",
        &role.canonical_embedded_config_sha256,
    )?;
    ensure_materialization_report_field("environment", &role.environment)?;
    ensure_materialization_report_field("root_trust_anchor", &role.root_trust_anchor)?;
    ensure_materialization_report_field("runtime_variant", &role.runtime_variant)?;
    ensure_materialization_report_sha256("wasm_sha256", &role.wasm_sha256)?;
    ensure_materialization_report_sha256("wasm_gz_sha256", &role.wasm_gz_sha256)?;
    ensure_materialization_report_sha256("installed_module_hash", &role.installed_module_hash)?;
    ensure_materialization_report_sha256("candid_sha256", &role.candid_sha256)?;
    Ok(())
}

fn validate_materialization_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    for blocker in blockers {
        ensure_materialization_report_field("blocker.code", &blocker.code)?;
        ensure_materialization_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionMaterializationIdentityReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}
