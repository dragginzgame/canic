use crate::{
    evidence_envelope::{
        EvidenceMessageSeverityV1, EvidenceMessageV1, InputFingerprintV1, file_input_fingerprint,
    },
    install_root::{InstallState, RootVerificationStatus},
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub const DEPLOYMENT_CATALOG_REPORT_SCHEMA_ID: &str = "canic.deployment_catalog_report.v1";
const NO_DEPLOYMENT_STATE_WARNING_CODE: &str = "catalog.no_deployment_state";
const LOCAL_STATE_FINGERPRINT_FAILED_WARNING_CODE: &str = "catalog.local_state_fingerprint_failed";
const MALFORMED_DEPLOYMENT_STATE_WARNING_CODE: &str = "catalog.malformed_deployment_state";

///
/// DeploymentCatalogRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeploymentCatalogRequest {
    pub icp_root: PathBuf,
    pub network: String,
    pub generated_at: String,
}

///
/// DeploymentCatalogReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentCatalogReportV1 {
    pub schema_version: u32,
    pub generated_at: String,
    pub project_root: Option<String>,
    pub entries: Vec<DeploymentCatalogEntryV1>,
    pub warnings: Vec<EvidenceMessageV1>,
}

///
/// DeploymentCatalogEntryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentCatalogEntryV1 {
    pub deployment: String,
    pub fleet: Option<String>,
    pub network: Option<String>,
    pub root_principal: Option<String>,
    pub root_verification: DeploymentCatalogRootVerificationV1,
    pub local_state_ref: Option<InputFingerprintV1>,
    pub warnings: Vec<EvidenceMessageV1>,
}

///
/// DeploymentCatalogRootVerificationV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentCatalogRootVerificationV1 {
    Unknown,
    NotVerified,
    Verified,
}

///
/// DeploymentCatalogError
///
#[derive(Debug, ThisError)]
pub enum DeploymentCatalogError {
    #[error("deployment target {deployment} is not known on network {network}")]
    UnknownDeployment { network: String, deployment: String },

    #[error("failed to read deployment catalog state directory {}: {source}", path.display())]
    StateDirectory { path: PathBuf, source: io::Error },
}

#[must_use]
pub const fn deployment_catalog_report_schema_id() -> &'static str {
    DEPLOYMENT_CATALOG_REPORT_SCHEMA_ID
}

pub fn build_deployment_catalog_report(
    request: &DeploymentCatalogRequest,
) -> Result<DeploymentCatalogReportV1, DeploymentCatalogError> {
    let deployments_dir = deployment_state_dir(&request.icp_root, &request.network);
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    if !deployments_dir.exists() {
        warnings.push(catalog_warning(
            NO_DEPLOYMENT_STATE_WARNING_CODE,
            format!(
                "no deployment-target state exists for network {}",
                request.network
            ),
            Some(path_subject(&deployments_dir, &request.icp_root)),
        ));
        return Ok(report(request, entries, warnings));
    }

    let read_dir = fs::read_dir(&deployments_dir).map_err(|source| {
        DeploymentCatalogError::StateDirectory {
            path: deployments_dir.clone(),
            source,
        }
    })?;

    let mut paths = read_dir
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| DeploymentCatalogError::StateDirectory {
            path: deployments_dir.clone(),
            source,
        })?;
    paths.sort();

    for path in paths {
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        match catalog_entry_from_path(&request.icp_root, &request.network, &path) {
            Ok(entry) => entries.push(entry),
            Err(warning) => warnings.push(warning),
        }
    }

    entries.sort_by(|left, right| left.deployment.cmp(&right.deployment));
    Ok(report(request, entries, warnings))
}

pub fn inspect_deployment_catalog_report(
    request: &DeploymentCatalogRequest,
    deployment: &str,
) -> Result<DeploymentCatalogReportV1, DeploymentCatalogError> {
    let mut report = build_deployment_catalog_report(request)?;
    if let Some(entry) = report
        .entries
        .iter()
        .find(|entry| entry.deployment == deployment)
        .cloned()
    {
        report.entries = vec![entry];
        return Ok(report);
    }

    Err(DeploymentCatalogError::UnknownDeployment {
        network: request.network.clone(),
        deployment: deployment.to_string(),
    })
}

#[must_use]
pub fn deployment_catalog_report_text(report: &DeploymentCatalogReportV1) -> String {
    let mut lines = Vec::new();
    lines.push("Deployment catalog:".to_string());
    lines.push(format!("generated_at: {}", report.generated_at));
    lines.push(format!("entries: {}", report.entries.len()));
    if let Some(project_root) = &report.project_root {
        lines.push(format!("project_root: {project_root}"));
    }
    if !report.warnings.is_empty() {
        lines.push("warnings:".to_string());
        for warning in &report.warnings {
            lines.push(format!("  {}: {}", warning.code, warning.message));
        }
    }
    if report.entries.is_empty() {
        lines.push("deployments: none".to_string());
        return lines.join("\n");
    }

    lines.push("deployments:".to_string());
    for entry in &report.entries {
        lines.push(format!("  {}", entry.deployment));
        if let Some(fleet) = &entry.fleet {
            lines.push(format!("    fleet: {fleet}"));
        }
        if let Some(network) = &entry.network {
            lines.push(format!("    network: {network}"));
        }
        if let Some(root) = &entry.root_principal {
            lines.push(format!("    root_principal: {root}"));
        }
        lines.push(format!(
            "    root_verification: {}",
            root_verification_label(entry.root_verification)
        ));
        if !entry.warnings.is_empty() {
            lines.push("    warnings:".to_string());
            for warning in &entry.warnings {
                lines.push(format!("      {}: {}", warning.code, warning.message));
            }
        }
    }

    lines.join("\n")
}

fn report(
    request: &DeploymentCatalogRequest,
    entries: Vec<DeploymentCatalogEntryV1>,
    warnings: Vec<EvidenceMessageV1>,
) -> DeploymentCatalogReportV1 {
    DeploymentCatalogReportV1 {
        schema_version: 1,
        generated_at: request.generated_at.clone(),
        project_root: Some(".".to_string()),
        entries,
        warnings,
    }
}

fn catalog_entry_from_path(
    root: &Path,
    network: &str,
    path: &Path,
) -> Result<DeploymentCatalogEntryV1, EvidenceMessageV1> {
    let deployment = path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| {
            malformed_state_warning(path, root, "deployment state file name is not UTF-8")
        })?
        .to_string();
    let bytes = fs::read(path).map_err(|err| {
        malformed_state_warning(path, root, format!("failed to read state: {err}"))
    })?;
    let state = serde_json::from_slice::<InstallState>(&bytes).map_err(|err| {
        malformed_state_warning(path, root, format!("failed to decode state: {err}"))
    })?;

    if state.deployment_name != deployment {
        return Err(malformed_state_warning(
            path,
            root,
            format!(
                "deployment state filename is {deployment}, but state records {}",
                state.deployment_name
            ),
        ));
    }
    if state.network != network {
        return Err(malformed_state_warning(
            path,
            root,
            format!(
                "deployment state is for network {}, but catalog network is {network}",
                state.network
            ),
        ));
    }

    let (local_state_ref, mut warnings) =
        match file_input_fingerprint("deployment_state", path, root, None, None) {
            Ok(fingerprint) => (Some(fingerprint), Vec::new()),
            Err(err) => (
                None,
                vec![catalog_warning(
                    LOCAL_STATE_FINGERPRINT_FAILED_WARNING_CODE,
                    format!("failed to fingerprint deployment state: {err}"),
                    Some(path_subject(path, root)),
                )],
            ),
        };

    warnings.sort_by(|left, right| left.code.cmp(&right.code));
    Ok(DeploymentCatalogEntryV1 {
        deployment: state.deployment_name,
        fleet: Some(state.fleet_template),
        network: Some(state.network),
        root_principal: Some(state.root_canister_id),
        root_verification: catalog_root_verification(&state.root_verification),
        local_state_ref,
        warnings,
    })
}

fn deployment_state_dir(root: &Path, network: &str) -> PathBuf {
    root.join(".canic").join(network).join("deployments")
}

const fn catalog_root_verification(
    status: &RootVerificationStatus,
) -> DeploymentCatalogRootVerificationV1 {
    match status {
        RootVerificationStatus::Verified => DeploymentCatalogRootVerificationV1::Verified,
        RootVerificationStatus::NotVerified => DeploymentCatalogRootVerificationV1::NotVerified,
    }
}

const fn root_verification_label(status: DeploymentCatalogRootVerificationV1) -> &'static str {
    match status {
        DeploymentCatalogRootVerificationV1::Unknown => "unknown",
        DeploymentCatalogRootVerificationV1::NotVerified => "not_verified",
        DeploymentCatalogRootVerificationV1::Verified => "verified",
    }
}

fn malformed_state_warning(
    path: &Path,
    root: &Path,
    message: impl Into<String>,
) -> EvidenceMessageV1 {
    catalog_warning(
        MALFORMED_DEPLOYMENT_STATE_WARNING_CODE,
        message,
        Some(path_subject(path, root)),
    )
}

fn catalog_warning(
    code: &str,
    message: impl Into<String>,
    source: Option<String>,
) -> EvidenceMessageV1 {
    EvidenceMessageV1 {
        code: code.to_string(),
        message: message.into(),
        severity: EvidenceMessageSeverityV1::Warning,
        source,
        related_input: None,
    }
}

fn path_subject(path: &Path, root: &Path) -> String {
    crate::evidence_envelope::command_path_for_root(path, root)
}

#[cfg(test)]
mod tests;
