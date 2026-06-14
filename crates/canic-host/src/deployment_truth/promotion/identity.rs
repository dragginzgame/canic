use super::super::{
    BuildMaterializationEvidenceV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
    PromotionArtifactIdentityGroupV1, PromotionArtifactIdentityKindV1,
    PromotionArtifactIdentityReportV1, PromotionArtifactIdentitySummaryV1,
    PromotionArtifactLevelV1, PromotionMaterializationOutputGroupV1, PromotionReadinessStatusV1,
    RoleArtifactSourceKindV1, RoleArtifactSourceV1, RoleArtifactV1,
    RolePromotionArtifactIdentityV1, RolePromotionInputV1, RolePromotionMaterializationIdentityV1,
    RolePromotionPlanTransformV1, SafetyFindingV1, SafetySeverityV1,
};
use super::digest::promotion_artifact_identity_report_digest;
use super::ensure::{
    ensure_identity_optional_sha256, ensure_identity_report_field, ensure_identity_report_sha256,
};
use super::error::PromotionArtifactIdentityReportError;
use super::request::PromotionArtifactIdentityReportRequest;
use std::collections::{BTreeMap, BTreeSet};

pub fn promotion_artifact_identity_report_from_inputs(
    request: PromotionArtifactIdentityReportRequest,
) -> Result<PromotionArtifactIdentityReportV1, PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("report_id", &request.report_id)?;
    let report = promotion_artifact_identity_report(&request.report_id, &request.inputs);
    validate_promotion_artifact_identity_report(&report)?;
    Ok(report)
}

#[must_use]
pub fn promotion_artifact_identity_report(
    report_id: impl Into<String>,
    inputs: &[RolePromotionInputV1],
) -> PromotionArtifactIdentityReportV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    for input in inputs {
        if let Err(err) = super::transform::validate_role_artifact_source(&input.source) {
            blockers.push(super::promotion_finding(
                "promotion_artifact_source_invalid",
                err.to_string(),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
        }
        if input.role != input.source.role {
            blockers.push(super::promotion_finding(
                "promotion_source_role_mismatch",
                format!(
                    "promotion input role {} does not match artifact source role {}",
                    input.role, input.source.role
                ),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
        }
        roles.push(role_promotion_artifact_identity(input));
    }
    let identity_groups = promotion_artifact_identity_groups(&roles);
    let summary = promotion_artifact_identity_summary(&roles, &identity_groups);

    let mut report = PromotionArtifactIdentityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        artifact_identity_report_digest: String::new(),
        status: if blockers.is_empty() {
            PromotionReadinessStatusV1::Ready
        } else {
            PromotionReadinessStatusV1::Blocked
        },
        summary,
        identity_groups,
        roles,
        blockers,
    };
    report.artifact_identity_report_digest = promotion_artifact_identity_report_digest(&report);
    report
}

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

pub(super) fn artifact_identity_changed(before: &RoleArtifactV1, after: &RoleArtifactV1) -> bool {
    before.source != after.source
        || before.wasm_path != after.wasm_path
        || before.wasm_gz_path != after.wasm_gz_path
        || before.wasm_sha256 != after.wasm_sha256
        || before.wasm_gz_sha256 != after.wasm_gz_sha256
        || before.candid_path != after.candid_path
        || before.candid_sha256 != after.candid_sha256
}

pub(super) fn role_materialization_identity_matches(
    before: &RoleArtifactV1,
    after: &RoleArtifactV1,
) -> bool {
    before.source == after.source
        && before.wasm_path == after.wasm_path
        && before.wasm_gz_path == after.wasm_gz_path
        && before.wasm_sha256 == after.wasm_sha256
        && before.wasm_gz_sha256 == after.wasm_gz_sha256
        && before.candid_path == after.candid_path
        && before.candid_sha256 == after.candid_sha256
        && before.canonical_embedded_config_sha256 == after.canonical_embedded_config_sha256
}

pub(super) fn role_promotion_artifact_identity(
    input: &RolePromotionInputV1,
) -> RolePromotionArtifactIdentityV1 {
    let wasm_sha256 = input.source.expected_wasm_sha256.clone();
    let wasm_gz_sha256 = input.source.expected_wasm_gz_sha256.clone();
    RolePromotionArtifactIdentityV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        identity_kind: promotion_artifact_identity_kind(input.promotion_level, &input.source),
        digest_pinned: wasm_sha256.is_some() || wasm_gz_sha256.is_some(),
        wasm_sha256,
        wasm_gz_sha256,
        candid_sha256: input.source.expected_candid_sha256.clone(),
        canonical_embedded_config_sha256: input
            .source
            .expected_canonical_embedded_config_sha256
            .clone(),
    }
}

pub(super) fn role_materialization_identity_from_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> RolePromotionMaterializationIdentityV1 {
    RolePromotionMaterializationIdentityV1 {
        role: evidence.recipe.package_or_role_selector.clone(),
        evidence_id: evidence.evidence_id.clone(),
        materialization_evidence_digest: evidence.materialization_evidence_digest.clone(),
        recipe_id: evidence.recipe.recipe_id.clone(),
        materialization_input_id: evidence
            .materialization_input
            .materialization_input_id
            .clone(),
        materialization_result_id: evidence
            .materialization_result
            .materialization_result_id
            .clone(),
        materialization_input_digest: evidence.computed_materialization_input_digest.clone(),
        canonical_embedded_config_sha256: evidence
            .materialization_input
            .canonical_embedded_config_sha256
            .clone(),
        network: evidence.materialization_input.network.clone(),
        root_trust_anchor: evidence.materialization_input.root_trust_anchor.clone(),
        runtime_variant: evidence.materialization_input.runtime_variant.clone(),
        wasm_sha256: evidence.materialization_result.wasm_sha256.clone(),
        wasm_gz_sha256: evidence.materialization_result.wasm_gz_sha256.clone(),
        installed_module_hash: evidence
            .materialization_result
            .installed_module_hash
            .clone(),
        candid_sha256: evidence.materialization_result.candid_sha256.clone(),
    }
}

pub(super) fn promotion_artifact_identity_groups(
    roles: &[RolePromotionArtifactIdentityV1],
) -> Vec<PromotionArtifactIdentityGroupV1> {
    let mut groups = BTreeMap::<String, PromotionArtifactIdentityGroupV1>::new();
    for role in roles {
        let identity_key = artifact_identity_key_for_role(role);
        let group = groups.entry(identity_key.clone()).or_insert_with(|| {
            PromotionArtifactIdentityGroupV1 {
                identity_key,
                identity_kind: role.identity_kind,
                roles: Vec::new(),
                source_kinds: Vec::new(),
                source_locators: Vec::new(),
                digest_pinned: role.digest_pinned,
                wasm_sha256: role.wasm_sha256.clone(),
                wasm_gz_sha256: role.wasm_gz_sha256.clone(),
                candid_sha256: role.candid_sha256.clone(),
                canonical_embedded_config_sha256: role.canonical_embedded_config_sha256.clone(),
            }
        });
        if !group.source_kinds.contains(&role.source_kind) {
            group.source_kinds.push(role.source_kind);
        }
        if let Some(locator) = &role.source_locator
            && !group.source_locators.contains(locator)
        {
            group.source_locators.push(locator.clone());
        }
        group.roles.push(role.role.clone());
    }
    groups.into_values().collect()
}

pub(super) fn promotion_artifact_identity_summary(
    roles: &[RolePromotionArtifactIdentityV1],
    groups: &[PromotionArtifactIdentityGroupV1],
) -> PromotionArtifactIdentitySummaryV1 {
    PromotionArtifactIdentitySummaryV1 {
        role_count: roles.len(),
        identity_group_count: groups.len(),
        shared_identity_group_count: groups.iter().filter(|group| group.roles.len() > 1).count(),
        digest_pinned_role_count: roles.iter().filter(|role| role.digest_pinned).count(),
        source_build_role_count: roles
            .iter()
            .filter(|role| role.identity_kind == PromotionArtifactIdentityKindV1::SourceBuild)
            .count(),
        deferred_identity_role_count: roles
            .iter()
            .filter(|role| role.identity_kind == PromotionArtifactIdentityKindV1::Deferred)
            .count(),
    }
}

pub(super) fn promotion_materialization_output_groups(
    roles: &[RolePromotionMaterializationIdentityV1],
) -> Vec<PromotionMaterializationOutputGroupV1> {
    let mut groups = BTreeMap::<String, PromotionMaterializationOutputGroupV1>::new();
    for role in roles {
        let output_identity_key = materialization_output_key_for_role(role);
        let group = groups
            .entry(output_identity_key.clone())
            .or_insert_with(|| PromotionMaterializationOutputGroupV1 {
                output_identity_key,
                roles: Vec::new(),
                wasm_sha256: role.wasm_sha256.clone(),
                wasm_gz_sha256: role.wasm_gz_sha256.clone(),
                installed_module_hash: role.installed_module_hash.clone(),
                candid_sha256: role.candid_sha256.clone(),
            });
        group.roles.push(role.role.clone());
    }
    groups.into_values().collect()
}

const fn promotion_artifact_identity_kind(
    promotion_level: PromotionArtifactLevelV1,
    source: &RoleArtifactSourceV1,
) -> PromotionArtifactIdentityKindV1 {
    if matches!(promotion_level, PromotionArtifactLevelV1::SourceBuild) {
        return PromotionArtifactIdentityKindV1::SourceBuild;
    }
    match (
        source.expected_wasm_sha256.is_some(),
        source.expected_wasm_gz_sha256.is_some(),
    ) {
        (true, true) => PromotionArtifactIdentityKindV1::SealedWasmAndCompressedWasm,
        (true, false) => PromotionArtifactIdentityKindV1::SealedWasm,
        (false, true) => PromotionArtifactIdentityKindV1::SealedCompressedWasm,
        (false, false) => PromotionArtifactIdentityKindV1::Deferred,
    }
}

pub(super) fn artifact_identity_key_for_role(role: &RolePromotionArtifactIdentityV1) -> String {
    match role.identity_kind {
        PromotionArtifactIdentityKindV1::SealedWasm
        | PromotionArtifactIdentityKindV1::SealedCompressedWasm
        | PromotionArtifactIdentityKindV1::SealedWasmAndCompressedWasm => sealed_identity_key(
            role.wasm_sha256.as_deref(),
            role.wasm_gz_sha256.as_deref(),
            role.candid_sha256.as_deref(),
            role.canonical_embedded_config_sha256.as_deref(),
        ),
        PromotionArtifactIdentityKindV1::SourceBuild => format!(
            "source_build:source_kind={:?}:locator={}:candid={}:config={}",
            role.source_kind,
            optional_identity_part(role.source_locator.as_deref()),
            optional_identity_part(role.candid_sha256.as_deref()),
            optional_identity_part(role.canonical_embedded_config_sha256.as_deref())
        ),
        PromotionArtifactIdentityKindV1::Deferred => format!(
            "deferred:source_kind={:?}:locator={}",
            role.source_kind,
            optional_identity_part(role.source_locator.as_deref())
        ),
    }
}

pub(super) fn artifact_identity_key_for_group(group: &PromotionArtifactIdentityGroupV1) -> String {
    match group.identity_kind {
        PromotionArtifactIdentityKindV1::SealedWasm
        | PromotionArtifactIdentityKindV1::SealedCompressedWasm
        | PromotionArtifactIdentityKindV1::SealedWasmAndCompressedWasm => sealed_identity_key(
            group.wasm_sha256.as_deref(),
            group.wasm_gz_sha256.as_deref(),
            group.candid_sha256.as_deref(),
            group.canonical_embedded_config_sha256.as_deref(),
        ),
        PromotionArtifactIdentityKindV1::SourceBuild => format!(
            "source_build:source_kind={}:locator={}:candid={}:config={}",
            source_kind_identity_part(single_group_source_kind(group)),
            optional_identity_part(single_group_source_locator(group)),
            optional_identity_part(group.candid_sha256.as_deref()),
            optional_identity_part(group.canonical_embedded_config_sha256.as_deref())
        ),
        PromotionArtifactIdentityKindV1::Deferred => format!(
            "deferred:source_kind={}:locator={}",
            source_kind_identity_part(single_group_source_kind(group)),
            optional_identity_part(single_group_source_locator(group))
        ),
    }
}

pub(super) fn materialization_output_key_for_role(
    role: &RolePromotionMaterializationIdentityV1,
) -> String {
    materialization_output_key(
        &role.wasm_sha256,
        &role.wasm_gz_sha256,
        &role.installed_module_hash,
        &role.candid_sha256,
    )
}

pub(super) fn materialization_output_key_for_group(
    group: &PromotionMaterializationOutputGroupV1,
) -> String {
    materialization_output_key(
        &group.wasm_sha256,
        &group.wasm_gz_sha256,
        &group.installed_module_hash,
        &group.candid_sha256,
    )
}

pub(super) fn role_summary_artifact_identity_changed(role: &RolePromotionPlanTransformV1) -> bool {
    role.artifact_source_before != role.artifact_source_after
        || role.wasm_sha256_before != role.wasm_sha256_after
        || role.wasm_gz_sha256_before != role.wasm_gz_sha256_after
        || role.candid_sha256_before != role.candid_sha256_after
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

fn materialization_output_key(
    wasm_sha256: &str,
    wasm_gz_sha256: &str,
    installed_module_hash: &str,
    candid_sha256: &str,
) -> String {
    format!(
        "materialized:wasm={wasm_sha256}:wasm_gz={wasm_gz_sha256}:installed={installed_module_hash}:candid={candid_sha256}"
    )
}

fn source_kind_identity_part(kind: Option<RoleArtifactSourceKindV1>) -> String {
    kind.map_or_else(|| "not-recorded".to_string(), |kind| format!("{kind:?}"))
}

fn single_group_source_kind(
    group: &PromotionArtifactIdentityGroupV1,
) -> Option<RoleArtifactSourceKindV1> {
    group.source_kinds.first().copied()
}

fn single_group_source_locator(group: &PromotionArtifactIdentityGroupV1) -> Option<&str> {
    group.source_locators.first().map(String::as_str)
}

fn sealed_identity_key(
    wasm_sha256: Option<&str>,
    wasm_gz_sha256: Option<&str>,
    candid_sha256: Option<&str>,
    canonical_embedded_config_sha256: Option<&str>,
) -> String {
    format!(
        "sealed:wasm={}:wasm_gz={}:candid={}:config={}",
        optional_identity_part(wasm_sha256),
        optional_identity_part(wasm_gz_sha256),
        optional_identity_part(candid_sha256),
        optional_identity_part(canonical_embedded_config_sha256)
    )
}

const fn optional_identity_part(value: Option<&str>) -> &str {
    match value {
        Some(value) => value,
        None => "not-recorded",
    }
}
