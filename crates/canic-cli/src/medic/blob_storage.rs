//! Module: canic_cli::medic::blob_storage
//!
//! Responsibility: classify blob-storage billing readiness for deployment Medic reports.
//! Does not own: blob-storage mutation, target resolution, or report rendering.
//! Boundary: maps Candid capability and command readiness evidence into Medic checks.

use crate::{
    blob_storage::{
        self as blob_storage_api, BlobStorageCommandError, BlobStorageMedicStatus,
        BlobStorageMedicSummary,
    },
    medic::{
        command::MedicOptions,
        report::{MedicCategory, MedicCheck, MedicSource},
    },
};
use std::{fs, path::Path};

use canic_core::protocol::{
    BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, BLOB_STORAGE_STATUS,
    BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
};
use canic_host::{
    candid_endpoints::parse_candid_service_endpoints, icp::local_canister_candid_path,
};

pub(super) fn check_blob_storage_billing(
    options: &MedicOptions,
    canister: &str,
    network: &str,
) -> MedicCheck {
    match blob_storage_api::medic_summary(
        options.deployment_name(),
        canister,
        network,
        &options.icp,
    ) {
        Ok(summary) => blob_storage_medic_check_from_summary(summary),
        Err(err) => blob_storage_medic_error_check(err, options.deployment_name(), canister),
    }
}

pub(super) fn check_blob_storage_not_selected(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    network: &str,
) -> MedicCheck {
    let next = icp_root
        .and_then(|root| {
            blob_storage_billing_roles_from_candid_dir(root, network)
                .into_iter()
                .next()
        })
        .map_or_else(
            || {
                "run canic medic deployment <deployment> --blob-storage <canister-or-role>"
                    .to_string()
            },
            |first| {
                format!(
                    "run canic medic deployment {} --blob-storage {first}",
                    options.deployment_name()
                )
            },
        );
    MedicCheck::not_evaluated(
        MedicCategory::BlobStorage,
        "blob_storage_not_selected",
        "blob_storage",
        "no blob-storage target was selected",
        next,
        MedicSource::Command,
    )
}

pub(super) fn blob_storage_billing_roles_from_candid_dir(
    icp_root: &Path,
    network: &str,
) -> Vec<String> {
    let canisters_dir = icp_root.join(".icp").join(network).join("canisters");
    let Ok(entries) = fs::read_dir(canisters_dir) else {
        return Vec::new();
    };
    let mut roles = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|role| {
            let candid_path = local_canister_candid_path(icp_root, network, role);
            candid_path_declares_blob_storage_billing(&candid_path)
        })
        .collect::<Vec<_>>();
    roles.sort();
    roles.dedup();
    roles
}

fn candid_path_declares_blob_storage_billing(path: &Path) -> bool {
    let Ok(candid) = fs::read_to_string(path) else {
        return false;
    };
    candid_declares_blob_storage_billing(&candid)
}

pub(super) fn candid_declares_blob_storage_billing(candid: &str) -> bool {
    let Ok(endpoints) = parse_candid_service_endpoints(candid) else {
        return false;
    };
    [
        BLOB_STORAGE_STATUS,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
    ]
    .iter()
    .all(|method| endpoints.iter().any(|endpoint| endpoint.name == *method))
}

pub(super) fn blob_storage_medic_check_from_summary(
    summary: BlobStorageMedicSummary,
) -> MedicCheck {
    match summary.status {
        BlobStorageMedicStatus::Ready => MedicCheck::pass(
            MedicCategory::BlobStorage,
            "blob_storage_billing_ready",
            "blob_storage",
            summary.detail,
            summary.next,
            MedicSource::BlobStorageReadiness,
        ),
        BlobStorageMedicStatus::Warning => MedicCheck::warn(
            MedicCategory::BlobStorage,
            "blob_storage_billing_unready",
            "blob_storage",
            summary.detail,
            summary.next,
            MedicSource::BlobStorageReadiness,
        ),
        BlobStorageMedicStatus::Blocked => MedicCheck::fail(
            MedicCategory::BlobStorage,
            "blob_storage_billing_unready",
            "blob_storage",
            summary.detail,
            summary.next,
            MedicSource::BlobStorageReadiness,
        ),
    }
}

pub(super) fn blob_storage_medic_error_check(
    error: BlobStorageCommandError,
    deployment: &str,
    canister: &str,
) -> MedicCheck {
    let (code, next) = match &error {
        BlobStorageCommandError::UnknownTarget { .. } => (
            "blob_storage_target_missing",
            format!(
                "choose a registered blob-storage role or canister for deployment {deployment}"
            ),
        ),
        BlobStorageCommandError::AmbiguousRole { .. } => (
            "blob_storage_target_ambiguous",
            "use one canister principal instead of an ambiguous role".to_string(),
        ),
        BlobStorageCommandError::CandidUnavailable { .. }
        | BlobStorageCommandError::MethodUnavailable { .. } => (
            "blob_storage_target_not_blob_storage",
            "select a canister that exposes blob-storage billing readiness endpoints".to_string(),
        ),
        _ => (
            "blob_storage_billing_unready",
            format!("run canic blob-storage status {deployment} {canister}"),
        ),
    };

    MedicCheck::fail(
        MedicCategory::BlobStorage,
        code,
        "blob_storage",
        error.to_string(),
        next,
        MedicSource::BlobStorageReadiness,
    )
}
