use super::super::{
    BuildMaterializationEvidenceV1, BuildMaterializationInputV1, BuildMaterializationResultV1,
    BuildRecipeIdentityV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionArtifactLevelV1,
    PromotionMaterializationIdentityReportV1, PromotionMaterializationOutputGroupV1,
    PromotionPlanTransformV1, PromotionReadinessStatusV1, RoleArtifactV1, RolePromotionInputV1,
    RolePromotionMaterializationIdentityV1, RolePromotionMaterializationLinkV1,
    RolePromotionPlanTransformV1, SafetyFindingV1, SafetySeverityV1,
};
use super::digest::{
    build_materialization_evidence_digest, build_materialization_input_digest,
    promotion_materialization_identity_report_digest,
};
use super::ensure::{
    ensure_materialization_field, ensure_materialization_link, ensure_materialization_report_field,
    ensure_materialization_report_sha256, ensure_materialization_sha256, ensure_transform_field,
};
use super::error::{
    PromotionMaterializationIdentityError, PromotionMaterializationIdentityReportError,
    PromotionPlanTransformError,
};
use super::identity::{
    materialization_output_key_for_group, materialization_output_key_for_role,
    promotion_materialization_output_groups, role_materialization_identity_from_evidence,
};
use super::request::{
    BuildMaterializationEvidenceRequest, PromotionMaterializationIdentityReportRequest,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_materialization_evidence(
    request: BuildMaterializationEvidenceRequest,
) -> Result<BuildMaterializationEvidenceV1, PromotionMaterializationIdentityError> {
    ensure_materialization_field("evidence_id", &request.evidence_id)?;
    validate_build_recipe_identity(&request.recipe)?;
    validate_build_materialization_input(&request.materialization_input)?;
    validate_build_materialization_result(&request.materialization_result)?;
    let computed_materialization_input_digest =
        build_materialization_input_digest(&request.materialization_input);
    let mut evidence = BuildMaterializationEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: request.evidence_id,
        materialization_evidence_digest: String::new(),
        recipe_id_matches_input: request.recipe.recipe_id
            == request.materialization_input.build_recipe_id,
        recipe_id_matches_result: request.recipe.recipe_id
            == request.materialization_result.build_recipe_id,
        materialization_input_digest_matches_result: computed_materialization_input_digest
            == request.materialization_result.materialization_input_digest,
        computed_materialization_input_digest,
        recipe: request.recipe,
        materialization_input: request.materialization_input,
        materialization_result: request.materialization_result,
    };
    evidence.materialization_evidence_digest = build_materialization_evidence_digest(&evidence);
    validate_build_materialization_evidence(&evidence)?;
    Ok(evidence)
}

pub fn validate_build_materialization_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionMaterializationIdentityError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: evidence.schema_version,
            },
        );
    }
    ensure_materialization_field("evidence_id", &evidence.evidence_id)?;
    ensure_materialization_sha256(
        "materialization_evidence_digest",
        &evidence.materialization_evidence_digest,
    )?;
    validate_build_recipe_identity(&evidence.recipe)?;
    validate_build_materialization_input(&evidence.materialization_input)?;
    validate_build_materialization_result(&evidence.materialization_result)?;
    ensure_materialization_sha256(
        "computed_materialization_input_digest",
        &evidence.computed_materialization_input_digest,
    )?;
    ensure_materialization_link(
        "recipe_id_matches_input",
        evidence.recipe_id_matches_input
            == (evidence.recipe.recipe_id == evidence.materialization_input.build_recipe_id),
    )?;
    ensure_materialization_link("recipe_id_matches_input", evidence.recipe_id_matches_input)?;
    ensure_materialization_link(
        "recipe_id_matches_result",
        evidence.recipe_id_matches_result
            == (evidence.recipe.recipe_id == evidence.materialization_result.build_recipe_id),
    )?;
    ensure_materialization_link(
        "recipe_id_matches_result",
        evidence.recipe_id_matches_result,
    )?;
    let computed = build_materialization_input_digest(&evidence.materialization_input);
    if computed != evidence.computed_materialization_input_digest {
        return Err(PromotionMaterializationIdentityError::DigestMismatch {
            field: "computed_materialization_input_digest",
            expected: computed,
            found: evidence.computed_materialization_input_digest.clone(),
        });
    }
    ensure_materialization_link(
        "materialization_input_digest_matches_result",
        evidence.materialization_input_digest_matches_result
            == (evidence.computed_materialization_input_digest
                == evidence.materialization_result.materialization_input_digest),
    )?;
    ensure_materialization_link(
        "materialization_input_digest_matches_result",
        evidence.materialization_input_digest_matches_result,
    )?;
    if evidence.materialization_evidence_digest != build_materialization_evidence_digest(evidence) {
        return Err(PromotionMaterializationIdentityError::LinkageMismatch {
            field: "materialization_evidence_digest",
        });
    }
    Ok(())
}

pub fn promotion_materialization_identity_report_from_evidence(
    request: PromotionMaterializationIdentityReportRequest,
) -> Result<PromotionMaterializationIdentityReportV1, PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field("report_id", &request.report_id)?;
    for evidence in &request.evidence {
        validate_build_materialization_evidence(evidence)?;
    }
    let report = promotion_materialization_identity_report(&request.report_id, &request.evidence);
    validate_promotion_materialization_identity_report(&report)?;
    Ok(report)
}

#[must_use]
pub fn promotion_materialization_identity_report(
    report_id: impl Into<String>,
    evidence: &[BuildMaterializationEvidenceV1],
) -> PromotionMaterializationIdentityReportV1 {
    let roles = evidence
        .iter()
        .map(role_materialization_identity_from_evidence)
        .collect::<Vec<_>>();
    let output_groups = promotion_materialization_output_groups(&roles);
    let blockers = Vec::new();
    let mut report = PromotionMaterializationIdentityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        materialization_identity_report_digest: String::new(),
        status: PromotionReadinessStatusV1::Ready,
        roles,
        output_groups,
        blockers,
    };
    report.materialization_identity_report_digest =
        promotion_materialization_identity_report_digest(&report);
    report
}

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

pub fn validate_build_recipe_identity(
    recipe: &BuildRecipeIdentityV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field("recipe_id", &recipe.recipe_id)?;
    ensure_materialization_field("source_revision", &recipe.source_revision)?;
    ensure_materialization_field("package_or_role_selector", &recipe.package_or_role_selector)?;
    ensure_materialization_field("cargo_profile", &recipe.cargo_profile)?;
    ensure_materialization_sha256("cargo_features_digest", &recipe.cargo_features_digest)?;
    ensure_materialization_sha256("cargo_lock_digest", &recipe.cargo_lock_digest)?;
    ensure_materialization_field("rust_toolchain", &recipe.rust_toolchain)?;
    ensure_materialization_field("builder_version", &recipe.builder_version)?;
    ensure_materialization_field("target_triple", &recipe.target_triple)?;
    ensure_materialization_field("linker_identity", &recipe.linker_identity)?;
    ensure_materialization_field("deterministic_build_mode", &recipe.deterministic_build_mode)?;
    ensure_materialization_field("wasm_opt_version", &recipe.wasm_opt_version)?;
    ensure_materialization_field("compression_identity", &recipe.compression_identity)?;
    Ok(())
}

pub fn validate_build_materialization_input(
    input: &BuildMaterializationInputV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field("materialization_input_id", &input.materialization_input_id)?;
    ensure_materialization_field("build_recipe_id", &input.build_recipe_id)?;
    ensure_materialization_sha256(
        "canonical_embedded_config_sha256",
        &input.canonical_embedded_config_sha256,
    )?;
    ensure_materialization_field("network", &input.network)?;
    ensure_materialization_field("root_trust_anchor", &input.root_trust_anchor)?;
    ensure_materialization_field("runtime_variant", &input.runtime_variant)?;
    Ok(())
}

pub fn validate_build_materialization_result(
    result: &BuildMaterializationResultV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field(
        "materialization_result_id",
        &result.materialization_result_id,
    )?;
    ensure_materialization_field("build_recipe_id", &result.build_recipe_id)?;
    ensure_materialization_sha256(
        "materialization_input_digest",
        &result.materialization_input_digest,
    )?;
    ensure_materialization_sha256("wasm_sha256", &result.wasm_sha256)?;
    ensure_materialization_sha256("wasm_gz_sha256", &result.wasm_gz_sha256)?;
    ensure_materialization_sha256("installed_module_hash", &result.installed_module_hash)?;
    ensure_materialization_sha256("candid_sha256", &result.candid_sha256)?;
    Ok(())
}

pub(super) fn attach_source_build_materialization(
    transform: &mut PromotionPlanTransformV1,
    inputs: &[RolePromotionInputV1],
    evidence: &[BuildMaterializationEvidenceV1],
) -> Result<(), PromotionPlanTransformError> {
    let input_roles = inputs
        .iter()
        .map(|input| input.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut links = BTreeMap::new();
    for item in evidence {
        validate_build_materialization_evidence(item)?;
        let role = item.recipe.package_or_role_selector.as_str();
        if !input_roles.contains(role) {
            return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
                role: role.to_string(),
            });
        }
        if links
            .insert(role.to_string(), materialization_link_from_evidence(item))
            .is_some()
        {
            return Err(PromotionPlanTransformError::DuplicateMaterializationRole {
                role: role.to_string(),
            });
        }
    }

    for role in &mut transform.roles {
        match role.promotion_level {
            PromotionArtifactLevelV1::SourceBuild => {
                let Some(link) = links.remove(&role.role) else {
                    return Err(PromotionPlanTransformError::MaterializationRoleMissing {
                        role: role.role.clone(),
                    });
                };
                role.source_build_materialization = Some(link);
            }
            PromotionArtifactLevelV1::SealedWasm => {
                if links.remove(&role.role).is_some() {
                    return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
                        role: role.role.clone(),
                    });
                }
            }
        }
    }

    if let Some(role) = links.keys().next() {
        return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
            role: role.clone(),
        });
    }
    Ok(())
}

fn materialization_link_from_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> RolePromotionMaterializationLinkV1 {
    RolePromotionMaterializationLinkV1 {
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
        wasm_sha256: evidence.materialization_result.wasm_sha256.clone(),
        wasm_gz_sha256: evidence.materialization_result.wasm_gz_sha256.clone(),
        installed_module_hash: evidence
            .materialization_result
            .installed_module_hash
            .clone(),
        candid_sha256: evidence.materialization_result.candid_sha256.clone(),
    }
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

pub(super) fn validate_role_materialization_link(
    role: &RolePromotionPlanTransformV1,
    promoted_role: &RoleArtifactV1,
) -> Result<(), PromotionPlanTransformError> {
    let Some(link) = &role.source_build_materialization else {
        return Ok(());
    };
    super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization",
        role.promotion_level == PromotionArtifactLevelV1::SourceBuild,
    )?;
    super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.role",
        link.role == role.role,
    )?;
    ensure_transform_field(
        "source_build_materialization.evidence_id",
        &link.evidence_id,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.materialization_evidence_digest",
        &link.materialization_evidence_digest,
    )?;
    ensure_transform_field("source_build_materialization.recipe_id", &link.recipe_id)?;
    ensure_transform_field(
        "source_build_materialization.materialization_input_id",
        &link.materialization_input_id,
    )?;
    ensure_transform_field(
        "source_build_materialization.materialization_result_id",
        &link.materialization_result_id,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.materialization_input_digest",
        &link.materialization_input_digest,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.wasm_sha256",
        &link.wasm_sha256,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.wasm_gz_sha256",
        &link.wasm_gz_sha256,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.installed_module_hash",
        &link.installed_module_hash,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.candid_sha256",
        &link.candid_sha256,
    )?;
    super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.wasm_sha256",
        promoted_role.wasm_sha256.as_deref() == Some(link.wasm_sha256.as_str()),
    )?;
    super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.wasm_gz_sha256",
        promoted_role.wasm_gz_sha256.as_deref() == Some(link.wasm_gz_sha256.as_str()),
    )?;
    super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.installed_module_hash",
        promoted_role.installed_module_hash.as_deref() == Some(link.installed_module_hash.as_str()),
    )?;
    super::transform::ensure_role_field_matches(
        role,
        "source_build_materialization.candid_sha256",
        promoted_role.candid_sha256.as_deref() == Some(link.candid_sha256.as_str()),
    )
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
    ensure_materialization_report_field("network", &role.network)?;
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
