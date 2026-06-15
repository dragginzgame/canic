use std::collections::BTreeSet;

use canic_core::bootstrap::parse_config_model;

use super::{
    evidence::{
        artifact_conflict_roles, artifact_evidence_by_role, artifact_states_by_role,
        duplicate_observed_roles, missing_evidence, observed_canisters_by_role,
        package_metadata_by_path,
    },
    findings::{
        DeclaredRoleFindingInput, ObservedOnlyRoleFindingInput, observed_canister_findings,
        role_finding_for_declared_role, role_finding_for_observed_only_role,
    },
    model::{
        ADOPTION_REPORT_SCHEMA_VERSION, AdoptionReportError, AdoptionReportInputsV1,
        AdoptionReportRequest, AdoptionReportV1,
    },
    recommendations::blocked_actions,
    summary::report_summary,
};

///
/// adoption_report_from_config_source
///
pub fn adoption_report_from_config_source(
    request: AdoptionReportRequest<'_>,
) -> Result<AdoptionReportV1, AdoptionReportError> {
    let config = parse_config_model(request.config_source)
        .map_err(|err| AdoptionReportError::InvalidConfig(err.to_string()))?;
    let fleet = config
        .fleet_name()
        .ok_or(AdoptionReportError::MissingFleetName)?
        .to_string();
    let attached_roles = config.attached_roles();
    let observed_by_role = observed_canisters_by_role(request.inventory);
    let observed_duplicate_roles = duplicate_observed_roles(&observed_by_role);
    let packages_by_path = package_metadata_by_path(request.package_metadata);
    let artifacts_by_role = artifact_states_by_role(request.artifact_manifest, request.inventory);
    let artifact_conflict_roles =
        artifact_conflict_roles(request.artifact_manifest, request.inventory);
    let artifact_evidence_by_role =
        artifact_evidence_by_role(request.artifact_manifest, request.inventory);
    let declared_roles = config.roles.keys().cloned().collect::<BTreeSet<_>>();

    let mut role_findings = Vec::new();
    let mut seen_roles = BTreeSet::new();

    for (role, declaration) in &config.roles {
        seen_roles.insert(role.as_str().to_string());
        role_findings.push(role_finding_for_declared_role(DeclaredRoleFindingInput {
            profile: request.profile,
            fleet: &fleet,
            role,
            package: declaration.package.as_str(),
            attached: attached_roles.contains(role),
            observed: observed_by_role.get(role.as_str()).map(Vec::as_slice),
            duplicate_observation: observed_duplicate_roles.contains(role.as_str()),
            packages_by_path: &packages_by_path,
            artifact_state: artifacts_by_role.get(role.as_str()).copied(),
            artifact_conflict: artifact_conflict_roles.contains(role.as_str()),
            artifact_evidence: artifact_evidence_by_role
                .get(role.as_str())
                .map(Vec::as_slice),
        }));
    }

    for (role, observed) in &observed_by_role {
        if seen_roles.contains(role) {
            continue;
        }
        role_findings.push(role_finding_for_observed_only_role(
            ObservedOnlyRoleFindingInput {
                profile: request.profile,
                fleet: &fleet,
                role,
                observed,
                duplicate_observation: observed_duplicate_roles.contains(role),
                artifact_state: artifacts_by_role.get(role.as_str()).copied(),
                artifact_conflict: artifact_conflict_roles.contains(role),
                artifact_evidence: artifact_evidence_by_role
                    .get(role.as_str())
                    .map(Vec::as_slice),
            },
        ));
    }

    role_findings.sort_by(|left, right| left.role.cmp(&right.role));

    let observed_canisters = observed_canister_findings(
        request.profile,
        &fleet,
        request.inventory,
        &declared_roles,
        &attached_roles,
    );
    let summary = report_summary(&role_findings, &observed_canisters);
    let mut recommendations = Vec::new();
    for finding in &role_findings {
        recommendations.extend(finding.recommendations.clone());
    }
    for finding in &observed_canisters {
        recommendations.extend(finding.recommendations.clone());
    }

    Ok(AdoptionReportV1 {
        schema_version: ADOPTION_REPORT_SCHEMA_VERSION,
        report_id: request.report_id.to_string(),
        generated_at: request.generated_at.to_string(),
        fleet,
        profile: request.profile,
        inputs: AdoptionReportInputsV1 {
            config_present: true,
            inventory_id: request
                .inventory
                .map(|inventory| inventory.inventory_id.clone()),
            artifact_manifest_id: request
                .artifact_manifest
                .map(|manifest| manifest.manifest_id.clone()),
            package_metadata_count: packages_by_path.len(),
            missing_or_stale_evidence: missing_evidence(
                request.inventory,
                request.artifact_manifest,
            ),
        },
        summary,
        role_findings,
        observed_canisters,
        recommendations,
        blocked_actions: blocked_actions(),
        warnings: Vec::new(),
    })
}
