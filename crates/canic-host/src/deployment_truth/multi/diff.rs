use super::super::stable_json_sha256_hex;
use crate::deployment_truth::{
    DeploymentCheckV1, DeploymentComparisonCategoryV1, DeploymentComparisonDiffV1,
    DeploymentInventoryV1, ObservedCanisterV1, ObservedPoolCanisterV1, RoleArtifactV1,
    SafetySeverityV1,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn compare_identity(
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
        "environment",
        Some(left.plan.deployment_identity.environment.as_str()),
        Some(right.plan.deployment_identity.environment.as_str()),
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

pub(super) fn compare_artifact_evidence(
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

pub(super) fn compare_observed_module_hashes(
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

pub(super) fn compare_embedded_config_evidence(
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

pub(super) fn compare_authority_evidence(
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

pub(super) fn compare_pool_evidence(
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

pub(super) fn compare_verifier_readiness_evidence(
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

pub(super) fn compare_external_lifecycle_evidence(
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
            .entry(canister.control_class.label().to_string())
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
