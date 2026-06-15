use crate::deployment_truth::{
    PromotionMaterializationIdentityReportV1, PromotionWasmStoreCatalogVerificationV1,
    PromotionWasmStoreIdentityReportV1, RolePromotionPlanTransformV1, RolePromotionProvenanceV1,
};

pub(super) fn role_promotion_provenance_from_transform(
    role: &RolePromotionPlanTransformV1,
) -> RolePromotionProvenanceV1 {
    RolePromotionProvenanceV1 {
        role: role.role.clone(),
        promotion_level: role.promotion_level,
        source_kind: role.source_kind,
        artifact_identity_changed: role.artifact_identity_changed,
        embedded_config_changed: role.embedded_config_changed,
        target_materialization_preserved: role.target_materialization_preserved,
        materialization_evidence_id: role
            .source_build_materialization
            .as_ref()
            .map(|materialization| materialization.evidence_id.clone()),
        materialization_evidence_digest: role
            .source_build_materialization
            .as_ref()
            .map(|materialization| materialization.materialization_evidence_digest.clone()),
        wasm_store_locator: None,
        wasm_store_catalog_observation_digest: None,
    }
}

pub(super) fn attach_wasm_store_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    report: Option<&PromotionWasmStoreIdentityReportV1>,
) {
    let Some(report) = report else {
        return;
    };
    for role in roles {
        if let Some(wasm_store_role) = report.roles.iter().find(|item| item.role == role.role) {
            role.wasm_store_locator = wasm_store_role.wasm_store_locator.clone();
        }
    }
}

pub(super) fn attach_wasm_store_catalog_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    verification: Option<&PromotionWasmStoreCatalogVerificationV1>,
) {
    let Some(verification) = verification else {
        return;
    };
    for role in roles {
        if let Some(catalog_role) = verification
            .roles
            .iter()
            .find(|item| item.role == role.role)
        {
            role.wasm_store_catalog_observation_digest =
                Some(catalog_role.catalog_observation_digest.clone());
        }
    }
}

pub(super) fn attach_materialization_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    report: Option<&PromotionMaterializationIdentityReportV1>,
) {
    let Some(report) = report else {
        return;
    };
    for role in roles {
        if let Some(materialization_role) = report.roles.iter().find(|item| item.role == role.role)
        {
            role.materialization_evidence_id = Some(materialization_role.evidence_id.clone());
            role.materialization_evidence_digest =
                Some(materialization_role.materialization_evidence_digest.clone());
        }
    }
}
