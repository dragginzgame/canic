//! Passive adoption profile and onboarding reports.

use crate::deployment_truth::{
    ArtifactSourceV1, CanisterControlClassV1, DeploymentInventoryV1, RoleArtifactManifestV1,
};
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const ADOPTION_REPORT_SCHEMA_VERSION: u32 = 1;

///
/// AdoptionReportRequest
///
#[derive(Clone, Debug)]
pub struct AdoptionReportRequest<'a> {
    pub report_id: &'a str,
    pub generated_at: &'a str,
    pub profile: AdoptionProfileV1,
    pub config_source: &'a str,
    pub inventory: Option<&'a DeploymentInventoryV1>,
    pub artifact_manifest: Option<&'a RoleArtifactManifestV1>,
    pub package_metadata: Vec<AdoptionPackageMetadataV1>,
}

///
/// AdoptionReportError
///
#[derive(Debug, Eq, Error, PartialEq)]
pub enum AdoptionReportError {
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("missing required [fleet].name in canic.toml")]
    MissingFleetName,
}

///
/// AdoptionProfileV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionProfileV1 {
    Brownfield,
    Partial,
    Standalone,
    LeafOnly,
    HybridExternalWasm,
    Minimal,
}

///
/// AdoptionReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub generated_at: String,
    pub fleet: String,
    pub profile: AdoptionProfileV1,
    pub inputs: AdoptionReportInputsV1,
    pub summary: AdoptionReportSummaryV1,
    pub role_findings: Vec<AdoptionRoleFindingV1>,
    pub observed_canisters: Vec<AdoptionObservedCanisterFindingV1>,
    pub recommendations: Vec<AdoptionRecommendationV1>,
    pub blocked_actions: Vec<String>,
    pub warnings: Vec<String>,
}

///
/// AdoptionReportInputsV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionReportInputsV1 {
    pub config_present: bool,
    pub inventory_id: Option<String>,
    pub artifact_manifest_id: Option<String>,
    pub package_metadata_count: usize,
    pub missing_or_stale_evidence: Vec<String>,
}

///
/// AdoptionReportSummaryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionReportSummaryV1 {
    pub managed_configured_roles: usize,
    pub declared_only_roles: usize,
    pub attached_unobserved_roles: usize,
    pub observed_only_canisters: usize,
    pub user_controlled_canisters: usize,
    pub external_controller_required: usize,
    pub evidence_conflicts: usize,
    pub mutating_actions_performed: usize,
}

///
/// AdoptionRoleFindingV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionRoleFindingV1 {
    pub fleet: String,
    pub role: String,
    pub classifications: Vec<AdoptionClassificationV1>,
    pub declaration_state: AdoptionDeclarationStateV1,
    pub topology_state: AdoptionTopologyStateV1,
    pub package_state: AdoptionPackageStateV1,
    pub observation_state: AdoptionObservationStateV1,
    pub authority_state: AdoptionAuthorityStateV1,
    pub artifact_state: AdoptionArtifactStateV1,
    pub evidence: Vec<String>,
    pub recommendations: Vec<AdoptionRecommendationV1>,
    pub warnings: Vec<String>,
}

///
/// AdoptionObservedCanisterFindingV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionObservedCanisterFindingV1 {
    pub canister_id: String,
    pub matched_fleet: Option<String>,
    pub matched_role: Option<String>,
    pub confidence: AdoptionMatchConfidenceV1,
    pub classifications: Vec<AdoptionClassificationV1>,
    pub controllers: Vec<String>,
    pub wasm_evidence: Option<String>,
    pub deployment_target_evidence: Option<String>,
    pub recommendations: Vec<AdoptionRecommendationV1>,
    pub warnings: Vec<String>,
}

///
/// AdoptionRecommendationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionRecommendationV1 {
    pub kind: String,
    pub severity: AdoptionRecommendationSeverityV1,
    pub description: String,
    pub suggested_action: Option<String>,
    pub suggested_action_effect: AdoptionSuggestedActionEffectV1,
    pub suggested_action_support: AdoptionSuggestedActionSupportV1,
    pub suggested_action_availability: AdoptionSuggestedActionAvailabilityV1,
    pub operator_action_requirement: AdoptionOperatorActionRequirementV1,
}

///
/// AdoptionPackageMetadataV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionPackageMetadataV1 {
    pub package: String,
    pub fleet: Option<String>,
    pub role: Option<String>,
}

///
/// AdoptionClassificationV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AdoptionClassificationV1 {
    Managed,
    DeclaredOnly,
    ObservedOnly,
    AttachedUnobserved,
    UserControlled,
    ExternalControllerRequired,
    ImportedPoolCandidate,
    EvidenceConflict,
}

///
/// AdoptionDeclarationStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionDeclarationStateV1 {
    Undeclared,
    Declared,
}

///
/// AdoptionTopologyStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionTopologyStateV1 {
    Unattached,
    Attached,
}

///
/// AdoptionObservationStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionObservationStateV1 {
    Unobserved,
    Observed,
    CandidateMatch,
    ConflictingMatch,
}

///
/// AdoptionAuthorityStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AdoptionAuthorityStateV1 {
    CanicAuthorized,
    UserControlled,
    External,
    Unknown,
}

///
/// AdoptionArtifactStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionArtifactStateV1 {
    CanicBuilt,
    ExternalWasm,
    Unknown,
}

///
/// AdoptionPackageStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionPackageStateV1 {
    NotPackageBacked,
    NotChecked,
    Matches,
    MissingFleet,
    MissingRole,
    Mismatch,
}

///
/// AdoptionMatchConfidenceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionMatchConfidenceV1 {
    None,
    Candidate,
    ExplicitEvidence,
    Conflict,
}

///
/// AdoptionRecommendationSeverityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionRecommendationSeverityV1 {
    Info,
    Warning,
    Blocked,
}

///
/// AdoptionSuggestedActionEffectV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionSuggestedActionEffectV1 {
    ReadOnly,
    MutatesState,
}

///
/// AdoptionSuggestedActionSupportV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionSuggestedActionSupportV1 {
    SupportedByAdoption,
    UnsupportedByAdoption,
}

///
/// AdoptionSuggestedActionAvailabilityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionSuggestedActionAvailabilityV1 {
    AllowedIn0500,
    BlockedIn0500,
}

///
/// AdoptionOperatorActionRequirementV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionOperatorActionRequirementV1 {
    Required,
    NotRequired,
}

struct DeclaredRoleFindingInput<'a> {
    fleet: &'a str,
    role: &'a CanisterRole,
    package: Option<&'a str>,
    attached: bool,
    observed: Option<&'a [&'a crate::deployment_truth::ObservedCanisterV1]>,
    duplicate_observation: bool,
    packages_by_path: &'a BTreeMap<String, AdoptionPackageMetadataV1>,
    artifact_state: Option<AdoptionArtifactStateV1>,
}

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
    let declared_roles = config.roles.keys().cloned().collect::<BTreeSet<_>>();

    let mut role_findings = Vec::new();
    let mut seen_roles = BTreeSet::new();

    for (role, declaration) in &config.roles {
        seen_roles.insert(role.as_str().to_string());
        role_findings.push(role_finding_for_declared_role(DeclaredRoleFindingInput {
            fleet: &fleet,
            role,
            package: declaration.package.as_deref(),
            attached: attached_roles.contains(role),
            observed: observed_by_role.get(role.as_str()).map(Vec::as_slice),
            duplicate_observation: observed_duplicate_roles.contains(role.as_str()),
            packages_by_path: &packages_by_path,
            artifact_state: artifacts_by_role.get(role.as_str()).copied(),
        }));
    }

    for (role, observed) in &observed_by_role {
        if seen_roles.contains(role) {
            continue;
        }
        role_findings.push(role_finding_for_observed_only_role(
            &fleet,
            role,
            observed,
            observed_duplicate_roles.contains(role),
            artifacts_by_role.get(role.as_str()).copied(),
        ));
    }

    role_findings.sort_by(|left, right| left.role.cmp(&right.role));

    let observed_canisters =
        observed_canister_findings(&fleet, request.inventory, &declared_roles, &attached_roles);
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
            missing_or_stale_evidence: missing_evidence(request.inventory),
        },
        summary,
        role_findings,
        observed_canisters,
        recommendations,
        blocked_actions: blocked_actions(),
        warnings: Vec::new(),
    })
}

fn role_finding_for_declared_role(input: DeclaredRoleFindingInput<'_>) -> AdoptionRoleFindingV1 {
    let role_name = input.role.as_str().to_string();
    let observed = input.observed.unwrap_or_default();
    let observed_any = !observed.is_empty();
    let mut classifications = BTreeSet::new();
    let mut evidence = Vec::new();
    let mut warnings = Vec::new();
    let mut recommendations = Vec::new();

    evidence.push("role declaration exists".to_string());
    if input.attached {
        evidence.push("topology attachment exists".to_string());
        classifications.insert(AdoptionClassificationV1::Managed);
    } else {
        evidence.push("no topology attachment exists".to_string());
        classifications.insert(AdoptionClassificationV1::DeclaredOnly);
        recommendations.push(attach_later_recommendation(input.fleet, &role_name));
    }

    if input.attached && !observed_any {
        classifications.insert(AdoptionClassificationV1::AttachedUnobserved);
        warnings.push("deployment-truth evidence does not confirm this attached role".to_string());
    }

    for canister in observed {
        evidence.push(format!("observed canister {}", canister.canister_id));
    }

    let authority_state = combined_authority_state(observed);
    if matches!(
        authority_state,
        AdoptionAuthorityStateV1::UserControlled | AdoptionAuthorityStateV1::External
    ) {
        classifications.insert(AdoptionClassificationV1::ExternalControllerRequired);
    }
    if matches!(authority_state, AdoptionAuthorityStateV1::UserControlled) {
        classifications.insert(AdoptionClassificationV1::UserControlled);
    }

    let package_state = package_state(
        input.package,
        input.fleet,
        &role_name,
        input.packages_by_path,
    );
    if matches!(
        package_state,
        AdoptionPackageStateV1::MissingFleet
            | AdoptionPackageStateV1::MissingRole
            | AdoptionPackageStateV1::Mismatch
    ) {
        classifications.insert(AdoptionClassificationV1::EvidenceConflict);
        warnings.push("package metadata does not match declared fleet role".to_string());
    }

    if input.duplicate_observation {
        classifications.insert(AdoptionClassificationV1::EvidenceConflict);
        warnings.push("deployment evidence contains conflicting role facts".to_string());
    }

    AdoptionRoleFindingV1 {
        fleet: input.fleet.to_string(),
        role: role_name,
        classifications: classifications.into_iter().collect(),
        declaration_state: AdoptionDeclarationStateV1::Declared,
        topology_state: if input.attached {
            AdoptionTopologyStateV1::Attached
        } else {
            AdoptionTopologyStateV1::Unattached
        },
        package_state,
        observation_state: observation_state(observed_any, input.duplicate_observation),
        authority_state,
        artifact_state: input
            .artifact_state
            .unwrap_or_else(|| artifact_state_from_observed(observed)),
        evidence,
        recommendations,
        warnings,
    }
}

fn role_finding_for_observed_only_role(
    fleet: &str,
    role: &str,
    observed: &[&crate::deployment_truth::ObservedCanisterV1],
    duplicate_observation: bool,
    artifact_state: Option<AdoptionArtifactStateV1>,
) -> AdoptionRoleFindingV1 {
    let mut classifications = BTreeSet::new();
    classifications.insert(AdoptionClassificationV1::ObservedOnly);
    if duplicate_observation {
        classifications.insert(AdoptionClassificationV1::EvidenceConflict);
    }

    let authority_state = combined_authority_state(observed);
    if matches!(authority_state, AdoptionAuthorityStateV1::UserControlled) {
        classifications.insert(AdoptionClassificationV1::UserControlled);
    }
    if matches!(
        authority_state,
        AdoptionAuthorityStateV1::UserControlled | AdoptionAuthorityStateV1::External
    ) {
        classifications.insert(AdoptionClassificationV1::ExternalControllerRequired);
    }

    AdoptionRoleFindingV1 {
        fleet: fleet.to_string(),
        role: role.to_string(),
        classifications: classifications.into_iter().collect(),
        declaration_state: AdoptionDeclarationStateV1::Undeclared,
        topology_state: AdoptionTopologyStateV1::Unattached,
        package_state: AdoptionPackageStateV1::NotPackageBacked,
        observation_state: observation_state(true, duplicate_observation),
        authority_state,
        artifact_state: artifact_state.unwrap_or_else(|| artifact_state_from_observed(observed)),
        evidence: observed
            .iter()
            .map(|canister| format!("observed canister {}", canister.canister_id))
            .collect(),
        recommendations: vec![declare_role_recommendation(fleet, role)],
        warnings: Vec::new(),
    }
}

fn observed_canister_findings(
    fleet: &str,
    inventory: Option<&DeploymentInventoryV1>,
    declarations: &BTreeSet<CanisterRole>,
    attached_roles: &BTreeSet<CanisterRole>,
) -> Vec<AdoptionObservedCanisterFindingV1> {
    let Some(inventory) = inventory else {
        return Vec::new();
    };

    let mut findings = Vec::new();
    for canister in &inventory.observed_canisters {
        let role = canister.role.as_deref();
        let declared =
            role.is_some_and(|role| declarations.contains(&CanisterRole::owned(role.to_string())));
        let attached = role
            .is_some_and(|role| attached_roles.contains(&CanisterRole::owned(role.to_string())));
        let mut classifications = BTreeSet::new();
        if role.is_none() || !declared {
            classifications.insert(AdoptionClassificationV1::ObservedOnly);
        }
        if matches!(
            authority_state_for_control_class(canister.control_class),
            AdoptionAuthorityStateV1::UserControlled
        ) {
            classifications.insert(AdoptionClassificationV1::UserControlled);
        }
        if matches!(
            authority_state_for_control_class(canister.control_class),
            AdoptionAuthorityStateV1::UserControlled | AdoptionAuthorityStateV1::External
        ) {
            classifications.insert(AdoptionClassificationV1::ExternalControllerRequired);
        }

        findings.push(AdoptionObservedCanisterFindingV1 {
            canister_id: canister.canister_id.clone(),
            matched_fleet: role.map(|_| fleet.to_string()),
            matched_role: role.map(str::to_string),
            confidence: match (role, declared, attached) {
                (Some(_), true, true) => AdoptionMatchConfidenceV1::ExplicitEvidence,
                (Some(_), _, _) => AdoptionMatchConfidenceV1::Candidate,
                (None, _, _) => AdoptionMatchConfidenceV1::None,
            },
            classifications: classifications.into_iter().collect(),
            controllers: canister.controllers.clone(),
            wasm_evidence: canister
                .module_hash
                .as_ref()
                .map(|hash| format!("module_hash={hash}")),
            deployment_target_evidence: Some(inventory.inventory_id.clone()),
            recommendations: match (role, declared) {
                (Some(role), false) => vec![declare_role_recommendation(fleet, role)],
                _ => Vec::new(),
            },
            warnings: Vec::new(),
        });
    }

    for pool in &inventory.observed_pool {
        findings.push(AdoptionObservedCanisterFindingV1 {
            canister_id: pool.canister_id.clone(),
            matched_fleet: pool.role.as_ref().map(|_| fleet.to_string()),
            matched_role: pool.role.clone(),
            confidence: AdoptionMatchConfidenceV1::Candidate,
            classifications: vec![AdoptionClassificationV1::ImportedPoolCandidate],
            controllers: Vec::new(),
            wasm_evidence: None,
            deployment_target_evidence: Some(format!("pool={}", pool.pool)),
            recommendations: Vec::new(),
            warnings: vec!["pool import is outside 0.50.0".to_string()],
        });
    }

    findings.sort_by(|left, right| left.canister_id.cmp(&right.canister_id));
    findings
}

fn package_state(
    package: Option<&str>,
    fleet: &str,
    role: &str,
    packages_by_path: &BTreeMap<String, AdoptionPackageMetadataV1>,
) -> AdoptionPackageStateV1 {
    let Some(package) = package else {
        return AdoptionPackageStateV1::NotPackageBacked;
    };
    let Some(metadata) = packages_by_path.get(package) else {
        return AdoptionPackageStateV1::NotChecked;
    };
    if metadata.fleet.is_none() {
        return AdoptionPackageStateV1::MissingFleet;
    }
    if metadata.role.is_none() {
        return AdoptionPackageStateV1::MissingRole;
    }
    if metadata.fleet.as_deref() == Some(fleet) && metadata.role.as_deref() == Some(role) {
        AdoptionPackageStateV1::Matches
    } else {
        AdoptionPackageStateV1::Mismatch
    }
}

fn observed_canisters_by_role(
    inventory: Option<&DeploymentInventoryV1>,
) -> BTreeMap<String, Vec<&crate::deployment_truth::ObservedCanisterV1>> {
    let mut observed = BTreeMap::<String, Vec<&crate::deployment_truth::ObservedCanisterV1>>::new();
    let Some(inventory) = inventory else {
        return observed;
    };

    for canister in &inventory.observed_canisters {
        if let Some(role) = &canister.role {
            observed.entry(role.clone()).or_default().push(canister);
        }
    }
    observed
}

fn duplicate_observed_roles(
    observed_by_role: &BTreeMap<String, Vec<&crate::deployment_truth::ObservedCanisterV1>>,
) -> BTreeSet<String> {
    observed_by_role
        .iter()
        .filter(|(_, canisters)| canisters.len() > 1)
        .map(|(role, _)| role.clone())
        .collect()
}

fn package_metadata_by_path(
    metadata: Vec<AdoptionPackageMetadataV1>,
) -> BTreeMap<String, AdoptionPackageMetadataV1> {
    metadata
        .into_iter()
        .map(|metadata| (metadata.package.clone(), metadata))
        .collect()
}

fn artifact_states_by_role(
    manifest: Option<&RoleArtifactManifestV1>,
    inventory: Option<&DeploymentInventoryV1>,
) -> BTreeMap<String, AdoptionArtifactStateV1> {
    let mut states = BTreeMap::new();

    if let Some(manifest) = manifest {
        for artifact in &manifest.role_artifacts {
            states.insert(
                artifact.role.clone(),
                match artifact.source {
                    ArtifactSourceV1::External | ArtifactSourceV1::Unknown => {
                        AdoptionArtifactStateV1::ExternalWasm
                    }
                    ArtifactSourceV1::LocalBuild
                    | ArtifactSourceV1::ReleaseSet
                    | ArtifactSourceV1::WasmStore => AdoptionArtifactStateV1::CanicBuilt,
                },
            );
        }
    }

    if let Some(inventory) = inventory {
        for artifact in &inventory.observed_artifacts {
            states
                .entry(artifact.role.clone())
                .or_insert(match artifact.source {
                    ArtifactSourceV1::External | ArtifactSourceV1::Unknown => {
                        AdoptionArtifactStateV1::ExternalWasm
                    }
                    ArtifactSourceV1::LocalBuild
                    | ArtifactSourceV1::ReleaseSet
                    | ArtifactSourceV1::WasmStore => AdoptionArtifactStateV1::CanicBuilt,
                });
        }
    }

    states
}

fn combined_authority_state(
    observed: &[&crate::deployment_truth::ObservedCanisterV1],
) -> AdoptionAuthorityStateV1 {
    let mut states = observed
        .iter()
        .map(|canister| authority_state_for_control_class(canister.control_class))
        .collect::<BTreeSet<_>>();
    if states.is_empty() {
        return AdoptionAuthorityStateV1::Unknown;
    }
    if states.remove(&AdoptionAuthorityStateV1::UserControlled) {
        return AdoptionAuthorityStateV1::UserControlled;
    }
    if states.remove(&AdoptionAuthorityStateV1::External) {
        return AdoptionAuthorityStateV1::External;
    }
    if states.remove(&AdoptionAuthorityStateV1::Unknown) {
        return AdoptionAuthorityStateV1::Unknown;
    }
    AdoptionAuthorityStateV1::CanicAuthorized
}

const fn authority_state_for_control_class(
    control_class: CanisterControlClassV1,
) -> AdoptionAuthorityStateV1 {
    match control_class {
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::CanicManagedPool => {
            AdoptionAuthorityStateV1::CanicAuthorized
        }
        CanisterControlClassV1::UserControlled => AdoptionAuthorityStateV1::UserControlled,
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::JointlyControlled => {
            AdoptionAuthorityStateV1::External
        }
        CanisterControlClassV1::UnknownUnsafe => AdoptionAuthorityStateV1::Unknown,
    }
}

const fn observation_state(observed: bool, conflict: bool) -> AdoptionObservationStateV1 {
    match (observed, conflict) {
        (_, true) => AdoptionObservationStateV1::ConflictingMatch,
        (true, false) => AdoptionObservationStateV1::Observed,
        (false, false) => AdoptionObservationStateV1::Unobserved,
    }
}

fn artifact_state_from_observed(
    observed: &[&crate::deployment_truth::ObservedCanisterV1],
) -> AdoptionArtifactStateV1 {
    if observed
        .iter()
        .any(|canister| canister.module_hash.is_some())
    {
        AdoptionArtifactStateV1::ExternalWasm
    } else {
        AdoptionArtifactStateV1::Unknown
    }
}

fn missing_evidence(inventory: Option<&DeploymentInventoryV1>) -> Vec<String> {
    if inventory.is_some() {
        Vec::new()
    } else {
        vec!["deployment inventory was not supplied".to_string()]
    }
}

fn report_summary(
    role_findings: &[AdoptionRoleFindingV1],
    observed_findings: &[AdoptionObservedCanisterFindingV1],
) -> AdoptionReportSummaryV1 {
    AdoptionReportSummaryV1 {
        managed_configured_roles: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::Managed)
            })
            .count(),
        declared_only_roles: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::DeclaredOnly)
            })
            .count(),
        attached_unobserved_roles: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::AttachedUnobserved)
            })
            .count(),
        observed_only_canisters: observed_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::ObservedOnly)
            })
            .count(),
        user_controlled_canisters: observed_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::UserControlled)
            })
            .count(),
        external_controller_required: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::ExternalControllerRequired)
            })
            .count(),
        evidence_conflicts: role_findings
            .iter()
            .filter(|finding| {
                finding
                    .classifications
                    .contains(&AdoptionClassificationV1::EvidenceConflict)
            })
            .count(),
        mutating_actions_performed: 0,
    }
}

fn declare_role_recommendation(fleet: &str, role: &str) -> AdoptionRecommendationV1 {
    AdoptionRecommendationV1 {
        kind: "declare_role".to_string(),
        severity: AdoptionRecommendationSeverityV1::Info,
        description: format!("declare observed role candidate {fleet}.{role} before attachment"),
        suggested_action: Some(format!(
            "canic fleet role declare {fleet} {role} --package <path>"
        )),
        suggested_action_effect: AdoptionSuggestedActionEffectV1::MutatesState,
        suggested_action_support: AdoptionSuggestedActionSupportV1::UnsupportedByAdoption,
        suggested_action_availability: AdoptionSuggestedActionAvailabilityV1::BlockedIn0500,
        operator_action_requirement: AdoptionOperatorActionRequirementV1::Required,
    }
}

fn attach_later_recommendation(fleet: &str, role: &str) -> AdoptionRecommendationV1 {
    AdoptionRecommendationV1 {
        kind: "attach_role_later".to_string(),
        severity: AdoptionRecommendationSeverityV1::Info,
        description: format!("attach {fleet}.{role} explicitly only when topology is ready"),
        suggested_action: Some(format!(
            "canic fleet role attach {fleet} {role} --subnet <subnet>"
        )),
        suggested_action_effect: AdoptionSuggestedActionEffectV1::MutatesState,
        suggested_action_support: AdoptionSuggestedActionSupportV1::UnsupportedByAdoption,
        suggested_action_availability: AdoptionSuggestedActionAvailabilityV1::BlockedIn0500,
        operator_action_requirement: AdoptionOperatorActionRequirementV1::Required,
    }
}

fn blocked_actions() -> Vec<String> {
    [
        "controller changes",
        "topology attachment",
        "pool import",
        "install",
        "upgrade",
        "reinstall",
        "stop",
        "start",
        "delete",
        "deploy",
        "promote",
        "rollback",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deployment_truth::{
        ArtifactDigestSourceV1, DeploymentInventoryV1, DeploymentRootObservationSourceV1,
        DeploymentRootObservationV1, LocalDeploymentConfigV1, ObservationStatusV1,
        ObservedArtifactV1, ObservedCanisterV1, ObservedPoolCanisterV1, RoleArtifactManifestV1,
        RoleArtifactV1, VerifierReadinessObservationV1,
    };

    const CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.api]
kind = "canister"
package = "api"

[roles.store]
kind = "canister"
package = "store"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.api]
kind = "singleton"
"#;

    #[test]
    fn adoption_report_preserves_declared_only_as_non_deployable() {
        let report = report(CONFIG, None, Vec::new());
        let store = role(&report, "store");

        assert_eq!(
            store.declaration_state,
            AdoptionDeclarationStateV1::Declared
        );
        assert_eq!(store.topology_state, AdoptionTopologyStateV1::Unattached);
        assert!(
            store
                .classifications
                .contains(&AdoptionClassificationV1::DeclaredOnly)
        );
        assert_eq!(report.summary.declared_only_roles, 1);
        assert_eq!(report.summary.mutating_actions_performed, 0);
        assert!(store.recommendations.iter().all(|recommendation| {
            recommendation.suggested_action_availability
                == AdoptionSuggestedActionAvailabilityV1::BlockedIn0500
        }));
        assert!(report.blocked_actions.contains(&"install".to_string()));
    }

    #[test]
    fn adoption_report_reports_attached_unobserved_without_teardown_inference() {
        let report = report(CONFIG, None, Vec::new());
        let api = role(&report, "api");

        assert!(
            api.classifications
                .contains(&AdoptionClassificationV1::Managed)
        );
        assert!(
            api.classifications
                .contains(&AdoptionClassificationV1::AttachedUnobserved)
        );
        assert_eq!(
            api.observation_state,
            AdoptionObservationStateV1::Unobserved
        );
        assert!(
            api.warnings
                .iter()
                .any(|warning| warning.contains("does not confirm"))
        );
    }

    #[test]
    fn adoption_report_classifies_observed_only_user_controlled_canister() {
        let inventory = inventory(vec![observed_canister(
            "aaaaa-aa",
            Some("legacy"),
            CanisterControlClassV1::UserControlled,
            Some("legacy-hash"),
        )]);
        let report = report(CONFIG, Some(&inventory), Vec::new());
        let legacy = role(&report, "legacy");

        assert_eq!(
            legacy.declaration_state,
            AdoptionDeclarationStateV1::Undeclared
        );
        assert!(
            legacy
                .classifications
                .contains(&AdoptionClassificationV1::ObservedOnly)
        );
        assert!(
            legacy
                .classifications
                .contains(&AdoptionClassificationV1::UserControlled)
        );
        assert!(
            legacy
                .classifications
                .contains(&AdoptionClassificationV1::ExternalControllerRequired)
        );
        assert_eq!(report.summary.observed_only_canisters, 1);
        assert_eq!(report.summary.user_controlled_canisters, 1);
        assert!(
            report
                .recommendations
                .iter()
                .any(|recommendation| recommendation.kind == "declare_role"
                    && recommendation.suggested_action_effect
                        == AdoptionSuggestedActionEffectV1::MutatesState
                    && recommendation.suggested_action_support
                        == AdoptionSuggestedActionSupportV1::UnsupportedByAdoption)
        );
    }

    #[test]
    fn adoption_report_keeps_managed_separate_from_authority() {
        let inventory = inventory(vec![observed_canister(
            "aaaaa-aa",
            Some("api"),
            CanisterControlClassV1::UserControlled,
            Some("api-hash"),
        )]);
        let report = report(CONFIG, Some(&inventory), Vec::new());
        let api = role(&report, "api");

        assert!(
            api.classifications
                .contains(&AdoptionClassificationV1::Managed)
        );
        assert!(
            api.classifications
                .contains(&AdoptionClassificationV1::ExternalControllerRequired)
        );
        assert_eq!(
            api.authority_state,
            AdoptionAuthorityStateV1::UserControlled
        );
    }

    #[test]
    fn adoption_report_marks_role_only_package_metadata_as_conflict() {
        let report = report(
            CONFIG,
            None,
            vec![AdoptionPackageMetadataV1 {
                package: "store".to_string(),
                fleet: None,
                role: Some("store".to_string()),
            }],
        );
        let store = role(&report, "store");

        assert_eq!(store.package_state, AdoptionPackageStateV1::MissingFleet);
        assert!(
            store
                .classifications
                .contains(&AdoptionClassificationV1::EvidenceConflict)
        );
    }

    #[test]
    fn adoption_report_marks_duplicate_observed_role_as_evidence_conflict() {
        let inventory = inventory(vec![
            observed_canister(
                "aaaaa-aa",
                Some("api"),
                CanisterControlClassV1::DeploymentControlled,
                Some("api-hash-a"),
            ),
            observed_canister(
                "bbbbb-bb",
                Some("api"),
                CanisterControlClassV1::DeploymentControlled,
                Some("api-hash-b"),
            ),
        ]);
        let report = report(CONFIG, Some(&inventory), Vec::new());
        let api = role(&report, "api");

        assert_eq!(
            api.observation_state,
            AdoptionObservationStateV1::ConflictingMatch
        );
        assert!(
            api.classifications
                .contains(&AdoptionClassificationV1::EvidenceConflict)
        );
        assert_eq!(report.summary.evidence_conflicts, 1);
    }

    #[test]
    fn adoption_report_classifies_pool_candidates_as_resources() {
        let mut inventory = inventory(Vec::new());
        inventory.observed_pool.push(ObservedPoolCanisterV1 {
            pool: "users".to_string(),
            canister_id: "ccccc-cc".to_string(),
            role: Some("user_shard".to_string()),
            control_class: CanisterControlClassV1::UnknownUnsafe,
        });

        let report = report(CONFIG, Some(&inventory), Vec::new());
        let pool = report
            .observed_canisters
            .iter()
            .find(|finding| finding.canister_id == "ccccc-cc")
            .expect("pool candidate finding");

        assert!(
            pool.classifications
                .contains(&AdoptionClassificationV1::ImportedPoolCandidate)
        );
        assert_eq!(pool.matched_role.as_deref(), Some("user_shard"));
    }

    #[test]
    fn adoption_report_round_trips_through_json() {
        let manifest = RoleArtifactManifestV1 {
            schema_version: 1,
            manifest_id: "manifest-1".to_string(),
            network: "local".to_string(),
            artifact_root: None,
            role_artifacts: vec![RoleArtifactV1 {
                role: "api".to_string(),
                source: ArtifactSourceV1::LocalBuild,
                build_profile: "fast".to_string(),
                wasm_path: None,
                wasm_gz_path: None,
                wasm_gz_size_bytes: None,
                wasm_sha256: None,
                wasm_gz_sha256: None,
                wasm_gz_sha256_source: None,
                observed_wasm_gz_file_sha256: None,
                observed_wasm_gz_file_sha256_source: None,
                installed_module_hash: None,
                candid_path: None,
                candid_sha256: None,
                raw_config_sha256: None,
                canonical_embedded_config_sha256: None,
                embedded_topology_sha256: None,
                builder_version: None,
                rust_toolchain: None,
                package_version: None,
            }],
            unresolved_artifacts: Vec::new(),
        };
        let report = adoption_report_from_config_source(AdoptionReportRequest {
            report_id: "adoption-1",
            generated_at: "2026-05-30T00:00:00Z",
            profile: AdoptionProfileV1::Brownfield,
            config_source: CONFIG,
            inventory: None,
            artifact_manifest: Some(&manifest),
            package_metadata: Vec::new(),
        })
        .expect("adoption report");

        let encoded = serde_json::to_string(&report).expect("encode report");
        let decoded = serde_json::from_str::<AdoptionReportV1>(&encoded).expect("decode report");

        assert_eq!(decoded, report);
        assert_eq!(
            role(&decoded, "api").artifact_state,
            AdoptionArtifactStateV1::CanicBuilt
        );
    }

    fn report(
        config_source: &str,
        inventory: Option<&DeploymentInventoryV1>,
        package_metadata: Vec<AdoptionPackageMetadataV1>,
    ) -> AdoptionReportV1 {
        adoption_report_from_config_source(AdoptionReportRequest {
            report_id: "adoption-1",
            generated_at: "2026-05-30T00:00:00Z",
            profile: AdoptionProfileV1::Brownfield,
            config_source,
            inventory,
            artifact_manifest: None,
            package_metadata,
        })
        .expect("adoption report")
    }

    fn role<'a>(report: &'a AdoptionReportV1, role: &str) -> &'a AdoptionRoleFindingV1 {
        report
            .role_findings
            .iter()
            .find(|finding| finding.role == role)
            .expect("role finding")
    }

    fn inventory(observed_canisters: Vec<ObservedCanisterV1>) -> DeploymentInventoryV1 {
        DeploymentInventoryV1 {
            schema_version: 1,
            inventory_id: "inventory-1".to_string(),
            observed_at: "2026-05-30T00:00:00Z".to_string(),
            observed_identity: None,
            observed_root: Some(DeploymentRootObservationV1 {
                deployment_name: "demo-dev".to_string(),
                network: "local".to_string(),
                fleet_template: "demo".to_string(),
                root_principal: "aaaaa-aa".to_string(),
                observed_canister_id: "aaaaa-aa".to_string(),
                observation_source: DeploymentRootObservationSourceV1::LocalDeploymentState,
                control_class: CanisterControlClassV1::DeploymentControlled,
                controllers: vec!["aaaaa-aa".to_string()],
                module_hash: None,
                status: Some("running".to_string()),
                role_assignment_source: Some("local-state".to_string()),
            }),
            local_config: LocalDeploymentConfigV1 {
                config_path: Some("fleets/demo/canic.toml".to_string()),
                raw_config_sha256: None,
                canonical_embedded_config_sha256: None,
            },
            observed_canisters,
            observed_pool: Vec::new(),
            observed_artifacts: vec![ObservedArtifactV1 {
                role: "legacy".to_string(),
                artifact_path: "observed:legacy".to_string(),
                file_sha256: None,
                file_sha256_source: Some(ArtifactDigestSourceV1::InstalledModuleHash),
                payload_sha256: None,
                payload_size_bytes: None,
                source: ArtifactSourceV1::External,
            }],
            observed_verifier_readiness: VerifierReadinessObservationV1 {
                status: ObservationStatusV1::NotObserved,
                role_epochs: Vec::new(),
            },
            unresolved_observations: Vec::new(),
        }
    }

    fn observed_canister(
        canister_id: &str,
        role: Option<&str>,
        control_class: CanisterControlClassV1,
        module_hash: Option<&str>,
    ) -> ObservedCanisterV1 {
        ObservedCanisterV1 {
            canister_id: canister_id.to_string(),
            role: role.map(str::to_string),
            control_class,
            controllers: vec!["controller".to_string()],
            module_hash: module_hash.map(str::to_string),
            status: Some("running".to_string()),
            root_trust_anchor: Some("root".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: role.map(|_| "explicit-test-evidence".to_string()),
        }
    }
}
