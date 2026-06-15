use crate::deployment_truth::{
    PromotionArtifactIdentityGroupV1, PromotionArtifactIdentityKindV1,
    PromotionArtifactIdentitySummaryV1, PromotionMaterializationOutputGroupV1,
    RoleArtifactSourceKindV1, RolePromotionArtifactIdentityV1,
    RolePromotionMaterializationIdentityV1,
};
use std::collections::BTreeMap;

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

pub(in crate::deployment_truth::promotion) fn promotion_materialization_output_groups(
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

pub(in crate::deployment_truth::promotion) fn materialization_output_key_for_role(
    role: &RolePromotionMaterializationIdentityV1,
) -> String {
    materialization_output_key(
        &role.wasm_sha256,
        &role.wasm_gz_sha256,
        &role.installed_module_hash,
        &role.candid_sha256,
    )
}

pub(in crate::deployment_truth::promotion) fn materialization_output_key_for_group(
    group: &PromotionMaterializationOutputGroupV1,
) -> String {
    materialization_output_key(
        &group.wasm_sha256,
        &group.wasm_gz_sha256,
        &group.installed_module_hash,
        &group.candid_sha256,
    )
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
