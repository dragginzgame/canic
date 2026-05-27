use super::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
struct DeploymentComparisonReportDigestInput<'a> {
    report_id: &'a str,
    compared_at: &'a str,
    left: &'a DeploymentComparisonTargetV1,
    right: &'a DeploymentComparisonTargetV1,
    status: SafetyStatusV1,
    identity_diff: &'a [DeploymentComparisonDiffV1],
    artifact_diff: &'a [DeploymentComparisonDiffV1],
    module_hash_diff: &'a [DeploymentComparisonDiffV1],
    embedded_config_diff: &'a [DeploymentComparisonDiffV1],
    authority_diff: &'a [DeploymentComparisonDiffV1],
    pool_diff: &'a [DeploymentComparisonDiffV1],
    verifier_readiness_diff: &'a [DeploymentComparisonDiffV1],
    external_lifecycle_diff: &'a [DeploymentComparisonDiffV1],
    hard_failures: &'a [SafetyFindingV1],
    warnings: &'a [SafetyFindingV1],
    next_actions: &'a [String],
}

///
/// DeploymentComparisonReportError
///
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum DeploymentComparisonReportError {
    #[error(
        "deployment comparison report schema version {actual} does not match expected {expected}"
    )]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("deployment comparison report field `{field}` is required")]
    MissingRequiredField { field: &'static str },
    #[error("deployment comparison report field `{field}` digest is stale")]
    DigestMismatch { field: &'static str },
    #[error("deployment comparison report status does not match report findings")]
    StatusMismatch,
}

/// Build a passive 0.46 cross-deployment comparison report from two existing
/// deployment-truth checks. This is evidence comparison only; it does not
/// query live inventory or mutate deployment state.
#[must_use]
pub fn deployment_comparison_report_from_checks(
    report_id: impl Into<String>,
    compared_at: impl Into<String>,
    left_label: impl Into<String>,
    right_label: impl Into<String>,
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
) -> DeploymentComparisonReportV1 {
    let left_label = left_label.into();
    let right_label = right_label.into();
    let mut identity_diff = Vec::new();
    let mut artifact_diff = Vec::new();
    let mut module_hash_diff = Vec::new();
    let mut embedded_config_diff = Vec::new();
    let mut authority_diff = Vec::new();
    let mut pool_diff = Vec::new();
    let mut verifier_readiness_diff = Vec::new();
    let mut external_lifecycle_diff = Vec::new();

    compare_identity(left, right, &mut identity_diff);
    compare_artifact_evidence(left, right, &mut artifact_diff);
    compare_observed_module_hashes(left, right, &mut module_hash_diff);
    compare_embedded_config_evidence(left, right, &mut embedded_config_diff);
    compare_authority_evidence(left, right, &mut authority_diff);
    compare_pool_evidence(left, right, &mut pool_diff);
    compare_verifier_readiness_evidence(left, right, &mut verifier_readiness_diff);
    compare_external_lifecycle_evidence(left, right, &mut external_lifecycle_diff);

    let mut hard_failures = Vec::new();
    let mut warnings = Vec::new();
    compare_input_check_consistency(&left_label, left, &mut hard_failures);
    compare_input_check_consistency(&right_label, right, &mut hard_failures);
    compare_input_check_status(&left_label, &left.report, &mut hard_failures, &mut warnings);
    compare_input_check_status(
        &right_label,
        &right.report,
        &mut hard_failures,
        &mut warnings,
    );
    let diff_groups = [
        identity_diff.as_slice(),
        artifact_diff.as_slice(),
        module_hash_diff.as_slice(),
        embedded_config_diff.as_slice(),
        authority_diff.as_slice(),
        pool_diff.as_slice(),
        verifier_readiness_diff.as_slice(),
        external_lifecycle_diff.as_slice(),
    ];
    warnings.extend(comparison_warnings(&diff_groups));
    let status = comparison_status(&hard_failures, &warnings);
    let next_actions = comparison_next_actions(status);

    let mut report = DeploymentComparisonReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        report_digest: String::new(),
        compared_at: compared_at.into(),
        left: comparison_target(left_label, left),
        right: comparison_target(right_label, right),
        status,
        identity_diff,
        artifact_diff,
        module_hash_diff,
        embedded_config_diff,
        authority_diff,
        pool_diff,
        verifier_readiness_diff,
        external_lifecycle_diff,
        hard_failures,
        warnings,
        next_actions,
    };
    report.report_digest = deployment_comparison_report_digest(&report);
    report
}

/// Validate archived 0.46 comparison report consistency and digest stability.
pub fn validate_deployment_comparison_report(
    report: &DeploymentComparisonReportV1,
) -> Result<(), DeploymentComparisonReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(DeploymentComparisonReportError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: report.schema_version,
        });
    }
    ensure_comparison_field("report_id", report.report_id.as_str())?;
    ensure_comparison_field("report_digest", report.report_digest.as_str())?;
    ensure_comparison_field("compared_at", report.compared_at.as_str())?;
    validate_comparison_target("left", &report.left)?;
    validate_comparison_target("right", &report.right)?;
    if report.status != comparison_status(&report.hard_failures, &report.warnings) {
        return Err(DeploymentComparisonReportError::StatusMismatch);
    }
    if report.report_digest != deployment_comparison_report_digest(report) {
        return Err(DeploymentComparisonReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

fn comparison_target(label: String, check: &DeploymentCheckV1) -> DeploymentComparisonTargetV1 {
    DeploymentComparisonTargetV1 {
        label,
        check_id: check.check_id.clone(),
        check_digest: stable_json_sha256_hex(check),
        plan_id: check.plan.plan_id.clone(),
        plan_digest: stable_json_sha256_hex(&check.plan),
        inventory_id: check.inventory.inventory_id.clone(),
        inventory_digest: stable_json_sha256_hex(&check.inventory),
        deployment_identity: check.plan.deployment_identity.clone(),
    }
}

fn compare_identity(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_identity_names(left, right, diffs);
    compare_identity_digests(left, right, diffs);
    compare_identity_plan_shape(left, right, diffs);
    compare_identity_trust_domain(left, right, diffs);
}

fn compare_identity_names(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_value(
        DeploymentComparisonCategoryV1::Identity,
        "deployment_name",
        Some(left.plan.deployment_identity.deployment_name.as_str()),
        Some(right.plan.deployment_identity.deployment_name.as_str()),
        "deployment names differ",
        diffs,
    );
    compare_value(
        DeploymentComparisonCategoryV1::Identity,
        "network",
        Some(left.plan.deployment_identity.network.as_str()),
        Some(right.plan.deployment_identity.network.as_str()),
        "deployment networks differ",
        diffs,
    );
    compare_optional(
        DeploymentComparisonCategoryV1::Identity,
        "root_principal",
        left.plan.deployment_identity.root_principal.as_deref(),
        right.plan.deployment_identity.root_principal.as_deref(),
        "root principals differ",
        diffs,
    );
}

fn compare_identity_digests(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_optional(
        DeploymentComparisonCategoryV1::Identity,
        "authority_profile_hash",
        left.plan
            .deployment_identity
            .authority_profile_hash
            .as_deref(),
        right
            .plan
            .deployment_identity
            .authority_profile_hash
            .as_deref(),
        "authority profile hashes differ",
        diffs,
    );
    compare_optional(
        DeploymentComparisonCategoryV1::Identity,
        "artifact_set_digest",
        left.plan.deployment_identity.artifact_set_digest.as_deref(),
        right
            .plan
            .deployment_identity
            .artifact_set_digest
            .as_deref(),
        "artifact set digests differ",
        diffs,
    );
    compare_optional(
        DeploymentComparisonCategoryV1::Identity,
        "role_topology_hash",
        left.plan.deployment_identity.role_topology_hash.as_deref(),
        right.plan.deployment_identity.role_topology_hash.as_deref(),
        "role topology hashes differ",
        diffs,
    );
    compare_optional(
        DeploymentComparisonCategoryV1::Identity,
        "pool_identity_set_digest",
        left.plan
            .deployment_identity
            .pool_identity_set_digest
            .as_deref(),
        right
            .plan
            .deployment_identity
            .pool_identity_set_digest
            .as_deref(),
        "pool identity set digests differ",
        diffs,
    );
    compare_optional(
        DeploymentComparisonCategoryV1::Identity,
        "canonical_runtime_config_digest",
        left.plan
            .deployment_identity
            .canonical_runtime_config_digest
            .as_deref(),
        right
            .plan
            .deployment_identity
            .canonical_runtime_config_digest
            .as_deref(),
        "canonical runtime config digests differ",
        diffs,
    );
    compare_optional(
        DeploymentComparisonCategoryV1::Identity,
        "role_embedded_config_set_digest",
        left.plan
            .deployment_identity
            .role_embedded_config_set_digest
            .as_deref(),
        right
            .plan
            .deployment_identity
            .role_embedded_config_set_digest
            .as_deref(),
        "role embedded config set digests differ",
        diffs,
    );
}

fn compare_identity_plan_shape(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_value(
        DeploymentComparisonCategoryV1::Identity,
        "fleet_template",
        Some(left.plan.fleet_template.as_str()),
        Some(right.plan.fleet_template.as_str()),
        "fleet templates differ",
        diffs,
    );
    compare_value(
        DeploymentComparisonCategoryV1::Identity,
        "runtime_variant",
        Some(left.plan.runtime_variant.as_str()),
        Some(right.plan.runtime_variant.as_str()),
        "runtime variants differ",
        diffs,
    );
}

fn compare_identity_trust_domain(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_optional(
        DeploymentComparisonCategoryV1::TrustDomain,
        "root_trust_anchor",
        left.plan.trust_domain.root_trust_anchor.as_deref(),
        right.plan.trust_domain.root_trust_anchor.as_deref(),
        "root trust anchors differ",
        diffs,
    );
    compare_optional(
        DeploymentComparisonCategoryV1::TrustDomain,
        "migration_from",
        left.plan.trust_domain.migration_from.as_deref(),
        right.plan.trust_domain.migration_from.as_deref(),
        "migration sources differ",
        diffs,
    );
}

fn compare_artifact_evidence(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_maps(
        DeploymentComparisonCategoryV1::Artifact,
        &role_artifact_fingerprints(&left.plan.role_artifacts),
        &role_artifact_fingerprints(&right.plan.role_artifacts),
        "role artifact identity differs",
        diffs,
    );
}

fn compare_observed_module_hashes(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_maps(
        DeploymentComparisonCategoryV1::ModuleHash,
        &observed_canister_map(&left.inventory, |canister| {
            canister
                .module_hash
                .clone()
                .unwrap_or_else(|| "missing".into())
        }),
        &observed_canister_map(&right.inventory, |canister| {
            canister
                .module_hash
                .clone()
                .unwrap_or_else(|| "missing".into())
        }),
        "observed module hash differs",
        diffs,
    );
}

fn compare_embedded_config_evidence(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_maps(
        DeploymentComparisonCategoryV1::EmbeddedConfig,
        &observed_canister_map(&left.inventory, |canister| {
            canister
                .canonical_embedded_config_digest
                .clone()
                .unwrap_or_else(|| "missing".into())
        }),
        &observed_canister_map(&right.inventory, |canister| {
            canister
                .canonical_embedded_config_digest
                .clone()
                .unwrap_or_else(|| "missing".into())
        }),
        "observed embedded config digest differs",
        diffs,
    );
}

fn compare_authority_evidence(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_maps(
        DeploymentComparisonCategoryV1::Authority,
        &observed_canister_map(&left.inventory, canister_authority_fingerprint),
        &observed_canister_map(&right.inventory, canister_authority_fingerprint),
        "observed authority evidence differs",
        diffs,
    );
}

fn compare_pool_evidence(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_maps(
        DeploymentComparisonCategoryV1::Pool,
        &pool_fingerprints(&left.inventory.observed_pool),
        &pool_fingerprints(&right.inventory.observed_pool),
        "observed pool evidence differs",
        diffs,
    );
}

fn compare_verifier_readiness_evidence(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_value(
        DeploymentComparisonCategoryV1::VerifierReadiness,
        "verifier_readiness",
        Some(stable_json_sha256_hex(&left.inventory.observed_verifier_readiness).as_str()),
        Some(stable_json_sha256_hex(&right.inventory.observed_verifier_readiness).as_str()),
        "verifier readiness observations differ",
        diffs,
    );
}

fn compare_external_lifecycle_evidence(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_maps(
        DeploymentComparisonCategoryV1::ExternalLifecycle,
        &control_class_counts(&left.inventory),
        &control_class_counts(&right.inventory),
        "external lifecycle control-class evidence differs",
        diffs,
    );
}

fn role_artifact_fingerprints(artifacts: &[RoleArtifactV1]) -> BTreeMap<String, String> {
    artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.role.clone(),
                stable_json_sha256_hex(&(
                    artifact.source,
                    artifact.wasm_sha256.as_deref(),
                    artifact.wasm_gz_sha256.as_deref(),
                    artifact.installed_module_hash.as_deref(),
                    artifact.candid_sha256.as_deref(),
                    artifact.canonical_embedded_config_sha256.as_deref(),
                    artifact.package_version.as_deref(),
                )),
            )
        })
        .collect()
}

fn observed_canister_map(
    inventory: &DeploymentInventoryV1,
    value: impl Fn(&ObservedCanisterV1) -> String,
) -> BTreeMap<String, String> {
    inventory
        .observed_canisters
        .iter()
        .map(|canister| (canister_subject(canister), value(canister)))
        .collect()
}

fn canister_authority_fingerprint(canister: &ObservedCanisterV1) -> String {
    stable_json_sha256_hex(&(
        canister.control_class,
        &canister.controllers,
        canister.root_trust_anchor.as_deref(),
    ))
}

fn pool_fingerprints(pool: &[ObservedPoolCanisterV1]) -> BTreeMap<String, String> {
    pool.iter()
        .map(|canister| {
            (
                format!("{}:{}", canister.pool, canister.canister_id),
                stable_json_sha256_hex(&(canister.role.as_deref(), canister.control_class)),
            )
        })
        .collect()
}

fn control_class_counts(inventory: &DeploymentInventoryV1) -> BTreeMap<String, String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for canister in &inventory.observed_canisters {
        *counts
            .entry(format!("{:?}", canister.control_class))
            .or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(class, count)| (class, count.to_string()))
        .collect()
}

fn canister_subject(canister: &ObservedCanisterV1) -> String {
    canister
        .role
        .as_ref()
        .map_or_else(|| canister.canister_id.clone(), Clone::clone)
}

fn compare_maps(
    category: DeploymentComparisonCategoryV1,
    left: &BTreeMap<String, String>,
    right: &BTreeMap<String, String>,
    message: &'static str,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    let subjects: BTreeSet<_> = left.keys().chain(right.keys()).cloned().collect();
    for subject in subjects {
        compare_optional(
            category,
            &subject,
            left.get(&subject).map(String::as_str),
            right.get(&subject).map(String::as_str),
            message,
            diffs,
        );
    }
}

fn compare_value(
    category: DeploymentComparisonCategoryV1,
    subject: &str,
    left: Option<&str>,
    right: Option<&str>,
    message: &'static str,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    if left == right {
        return;
    }
    diffs.push(DeploymentComparisonDiffV1 {
        category,
        subject: subject.to_string(),
        left: left.map(str::to_string),
        right: right.map(str::to_string),
        severity: SafetySeverityV1::Warning,
        message: message.to_string(),
    });
}

fn compare_optional(
    category: DeploymentComparisonCategoryV1,
    subject: &str,
    left: Option<&str>,
    right: Option<&str>,
    message: &'static str,
    diffs: &mut Vec<DeploymentComparisonDiffV1>,
) {
    compare_value(category, subject, left, right, message, diffs);
}

fn comparison_warnings(diff_groups: &[&[DeploymentComparisonDiffV1]]) -> Vec<SafetyFindingV1> {
    let diff_count = diff_groups.iter().map(|group| group.len()).sum::<usize>();
    if diff_count == 0 {
        return Vec::new();
    }
    vec![SafetyFindingV1 {
        code: "deployment_comparison_drift".to_string(),
        message: format!("deployment comparison found {diff_count} drift item(s)"),
        severity: SafetySeverityV1::Warning,
        subject: None,
    }]
}

fn compare_input_check_status(
    label: &str,
    report: &SafetyReportV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    match report.status {
        SafetyStatusV1::Safe => {}
        SafetyStatusV1::Warning => warnings.push(SafetyFindingV1 {
            code: "deployment_comparison_input_warning".to_string(),
            message: "input deployment check has warnings; comparison is drift evidence, not whole-deployment safety".to_string(),
            severity: SafetySeverityV1::Warning,
            subject: Some(format!("{label}:{}", report.report_id)),
        }),
        SafetyStatusV1::Blocked => hard_failures.push(SafetyFindingV1 {
            code: "deployment_comparison_input_blocked".to_string(),
            message: "input deployment check is blocked; comparison cannot be used as ready deployment evidence".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", report.report_id)),
        }),
        SafetyStatusV1::NotEvaluated => hard_failures.push(SafetyFindingV1 {
            code: "deployment_comparison_input_not_evaluated".to_string(),
            message: "input deployment check was not evaluated; comparison cannot establish deployment safety".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", report.report_id)),
        }),
    }
}

fn compare_input_check_consistency(
    label: &str,
    check: &DeploymentCheckV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if check.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        hard_failures.push(SafetyFindingV1 {
            code: "deployment_comparison_input_schema_mismatch".to_string(),
            message: "input deployment check schema version is unsupported".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", check.check_id)),
        });
        return;
    }

    let expected_diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    if check.diff != expected_diff {
        hard_failures.push(SafetyFindingV1 {
            code: "deployment_comparison_input_diff_stale".to_string(),
            message: "input deployment check diff does not match its plan and inventory"
                .to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", check.check_id)),
        });
        return;
    }

    let expected_report = safety_report_from_diff(
        &check.report.report_id,
        check.report.diff_id.clone(),
        &check.diff,
    );
    if check.report != expected_report {
        hard_failures.push(SafetyFindingV1 {
            code: "deployment_comparison_input_report_stale".to_string(),
            message: "input deployment check report does not match its diff".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{label}:{}", check.check_id)),
        });
    }
}

const fn comparison_status(
    hard_failures: &[SafetyFindingV1],
    warnings: &[SafetyFindingV1],
) -> SafetyStatusV1 {
    if !hard_failures.is_empty() {
        SafetyStatusV1::Blocked
    } else if !warnings.is_empty() {
        SafetyStatusV1::Warning
    } else {
        SafetyStatusV1::Safe
    }
}

fn comparison_next_actions(status: SafetyStatusV1) -> Vec<String> {
    match status {
        SafetyStatusV1::Safe => vec!["no cross-deployment drift detected".to_string()],
        SafetyStatusV1::Warning => {
            vec!["review comparison drift before promotion, rebuild, or teardown".to_string()]
        }
        SafetyStatusV1::Blocked => {
            vec!["resolve hard comparison failures before using this evidence".to_string()]
        }
        SafetyStatusV1::NotEvaluated => vec!["run deployment comparison".to_string()],
    }
}

fn deployment_comparison_report_digest(report: &DeploymentComparisonReportV1) -> String {
    stable_json_sha256_hex(&DeploymentComparisonReportDigestInput {
        report_id: &report.report_id,
        compared_at: &report.compared_at,
        left: &report.left,
        right: &report.right,
        status: report.status,
        identity_diff: &report.identity_diff,
        artifact_diff: &report.artifact_diff,
        module_hash_diff: &report.module_hash_diff,
        embedded_config_diff: &report.embedded_config_diff,
        authority_diff: &report.authority_diff,
        pool_diff: &report.pool_diff,
        verifier_readiness_diff: &report.verifier_readiness_diff,
        external_lifecycle_diff: &report.external_lifecycle_diff,
        hard_failures: &report.hard_failures,
        warnings: &report.warnings,
        next_actions: &report.next_actions,
    })
}

fn validate_comparison_target(
    prefix: &'static str,
    target: &DeploymentComparisonTargetV1,
) -> Result<(), DeploymentComparisonReportError> {
    ensure_comparison_field(field_name(prefix, "label"), target.label.as_str())?;
    ensure_comparison_field(field_name(prefix, "check_id"), target.check_id.as_str())?;
    ensure_comparison_field(
        field_name(prefix, "check_digest"),
        target.check_digest.as_str(),
    )?;
    ensure_comparison_field(field_name(prefix, "plan_id"), target.plan_id.as_str())?;
    ensure_comparison_field(
        field_name(prefix, "plan_digest"),
        target.plan_digest.as_str(),
    )?;
    ensure_comparison_field(
        field_name(prefix, "inventory_id"),
        target.inventory_id.as_str(),
    )?;
    ensure_comparison_field(
        field_name(prefix, "inventory_digest"),
        target.inventory_digest.as_str(),
    )?;
    ensure_comparison_field(
        field_name(prefix, "deployment_name"),
        target.deployment_identity.deployment_name.as_str(),
    )?;
    ensure_comparison_field(
        field_name(prefix, "network"),
        target.deployment_identity.network.as_str(),
    )?;
    Ok(())
}

fn field_name(prefix: &'static str, field: &'static str) -> &'static str {
    match (prefix, field) {
        ("left", "label") => "left.label",
        ("left", "check_id") => "left.check_id",
        ("left", "check_digest") => "left.check_digest",
        ("left", "plan_id") => "left.plan_id",
        ("left", "plan_digest") => "left.plan_digest",
        ("left", "inventory_id") => "left.inventory_id",
        ("left", "inventory_digest") => "left.inventory_digest",
        ("left", "deployment_name") => "left.deployment_identity.deployment_name",
        ("left", "network") => "left.deployment_identity.network",
        ("right", "label") => "right.label",
        ("right", "check_id") => "right.check_id",
        ("right", "check_digest") => "right.check_digest",
        ("right", "plan_id") => "right.plan_id",
        ("right", "plan_digest") => "right.plan_digest",
        ("right", "inventory_id") => "right.inventory_id",
        ("right", "inventory_digest") => "right.inventory_digest",
        ("right", "deployment_name") => "right.deployment_identity.deployment_name",
        ("right", "network") => "right.deployment_identity.network",
        _ => field,
    }
}

fn ensure_comparison_field(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentComparisonReportError> {
    if value.trim().is_empty() {
        return Err(DeploymentComparisonReportError::MissingRequiredField { field });
    }
    Ok(())
}
