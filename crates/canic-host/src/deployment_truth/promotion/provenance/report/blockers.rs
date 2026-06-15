use crate::deployment_truth::{
    ArtifactPromotionPlanV1, PromotionMaterializationIdentityReportV1,
    PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportV1,
    RolePromotionProvenanceV1, SafetyFindingV1, SafetySeverityV1,
};
use std::collections::BTreeSet;

pub(super) fn artifact_promotion_provenance_blockers(
    plan: &ArtifactPromotionPlanV1,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    materialization_report: Option<&PromotionMaterializationIdentityReportV1>,
    roles: &[RolePromotionProvenanceV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = plan.blockers.clone();
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(report) = wasm_store_report {
        blockers.extend(report.blockers.iter().cloned());
    }
    append_wasm_store_catalog_provenance_blockers(
        &mut blockers,
        wasm_store_report,
        wasm_store_catalog,
        &role_names,
    );
    if let Some(report) = materialization_report {
        blockers.extend(report.blockers.iter().cloned());
    }
    append_optional_report_unknown_role_blockers(
        &mut blockers,
        wasm_store_report,
        wasm_store_catalog,
        materialization_report,
        &role_names,
    );
    blockers
}

fn append_wasm_store_catalog_provenance_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    role_names: &BTreeSet<&str>,
) {
    let Some(verification) = wasm_store_catalog else {
        return;
    };
    blockers.extend(verification.blockers.iter().cloned());
    match wasm_store_report {
        Some(report) if verification.wasm_store_identity_report_id == report.report_id => {}
        Some(report) => blockers.push(super::super::super::promotion_finding(
            "promotion_provenance_wasm_store_catalog_identity_mismatch",
            format!(
                "wasm-store catalog verification references identity report {}, but provenance uses {}",
                verification.wasm_store_identity_report_id, report.report_id
            ),
            SafetySeverityV1::HardFailure,
            "wasm_store_catalog",
        )),
        None => blockers.push(super::super::super::promotion_finding(
            "promotion_provenance_wasm_store_catalog_identity_missing",
            "wasm-store catalog verification requires the referenced wasm-store identity report",
            SafetySeverityV1::HardFailure,
            "wasm_store_catalog",
        )),
    }
    if let Some(report) = wasm_store_report {
        append_wasm_store_catalog_locator_blockers(blockers, report, verification, role_names);
    }
}

fn append_wasm_store_catalog_locator_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    report: &PromotionWasmStoreIdentityReportV1,
    verification: &PromotionWasmStoreCatalogVerificationV1,
    role_names: &BTreeSet<&str>,
) {
    for catalog_role in &verification.roles {
        if !role_names.contains(catalog_role.role.as_str()) {
            continue;
        }
        match report.roles.iter().find(|role| role.role == catalog_role.role) {
            Some(identity_role)
                if identity_role.wasm_store_locator.as_deref()
                    == Some(catalog_role.wasm_store_locator.as_str()) => {}
            Some(identity_role) => blockers.push(super::super::super::promotion_finding(
                "promotion_provenance_wasm_store_catalog_locator_mismatch",
                format!(
                    "wasm-store catalog verification role {} uses locator {}, but identity report uses {}",
                    catalog_role.role,
                    catalog_role.wasm_store_locator,
                    identity_role.wasm_store_locator.as_deref().unwrap_or("none")
                ),
                SafetySeverityV1::HardFailure,
                &catalog_role.role,
            )),
            None => blockers.push(super::super::super::promotion_finding(
                "promotion_provenance_wasm_store_catalog_role_identity_missing",
                format!(
                    "wasm-store catalog verification role {} is missing from the wasm-store identity report",
                    catalog_role.role
                ),
                SafetySeverityV1::HardFailure,
                &catalog_role.role,
            )),
        }
    }
}

fn append_optional_report_unknown_role_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    materialization_report: Option<&PromotionMaterializationIdentityReportV1>,
    role_names: &BTreeSet<&str>,
) {
    if let Some(report) = wasm_store_report {
        for role in &report.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(super::super::super::promotion_finding(
                    "promotion_provenance_unknown_wasm_store_role",
                    format!(
                        "wasm-store identity report contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
    if let Some(verification) = wasm_store_catalog {
        for role in &verification.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(super::super::super::promotion_finding(
                    "promotion_provenance_unknown_wasm_store_catalog_role",
                    format!(
                        "wasm-store catalog verification contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
    if let Some(report) = materialization_report {
        for role in &report.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(super::super::super::promotion_finding(
                    "promotion_provenance_unknown_materialization_role",
                    format!(
                        "materialization identity report contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
}
