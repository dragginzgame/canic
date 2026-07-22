//! Module: fleets::adoption_report
//! Responsibility: build, render, and write `canic fleet adoption report` output.
//! Does not own: command dispatch, option parsing, fleet selection, or filesystem mutation outside report output.
//! Boundary: read-only adoption evidence assembly plus report/envelope formatting.

use crate::{evidence_support, output};
use canic_host::{
    adoption::{
        AdoptionArtifactStateV1, AdoptionAuthorityStateV1, AdoptionClassificationV1,
        AdoptionDeclarationStateV1, AdoptionMatchConfidenceV1, AdoptionObservationStateV1,
        AdoptionOperatorActionRequirementV1, AdoptionPackageMetadataV1, AdoptionPackageStateV1,
        AdoptionProfileV1, AdoptionRecommendationSeverityV1, AdoptionReportRequest,
        AdoptionReportV1, AdoptionSuggestedActionEffectV1, AdoptionSuggestedActionSupportV1,
        AdoptionTopologyStateV1, adoption_report_from_config_source,
    },
    build_provenance::build_provenance_schema,
    deployment_truth::{DeploymentInventoryV1, RoleArtifactManifestV1, RoleArtifactV1},
    evidence_envelope::{
        CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
        EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, InputFingerprintV1,
        PayloadSchemaRefV1, adoption_report_schema, deployment_check_schema,
        evidence_envelope_schema, evidence_summary_exit_class, file_input_fingerprint,
        json_payload_sha256,
    },
};
use serde::de::DeserializeOwned;
use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    FleetCommandError,
    options::{AdoptionReportFormat, AdoptionReportOptions},
};

pub(super) fn build_adoption_report_from_config_path(
    config_path: &Path,
    options: &AdoptionReportOptions,
    generated_at: &str,
) -> Result<AdoptionReportV1, FleetCommandError> {
    let config_source = fs::read_to_string(config_path)?;
    let inventory = adoption_inventory_from_options(options)?;
    let artifact_manifest = adoption_artifact_manifest_from_options(options)?;
    let package_metadata = adoption_package_metadata_from_options(config_path, options)?;
    let report_id = format!(
        "local:{}:{}:adoption-report",
        options.fleet,
        adoption_profile_label(options.profile)
    );

    adoption_report_from_config_source(AdoptionReportRequest {
        report_id: &report_id,
        generated_at,
        profile: options.profile,
        config_source: &config_source,
        inventory: inventory.as_ref(),
        artifact_manifest: artifact_manifest.as_ref(),
        package_metadata,
    })
    .map_err(FleetCommandError::from)
}

fn adoption_package_metadata_from_options(
    config_path: &Path,
    options: &AdoptionReportOptions,
) -> Result<Vec<AdoptionPackageMetadataV1>, FleetCommandError> {
    match (&options.package_metadata, &options.cargo_metadata) {
        (Some(path), _) => read_json_file(path),
        (None, Some(path)) => read_cargo_metadata_package_metadata(config_path, path),
        (None, None) => Ok(Vec::new()),
    }
}

fn adoption_artifact_manifest_from_options(
    options: &AdoptionReportOptions,
) -> Result<Option<RoleArtifactManifestV1>, FleetCommandError> {
    if let Some(path) = &options.artifact_manifest {
        return read_json_file(path).map(Some);
    }

    options
        .deployment_check
        .as_deref()
        .map(read_deployment_check_artifact_manifest)
        .transpose()
        .map(Option::flatten)
}

fn adoption_inventory_from_options(
    options: &AdoptionReportOptions,
) -> Result<Option<DeploymentInventoryV1>, FleetCommandError> {
    match (&options.inventory, &options.deployment_check) {
        (Some(path), _) => read_json_file(path).map(Some),
        (None, Some(path)) => read_deployment_check_inventory(path).map(Some),
        (None, None) => Ok(None),
    }
}

fn read_deployment_check_inventory(
    path: &Path,
) -> Result<DeploymentInventoryV1, FleetCommandError> {
    let value = read_json_file::<serde_json::Value>(path)?;
    let Some(inventory) = value.get("inventory") else {
        return Err(FleetCommandError::Usage(format!(
            "deployment check evidence {} is missing inventory",
            path.display()
        )));
    };

    Ok(serde_json::from_value(inventory.clone())?)
}

fn read_deployment_check_artifact_manifest(
    path: &Path,
) -> Result<Option<RoleArtifactManifestV1>, FleetCommandError> {
    let value = read_json_file::<serde_json::Value>(path)?;
    let Some(plan) = value.get("plan") else {
        return Ok(None);
    };
    let role_artifacts = plan
        .get("role_artifacts")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    let environment = plan
        .get("deployment_identity")
        .and_then(|identity| identity.get("environment"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let check_id = value
        .get("check_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown-check");

    Ok(Some(RoleArtifactManifestV1 {
        schema_version: 1,
        manifest_id: format!("deployment-check:{check_id}:role-artifacts"),
        environment,
        artifact_root: None,
        role_artifacts: serde_json::from_value::<Vec<RoleArtifactV1>>(role_artifacts)?,
        unresolved_artifacts: Vec::new(),
    }))
}

fn read_cargo_metadata_package_metadata(
    config_path: &Path,
    path: &Path,
) -> Result<Vec<AdoptionPackageMetadataV1>, FleetCommandError> {
    let value = read_json_file::<serde_json::Value>(path)?;
    let packages = value
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            FleetCommandError::Usage(format!(
                "cargo metadata evidence {} is missing packages",
                path.display()
            ))
        })?;
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new(""));
    let mut metadata = Vec::new();

    for package in packages {
        let Some(canic_metadata) = package
            .get("metadata")
            .and_then(|metadata| metadata.get("canic"))
        else {
            continue;
        };
        let Some(package_path) = cargo_metadata_package_path(config_dir, package) else {
            continue;
        };
        metadata.push(AdoptionPackageMetadataV1 {
            package: package_path,
            fleet: canic_metadata
                .get("fleet")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            role: canic_metadata
                .get("role")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
        });
    }

    Ok(metadata)
}

pub(super) fn cargo_metadata_package_path(
    config_dir: &Path,
    package: &serde_json::Value,
) -> Option<String> {
    let manifest_path = package.get("manifest_path")?.as_str()?;
    let package_dir = Path::new(manifest_path).parent()?;
    let relative = relative_package_dir(config_dir, package_dir);
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn relative_package_dir(config_dir: &Path, package_dir: &Path) -> PathBuf {
    if let Ok(relative) = package_dir.strip_prefix(config_dir) {
        return non_empty_relative_path(relative);
    }

    lexical_relative_path(config_dir, package_dir).unwrap_or_else(|| package_dir.to_path_buf())
}

fn non_empty_relative_path(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    }
}

fn lexical_relative_path(base: &Path, target: &Path) -> Option<PathBuf> {
    let base_components = normal_path_components(base);
    let target_components = normal_path_components(target);
    let common = base_components
        .iter()
        .zip(target_components.iter())
        .take_while(|(base, target)| base == target)
        .count();
    if common == 0 {
        return None;
    }

    let mut relative = PathBuf::new();
    for _ in common..base_components.len() {
        relative.push("..");
    }
    for component in &target_components[common..] {
        relative.push(component);
    }

    Some(non_empty_relative_path(&relative))
}

fn normal_path_components(path: &Path) -> Vec<String> {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir => components.push(String::new()),
            Component::CurDir => {}
            Component::ParentDir => components.push("..".to_string()),
            Component::Normal(segment) => components.push(segment.to_string_lossy().to_string()),
        }
    }

    components
}

fn read_json_file<T>(path: &Path) -> Result<T, FleetCommandError>
where
    T: DeserializeOwned,
{
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub(super) fn write_adoption_report(
    config_path: &Path,
    options: &AdoptionReportOptions,
    report: &AdoptionReportV1,
) -> Result<(), FleetCommandError> {
    match options.format {
        AdoptionReportFormat::Text => {
            output::write_text(options.output.as_deref(), &render_adoption_report(report))
        }
        AdoptionReportFormat::Json => output::write_pretty_json(options.output.as_deref(), report),
        AdoptionReportFormat::EnvelopeJson => {
            let envelope = build_adoption_report_envelope(config_path, options, report)?;
            output::write_pretty_json(options.output.as_deref(), &envelope)
        }
    }
}

fn build_adoption_report_envelope(
    config_path: &Path,
    options: &AdoptionReportOptions,
    report: &AdoptionReportV1,
) -> Result<EvidenceEnvelopeV1, FleetCommandError> {
    let payload = serde_json::to_value(report)?;
    let payload_sha256 = Some(json_payload_sha256(report)?);
    let config_root = config_path.parent().unwrap_or_else(|| Path::new("."));
    let summary = adoption_report_evidence_summary(report);
    let exit_class = evidence_summary_exit_class(&summary, false);

    Ok(EvidenceEnvelopeV1 {
        envelope_schema: evidence_envelope_schema(),
        canic_version: env!("CARGO_PKG_VERSION").to_string(),
        command: adoption_report_command_provenance(config_root, options),
        target: EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::FleetAdoption,
            deployment: None,
            fleet: Some(report.fleet.clone()),
            role: None,
            profile: Some(adoption_profile_label(report.profile).to_string()),
            environment: None,
        },
        generated_at: report.generated_at.clone(),
        source_config: Some(file_input_fingerprint(
            "canic_config",
            config_path,
            config_root,
            Some(PayloadSchemaRefV1::internal("canic.config.toml", "1")),
            None,
        )?),
        inputs: adoption_report_input_fingerprints(config_root, options)?,
        payload_schema: adoption_report_schema(),
        payload_sha256,
        payload,
        summary,
        exit_class,
    })
}

fn adoption_report_command_provenance(
    config_root: &Path,
    options: &AdoptionReportOptions,
) -> CommandProvenanceV1 {
    let mut argv_normalized = vec![
        "canic".to_string(),
        "fleet".to_string(),
        "adoption".to_string(),
        "report".to_string(),
        options.fleet.clone(),
        "--profile".to_string(),
        adoption_profile_label(options.profile).to_string(),
        "--evidence-envelope".to_string(),
    ];

    let mut argv_redactions = Vec::new();

    evidence_support::push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--deployment-check",
        options.deployment_check.as_deref(),
        config_root,
    );
    evidence_support::push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--inventory",
        options.inventory.as_deref(),
        config_root,
    );
    evidence_support::push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--artifact-manifest",
        options.artifact_manifest.as_deref(),
        config_root,
    );
    evidence_support::push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--cargo-metadata",
        options.cargo_metadata.as_deref(),
        config_root,
    );
    evidence_support::push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--package-metadata",
        options.package_metadata.as_deref(),
        config_root,
    );
    evidence_support::push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        "--build-provenance",
        options.build_provenance.as_deref(),
        config_root,
    );

    CommandProvenanceV1 {
        name: "canic fleet adoption report".to_string(),
        argv_normalized,
        argv_redactions,
        format: "envelope-json".to_string(),
    }
}

fn adoption_report_input_fingerprints(
    config_root: &Path,
    options: &AdoptionReportOptions,
) -> Result<Vec<InputFingerprintV1>, FleetCommandError> {
    let mut inputs = Vec::new();

    push_optional_input_fingerprint(
        &mut inputs,
        "deployment_check",
        options.deployment_check.as_deref(),
        config_root,
        Some(deployment_check_schema()),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "deployment_inventory",
        options.inventory.as_deref(),
        config_root,
        Some(PayloadSchemaRefV1::internal(
            "canic.deployment_inventory.v1",
            "1",
        )),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "role_artifact_manifest",
        options.artifact_manifest.as_deref(),
        config_root,
        Some(PayloadSchemaRefV1::internal(
            "canic.role_artifact_manifest.v1",
            "1",
        )),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "cargo_metadata",
        options.cargo_metadata.as_deref(),
        config_root,
        Some(PayloadSchemaRefV1::internal("cargo.metadata.v1", "1")),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "adoption_package_metadata",
        options.package_metadata.as_deref(),
        config_root,
        Some(PayloadSchemaRefV1::experimental(
            "canic.adoption_package_metadata.v1",
            "1",
        )),
    )?;
    push_optional_input_fingerprint(
        &mut inputs,
        "build_provenance",
        options.build_provenance.as_deref(),
        config_root,
        Some(build_provenance_schema()),
    )?;

    Ok(inputs)
}

fn push_optional_input_fingerprint(
    inputs: &mut Vec<InputFingerprintV1>,
    kind: &str,
    path: Option<&Path>,
    config_root: &Path,
    schema: Option<PayloadSchemaRefV1>,
) -> Result<(), FleetCommandError> {
    if let Some(path) = path {
        inputs.push(file_input_fingerprint(
            kind,
            path,
            config_root,
            schema,
            None,
        )?);
    }
    Ok(())
}

fn adoption_report_evidence_summary(report: &AdoptionReportV1) -> EvidenceSummaryV1 {
    EvidenceSummaryV1 {
        warnings: Vec::new(),
        blocked_actions: report
            .blocked_actions
            .iter()
            .map(|action| {
                EvidenceMessageV1::new(
                    "adoption.blocked_action",
                    action.clone(),
                    EvidenceMessageSeverityV1::Error,
                )
            })
            .collect(),
        missing_or_stale_evidence: report
            .inputs
            .missing_or_stale_evidence
            .iter()
            .map(|evidence| {
                EvidenceMessageV1::new(
                    "adoption.missing_or_stale_evidence",
                    evidence.clone(),
                    EvidenceMessageSeverityV1::Warning,
                )
            })
            .collect(),
        evidence_conflicts: adoption_evidence_conflict_messages(report),
    }
}

fn adoption_evidence_conflict_messages(report: &AdoptionReportV1) -> Vec<EvidenceMessageV1> {
    report
        .role_findings
        .iter()
        .filter(|finding| {
            finding
                .classifications
                .contains(&AdoptionClassificationV1::EvidenceConflict)
        })
        .map(|finding| {
            EvidenceMessageV1::new(
                "adoption.evidence_conflict",
                format!("role {} has conflicting adoption evidence", finding.role),
                EvidenceMessageSeverityV1::Error,
            )
        })
        .collect()
}

pub(super) fn render_adoption_report(report: &AdoptionReportV1) -> String {
    let mut lines = vec![
        "Adoption report:".to_string(),
        format!("  fleet: {}", report.fleet),
        format!("  profile: {}", adoption_profile_label(report.profile)),
        format!("  report_id: {}", report.report_id),
        format!("  generated_at: {}", report.generated_at),
        "  read_only: true".to_string(),
        "Summary:".to_string(),
    ];

    lines.extend(adoption_summary_lines(report));
    lines.extend(adoption_missing_evidence_lines(report));
    lines.extend(adoption_role_finding_lines(report));
    lines.extend(adoption_observed_canister_lines(report));
    lines.extend(adoption_recommendation_lines(report));
    lines.extend(adoption_blocked_action_lines(report));
    lines.join("\n")
}

fn adoption_summary_lines(report: &AdoptionReportV1) -> Vec<String> {
    let summary = &report.summary;
    [
        format!(
            "  managed_configured_roles: {}",
            summary.managed_configured_roles
        ),
        format!("  declared_only_roles: {}", summary.declared_only_roles),
        format!(
            "  attached_unobserved_roles: {}",
            summary.attached_unobserved_roles
        ),
        format!(
            "  observed_only_canisters: {}",
            summary.observed_only_canisters
        ),
        format!(
            "  user_controlled_canisters: {}",
            summary.user_controlled_canisters
        ),
        format!(
            "  external_controller_required: {}",
            summary.external_controller_required
        ),
        format!("  evidence_conflicts: {}", summary.evidence_conflicts),
        format!(
            "  mutating_actions_performed: {}",
            summary.mutating_actions_performed
        ),
    ]
    .into()
}

fn adoption_missing_evidence_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Missing or stale evidence:".to_string()];
    if report.inputs.missing_or_stale_evidence.is_empty() {
        lines.push("  - none".to_string());
    } else {
        lines.extend(
            report
                .inputs
                .missing_or_stale_evidence
                .iter()
                .map(|evidence| format!("  - {evidence}")),
        );
    }
    lines
}

fn adoption_role_finding_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Role findings:".to_string()];
    if report.role_findings.is_empty() {
        lines.push("  - none".to_string());
        return lines;
    }

    for finding in &report.role_findings {
        lines.push(format!(
            "  - {}.{}: {}",
            finding.fleet,
            finding.role,
            format_adoption_classifications(&finding.classifications)
        ));
        lines.push(format!(
            "    state: declaration={}, topology={}, observation={}, authority={}, artifact={}, package={}",
            adoption_declaration_state_label(finding.declaration_state),
            adoption_topology_state_label(finding.topology_state),
            adoption_observation_state_label(finding.observation_state),
            adoption_authority_state_label(finding.authority_state),
            adoption_artifact_state_label(finding.artifact_state),
            adoption_package_state_label(finding.package_state)
        ));
        lines.extend(
            finding
                .warnings
                .iter()
                .map(|warning| format!("    warning: {warning}")),
        );
    }

    lines
}

fn adoption_observed_canister_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Observed canisters:".to_string()];
    if report.observed_canisters.is_empty() {
        lines.push("  - none".to_string());
        return lines;
    }

    for finding in &report.observed_canisters {
        let role = finding.matched_role.as_deref().map_or("-", |role| role);
        lines.push(format!(
            "  - {}: role={}, confidence={}, classifications={}",
            finding.canister_id,
            role,
            adoption_match_confidence_label(finding.confidence),
            format_adoption_classifications(&finding.classifications)
        ));
        if !finding.controllers.is_empty() {
            lines.push(format!(
                "    controllers: {}",
                finding.controllers.join(",")
            ));
        }
        if let Some(evidence) = &finding.wasm_evidence {
            lines.push(format!("    wasm_evidence: {evidence}"));
        }
        if let Some(evidence) = &finding.deployment_target_evidence {
            lines.push(format!("    deployment_target_evidence: {evidence}"));
        }
        lines.extend(
            finding
                .warnings
                .iter()
                .map(|warning| format!("    warning: {warning}")),
        );
    }
    lines
}

fn adoption_recommendation_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Recommendations (report-only; not executed):".to_string()];
    if report.recommendations.is_empty() {
        lines.push("  - none".to_string());
        return lines;
    }

    for recommendation in &report.recommendations {
        lines.push(format!(
            "  - {} [{}; {}; {}; {}]",
            recommendation.description,
            adoption_recommendation_severity_label(recommendation.severity),
            adoption_action_effect_label(recommendation.suggested_action_effect),
            adoption_action_support_label(recommendation.suggested_action_support),
            adoption_operator_requirement_label(recommendation.operator_action_requirement)
        ));
        if let Some(action) = &recommendation.suggested_action {
            lines.push(format!("    suggested_action_preview: {action}"));
            lines.push("    status: not executed by adoption report".to_string());
            lines.push(format!(
                "    support: {}",
                adoption_action_support_label(recommendation.suggested_action_support)
            ));
        }
    }
    lines
}

fn adoption_blocked_action_lines(report: &AdoptionReportV1) -> Vec<String> {
    let mut lines = vec!["Blocked adoption actions (not executed by report):".to_string()];
    if report.blocked_actions.is_empty() {
        lines.push("  - none".to_string());
    } else {
        lines.extend(
            report
                .blocked_actions
                .iter()
                .map(|action| format!("  - {action}")),
        );
    }
    lines
}

fn format_adoption_classifications(classifications: &[AdoptionClassificationV1]) -> String {
    if classifications.is_empty() {
        return "-".to_string();
    }

    classifications
        .iter()
        .map(|classification| adoption_classification_label(*classification))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn current_adoption_report_generated_at() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "unix:{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    ))
}

const fn adoption_profile_label(profile: AdoptionProfileV1) -> &'static str {
    match profile {
        AdoptionProfileV1::Brownfield => "brownfield",
        AdoptionProfileV1::Partial => "partial",
        AdoptionProfileV1::Standalone => "standalone",
        AdoptionProfileV1::LeafOnly => "leaf-only",
        AdoptionProfileV1::HybridExternalWasm => "hybrid-external-wasm",
        AdoptionProfileV1::Minimal => "minimal",
    }
}

const fn adoption_classification_label(classification: AdoptionClassificationV1) -> &'static str {
    match classification {
        AdoptionClassificationV1::Managed => "managed",
        AdoptionClassificationV1::DeclaredOnly => "declared-only",
        AdoptionClassificationV1::ObservedOnly => "observed-only",
        AdoptionClassificationV1::AttachedUnobserved => "attached-unobserved",
        AdoptionClassificationV1::UserControlled => "user-controlled",
        AdoptionClassificationV1::ExternalControllerRequired => "external-controller-required",
        AdoptionClassificationV1::ImportedPoolCandidate => "imported-pool-candidate",
        AdoptionClassificationV1::EvidenceConflict => "evidence-conflict",
    }
}

const fn adoption_declaration_state_label(state: AdoptionDeclarationStateV1) -> &'static str {
    match state {
        AdoptionDeclarationStateV1::Undeclared => "undeclared",
        AdoptionDeclarationStateV1::Declared => "declared",
    }
}

const fn adoption_topology_state_label(state: AdoptionTopologyStateV1) -> &'static str {
    match state {
        AdoptionTopologyStateV1::Unattached => "unattached",
        AdoptionTopologyStateV1::Attached => "attached",
    }
}

const fn adoption_observation_state_label(state: AdoptionObservationStateV1) -> &'static str {
    match state {
        AdoptionObservationStateV1::Unobserved => "unobserved",
        AdoptionObservationStateV1::Observed => "observed",
        AdoptionObservationStateV1::CandidateMatch => "candidate-match",
        AdoptionObservationStateV1::ConflictingMatch => "conflicting-match",
    }
}

const fn adoption_authority_state_label(state: AdoptionAuthorityStateV1) -> &'static str {
    match state {
        AdoptionAuthorityStateV1::CanicAuthorized => "canic-authorized",
        AdoptionAuthorityStateV1::UserControlled => "user-controlled",
        AdoptionAuthorityStateV1::External => "external",
        AdoptionAuthorityStateV1::Unknown => "unknown",
    }
}

const fn adoption_artifact_state_label(state: AdoptionArtifactStateV1) -> &'static str {
    match state {
        AdoptionArtifactStateV1::CanicBuilt => "canic-built",
        AdoptionArtifactStateV1::ExternalWasm => "external-wasm",
        AdoptionArtifactStateV1::Unknown => "unknown",
    }
}

const fn adoption_package_state_label(state: AdoptionPackageStateV1) -> &'static str {
    match state {
        AdoptionPackageStateV1::UndeclaredRole => "undeclared-role",
        AdoptionPackageStateV1::NotChecked => "not-checked",
        AdoptionPackageStateV1::Matches => "matches",
        AdoptionPackageStateV1::MissingFleet => "missing-fleet",
        AdoptionPackageStateV1::MissingRole => "missing-role",
        AdoptionPackageStateV1::Mismatch => "mismatch",
    }
}

const fn adoption_match_confidence_label(confidence: AdoptionMatchConfidenceV1) -> &'static str {
    match confidence {
        AdoptionMatchConfidenceV1::None => "none",
        AdoptionMatchConfidenceV1::Candidate => "candidate",
        AdoptionMatchConfidenceV1::ExplicitEvidence => "explicit-evidence",
        AdoptionMatchConfidenceV1::Conflict => "conflict",
    }
}

const fn adoption_recommendation_severity_label(
    severity: AdoptionRecommendationSeverityV1,
) -> &'static str {
    match severity {
        AdoptionRecommendationSeverityV1::Info => "info",
        AdoptionRecommendationSeverityV1::Warning => "warning",
        AdoptionRecommendationSeverityV1::Blocked => "blocked",
    }
}

const fn adoption_action_effect_label(effect: AdoptionSuggestedActionEffectV1) -> &'static str {
    match effect {
        AdoptionSuggestedActionEffectV1::ReadOnly => "read-only",
        AdoptionSuggestedActionEffectV1::MutatesState => "mutates-state",
    }
}

const fn adoption_action_support_label(support: AdoptionSuggestedActionSupportV1) -> &'static str {
    match support {
        AdoptionSuggestedActionSupportV1::SupportedByAdoption => "supported-by-adoption",
        AdoptionSuggestedActionSupportV1::UnsupportedByAdoption => "unsupported-by-adoption",
    }
}

const fn adoption_operator_requirement_label(
    requirement: AdoptionOperatorActionRequirementV1,
) -> &'static str {
    match requirement {
        AdoptionOperatorActionRequirementV1::Required => "operator-action-required",
        AdoptionOperatorActionRequirementV1::NotRequired => "no-operator-action-required",
    }
}
