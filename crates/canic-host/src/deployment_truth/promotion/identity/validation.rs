use super::group::{
    artifact_identity_key_for_group, artifact_identity_key_for_role,
    promotion_artifact_identity_summary,
};
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionArtifactIdentityGroupV1,
    PromotionArtifactIdentityReportV1, PromotionReadinessStatusV1, RolePromotionArtifactIdentityV1,
    SafetyFindingV1, SafetySeverityV1,
};
use std::collections::BTreeSet;

use super::super::digest::promotion_artifact_identity_report_digest;
use super::super::ensure::{
    ensure_identity_optional_sha256, ensure_identity_report_field, ensure_identity_report_sha256,
};
use super::super::error::PromotionArtifactIdentityReportError;

pub fn validate_promotion_artifact_identity_report(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionArtifactIdentityReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_identity_report_field("report_id", &report.report_id)?;
    ensure_identity_report_sha256(
        "artifact_identity_report_digest",
        &report.artifact_identity_report_digest,
    )?;
    ensure_identity_report_status_matches_blockers(report)?;
    ensure_unique_artifact_identity_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_artifact_identity(role)?;
    }
    validate_artifact_identity_groups(&report.roles, &report.identity_groups)?;
    validate_artifact_identity_summary(report)?;
    validate_identity_report_blockers(&report.blockers)?;
    if report.artifact_identity_report_digest != promotion_artifact_identity_report_digest(report) {
        return Err(PromotionArtifactIdentityReportError::LinkageMismatch {
            field: "artifact_identity_report_digest",
        });
    }
    Ok(())
}

fn validate_role_artifact_identity(
    role: &RolePromotionArtifactIdentityV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("role", &role.role)?;
    ensure_identity_optional_sha256("wasm_sha256", role.wasm_sha256.as_deref())?;
    ensure_identity_optional_sha256("wasm_gz_sha256", role.wasm_gz_sha256.as_deref())?;
    ensure_identity_optional_sha256("candid_sha256", role.candid_sha256.as_deref())?;
    ensure_identity_optional_sha256(
        "canonical_embedded_config_sha256",
        role.canonical_embedded_config_sha256.as_deref(),
    )?;
    Ok(())
}

fn validate_artifact_identity_groups(
    roles: &[RolePromotionArtifactIdentityV1],
    groups: &[PromotionArtifactIdentityGroupV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut grouped_roles = BTreeSet::new();
    let mut group_keys = BTreeSet::new();
    for group in groups {
        validate_artifact_identity_group(group)?;
        if !group_keys.insert(group.identity_key.as_str()) {
            return Err(
                PromotionArtifactIdentityReportError::DuplicateIdentityGroup {
                    identity_key: group.identity_key.clone(),
                },
            );
        }
        if group.roles.is_empty() {
            return Err(PromotionArtifactIdentityReportError::EmptyIdentityGroup {
                identity_key: group.identity_key.clone(),
            });
        }
        for role in &group.roles {
            if !role_names.contains(role.as_str()) {
                return Err(PromotionArtifactIdentityReportError::UnknownGroupedRole {
                    role: role.clone(),
                });
            }
            if !grouped_roles.insert(role.as_str()) {
                return Err(PromotionArtifactIdentityReportError::DuplicateGroupedRole {
                    role: role.clone(),
                });
            }
            let role_identity = roles
                .iter()
                .find(|candidate| candidate.role == *role)
                .expect("known role should be present");
            let expected = artifact_identity_key_for_role(role_identity);
            if expected != group.identity_key {
                return Err(
                    PromotionArtifactIdentityReportError::IdentityGroupRoleMismatch {
                        role: role.clone(),
                        expected,
                        found: group.identity_key.clone(),
                    },
                );
            }
        }
    }
    for role in roles {
        if !grouped_roles.contains(role.role.as_str()) {
            return Err(PromotionArtifactIdentityReportError::MissingGroupedRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_artifact_identity_summary(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    let expected = promotion_artifact_identity_summary(&report.roles, &report.identity_groups);
    if report.summary.role_count != expected.role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "role_count",
        });
    }
    if report.summary.identity_group_count != expected.identity_group_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "identity_group_count",
        });
    }
    if report.summary.shared_identity_group_count != expected.shared_identity_group_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "shared_identity_group_count",
        });
    }
    if report.summary.digest_pinned_role_count != expected.digest_pinned_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "digest_pinned_role_count",
        });
    }
    if report.summary.source_build_role_count != expected.source_build_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "source_build_role_count",
        });
    }
    if report.summary.deferred_identity_role_count != expected.deferred_identity_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "deferred_identity_role_count",
        });
    }
    Ok(())
}

fn validate_artifact_identity_group(
    group: &PromotionArtifactIdentityGroupV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("identity_group.identity_key", &group.identity_key)?;
    if group.source_kinds.is_empty() {
        return Err(PromotionArtifactIdentityReportError::MissingRequiredField {
            field: "identity_group.source_kinds",
        });
    }
    ensure_identity_optional_sha256("identity_group.wasm_sha256", group.wasm_sha256.as_deref())?;
    ensure_identity_optional_sha256(
        "identity_group.wasm_gz_sha256",
        group.wasm_gz_sha256.as_deref(),
    )?;
    ensure_identity_optional_sha256(
        "identity_group.candid_sha256",
        group.candid_sha256.as_deref(),
    )?;
    ensure_identity_optional_sha256(
        "identity_group.canonical_embedded_config_sha256",
        group.canonical_embedded_config_sha256.as_deref(),
    )?;
    let expected = artifact_identity_key_for_group(group);
    if expected != group.identity_key {
        return Err(
            PromotionArtifactIdentityReportError::IdentityGroupKeyMismatch {
                expected,
                found: group.identity_key.clone(),
            },
        );
    }
    Ok(())
}

const fn ensure_identity_report_status_matches_blockers(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionArtifactIdentityReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_artifact_identity_roles(
    roles: &[RolePromotionArtifactIdentityV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionArtifactIdentityReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_identity_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    for blocker in blockers {
        ensure_identity_report_field("blocker.code", &blocker.code)?;
        ensure_identity_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionArtifactIdentityReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}
