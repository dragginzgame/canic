//! Passive adoption profile and onboarding reports.

use crate::deployment_truth::{
    ArtifactSourceV1, CanisterControlClassV1, DeploymentInventoryV1, RoleArtifactManifestV1,
};
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
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

impl FromStr for AdoptionProfileV1 {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "brownfield" => Ok(Self::Brownfield),
            "partial" => Ok(Self::Partial),
            "standalone" => Ok(Self::Standalone),
            "leaf-only" => Ok(Self::LeafOnly),
            "hybrid-external-wasm" => Ok(Self::HybridExternalWasm),
            "minimal" => Ok(Self::Minimal),
            other => Err(format!("invalid adoption profile: {other}")),
        }
    }
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
    UndeclaredRole,
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
    profile: AdoptionProfileV1,
    fleet: &'a str,
    role: &'a CanisterRole,
    package: &'a str,
    attached: bool,
    observed: Option<&'a [&'a crate::deployment_truth::ObservedCanisterV1]>,
    duplicate_observation: bool,
    packages_by_path: &'a BTreeMap<String, AdoptionPackageMetadataV1>,
    artifact_state: Option<AdoptionArtifactStateV1>,
    artifact_conflict: bool,
    artifact_evidence: Option<&'a [String]>,
}

struct ObservedOnlyRoleFindingInput<'a> {
    profile: AdoptionProfileV1,
    fleet: &'a str,
    role: &'a str,
    observed: &'a [&'a crate::deployment_truth::ObservedCanisterV1],
    duplicate_observation: bool,
    artifact_state: Option<AdoptionArtifactStateV1>,
    artifact_conflict: bool,
    artifact_evidence: Option<&'a [String]>,
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
        if input.profile == AdoptionProfileV1::LeafOnly
            && is_leaf_only_authority_sensitive_role(&role_name)
        {
            warnings.push(
                "leaf-only profile leaves authority-sensitive declared roles unattached"
                    .to_string(),
            );
        } else {
            recommendations.push(attach_later_recommendation(input.fleet, &role_name));
        }
    }

    if input.attached && !observed_any {
        classifications.insert(AdoptionClassificationV1::AttachedUnobserved);
        warnings.push("deployment-truth evidence does not confirm this attached role".to_string());
    }

    for canister in observed {
        evidence.push(format!("observed canister {}", canister.canister_id));
        if let Some(hash) = &canister.module_hash {
            evidence.push(format!("observed canister module_hash={hash}"));
        }
    }
    if let Some(artifact_evidence) = input.artifact_evidence {
        evidence.extend(artifact_evidence.iter().cloned());
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

    if input.artifact_conflict {
        classifications.insert(AdoptionClassificationV1::EvidenceConflict);
        warnings.push("artifact evidence contains conflicting role facts".to_string());
    }

    let artifact_state = input
        .artifact_state
        .unwrap_or_else(|| artifact_state_from_observed(observed));
    if input.profile == AdoptionProfileV1::HybridExternalWasm
        && artifact_state == AdoptionArtifactStateV1::ExternalWasm
    {
        warnings.push(
            "external Wasm evidence is reported only; artifact registry import is outside adoption reporting"
                .to_string(),
        );
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
        artifact_state,
        evidence,
        recommendations,
        warnings,
    }
}

fn role_finding_for_observed_only_role(
    input: ObservedOnlyRoleFindingInput<'_>,
) -> AdoptionRoleFindingV1 {
    let mut classifications = BTreeSet::new();
    classifications.insert(AdoptionClassificationV1::ObservedOnly);
    if input.duplicate_observation {
        classifications.insert(AdoptionClassificationV1::EvidenceConflict);
    }
    if input.artifact_conflict {
        classifications.insert(AdoptionClassificationV1::EvidenceConflict);
    }

    let authority_state = combined_authority_state(input.observed);
    if matches!(authority_state, AdoptionAuthorityStateV1::UserControlled) {
        classifications.insert(AdoptionClassificationV1::UserControlled);
    }
    if matches!(
        authority_state,
        AdoptionAuthorityStateV1::UserControlled | AdoptionAuthorityStateV1::External
    ) {
        classifications.insert(AdoptionClassificationV1::ExternalControllerRequired);
    }

    let artifact_state = input
        .artifact_state
        .unwrap_or_else(|| artifact_state_from_observed(input.observed));
    let mut evidence = input
        .observed
        .iter()
        .flat_map(|canister| {
            let mut evidence = vec![format!("observed canister {}", canister.canister_id)];
            if let Some(hash) = &canister.module_hash {
                evidence.push(format!("observed canister module_hash={hash}"));
            }
            evidence
        })
        .collect::<Vec<_>>();
    if let Some(artifact_evidence) = input.artifact_evidence {
        evidence.extend(artifact_evidence.iter().cloned());
    }

    let mut warnings = observed_only_warnings(input.profile, input.role);
    if input.artifact_conflict {
        warnings.push("artifact evidence contains conflicting role facts".to_string());
    }
    if input.profile == AdoptionProfileV1::HybridExternalWasm
        && artifact_state == AdoptionArtifactStateV1::ExternalWasm
    {
        warnings.push(
            "external Wasm evidence is reported only; artifact registry import is outside adoption reporting"
                .to_string(),
        );
    }

    AdoptionRoleFindingV1 {
        fleet: input.fleet.to_string(),
        role: input.role.to_string(),
        classifications: classifications.into_iter().collect(),
        declaration_state: AdoptionDeclarationStateV1::Undeclared,
        topology_state: AdoptionTopologyStateV1::Unattached,
        package_state: AdoptionPackageStateV1::UndeclaredRole,
        observation_state: observation_state(true, input.duplicate_observation),
        authority_state,
        artifact_state,
        evidence,
        recommendations: observed_only_recommendations(
            input.profile,
            input.fleet,
            input.role,
            authority_state,
        ),
        warnings,
    }
}

fn observed_canister_findings(
    profile: AdoptionProfileV1,
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
                (Some(role), false) => observed_only_recommendations(
                    profile,
                    fleet,
                    role,
                    authority_state_for_control_class(canister.control_class),
                ),
                _ => Vec::new(),
            },
            warnings: role
                .map(|role| observed_only_warnings(profile, role))
                .unwrap_or_default(),
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

fn observed_only_recommendations(
    profile: AdoptionProfileV1,
    fleet: &str,
    role: &str,
    authority_state: AdoptionAuthorityStateV1,
) -> Vec<AdoptionRecommendationV1> {
    if profile == AdoptionProfileV1::LeafOnly && is_leaf_only_authority_sensitive_role(role) {
        return Vec::new();
    }

    if authority_state != AdoptionAuthorityStateV1::CanicAuthorized {
        return vec![review_authority_before_declaration_recommendation(
            fleet,
            role,
            authority_state,
        )];
    }

    vec![declare_role_recommendation(fleet, role)]
}

fn observed_only_warnings(profile: AdoptionProfileV1, role: &str) -> Vec<String> {
    if profile == AdoptionProfileV1::LeafOnly && is_leaf_only_authority_sensitive_role(role) {
        return vec![
            "leaf-only profile leaves authority-sensitive observed roles external".to_string(),
        ];
    }

    Vec::new()
}

fn is_leaf_only_authority_sensitive_role(role: &str) -> bool {
    matches!(role, "root" | "governance" | "governance_root")
}

fn package_state(
    package: &str,
    fleet: &str,
    role: &str,
    packages_by_path: &BTreeMap<String, AdoptionPackageMetadataV1>,
) -> AdoptionPackageStateV1 {
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
                artifact_state_for_source(artifact.source),
            );
        }
    }

    if let Some(inventory) = inventory {
        for artifact in &inventory.observed_artifacts {
            states
                .entry(artifact.role.clone())
                .or_insert_with(|| artifact_state_for_source(artifact.source));
        }
    }

    states
}

fn artifact_conflict_roles(
    manifest: Option<&RoleArtifactManifestV1>,
    inventory: Option<&DeploymentInventoryV1>,
) -> BTreeSet<String> {
    let mut manifest_states = BTreeMap::new();
    let mut conflict_roles = BTreeSet::new();

    if let Some(manifest) = manifest {
        for artifact in &manifest.role_artifacts {
            let state = artifact_state_for_source(artifact.source);
            if manifest_states
                .insert(artifact.role.clone(), state)
                .is_some_and(|previous| previous != state)
            {
                conflict_roles.insert(artifact.role.clone());
            }
        }
    }

    if let Some(inventory) = inventory {
        for artifact in &inventory.observed_artifacts {
            let state = artifact_state_for_source(artifact.source);
            if manifest_states
                .get(&artifact.role)
                .is_some_and(|previous| *previous != state)
            {
                conflict_roles.insert(artifact.role.clone());
            }
        }
    }

    conflict_roles
}

fn artifact_evidence_by_role(
    manifest: Option<&RoleArtifactManifestV1>,
    inventory: Option<&DeploymentInventoryV1>,
) -> BTreeMap<String, Vec<String>> {
    let mut evidence = BTreeMap::<String, Vec<String>>::new();

    if let Some(manifest) = manifest {
        for artifact in &manifest.role_artifacts {
            let role_evidence = evidence.entry(artifact.role.clone()).or_default();
            role_evidence.push(format!(
                "artifact manifest source={}",
                artifact_source_label(artifact.source)
            ));
            if let Some(hash) = &artifact.installed_module_hash {
                role_evidence.push(format!("artifact manifest installed_module_hash={hash}"));
            }
            if let Some(hash) = &artifact.wasm_sha256 {
                role_evidence.push(format!("artifact manifest wasm_sha256={hash}"));
            }
            if let Some(hash) = &artifact.wasm_gz_sha256 {
                role_evidence.push(format!("artifact manifest wasm_gz_sha256={hash}"));
            }
        }
    }

    if let Some(inventory) = inventory {
        for artifact in &inventory.observed_artifacts {
            let role_evidence = evidence.entry(artifact.role.clone()).or_default();
            role_evidence.push(format!(
                "observed artifact source={} path={}",
                artifact_source_label(artifact.source),
                artifact.artifact_path
            ));
            if let Some(hash) = &artifact.file_sha256 {
                role_evidence.push(format!("observed artifact file_sha256={hash}"));
            }
            if let Some(hash) = &artifact.payload_sha256 {
                role_evidence.push(format!("observed artifact payload_sha256={hash}"));
            }
            if let Some(size) = artifact.payload_size_bytes {
                role_evidence.push(format!("observed artifact payload_size_bytes={size}"));
            }
        }
    }

    evidence
}

const fn artifact_state_for_source(source: ArtifactSourceV1) -> AdoptionArtifactStateV1 {
    match source {
        ArtifactSourceV1::External | ArtifactSourceV1::Unknown => {
            AdoptionArtifactStateV1::ExternalWasm
        }
        ArtifactSourceV1::LocalBuild
        | ArtifactSourceV1::ReleaseSet
        | ArtifactSourceV1::WasmStore => AdoptionArtifactStateV1::CanicBuilt,
    }
}

const fn artifact_source_label(source: ArtifactSourceV1) -> &'static str {
    match source {
        ArtifactSourceV1::LocalBuild => "local-build",
        ArtifactSourceV1::ReleaseSet => "release-set",
        ArtifactSourceV1::WasmStore => "wasm-store",
        ArtifactSourceV1::External => "external",
        ArtifactSourceV1::Unknown => "unknown",
    }
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

fn missing_evidence(
    inventory: Option<&DeploymentInventoryV1>,
    artifact_manifest: Option<&RoleArtifactManifestV1>,
) -> Vec<String> {
    let mut evidence = Vec::new();

    if let Some(inventory) = inventory {
        evidence.extend(inventory.unresolved_observations.iter().map(|gap| {
            format!(
                "unresolved inventory observation {}: {}",
                gap.key, gap.description
            )
        }));
    } else {
        evidence.push("deployment inventory was not supplied".to_string());
    }

    if let Some(manifest) = artifact_manifest {
        evidence.extend(manifest.unresolved_artifacts.iter().map(|gap| {
            format!(
                "unresolved artifact evidence {}: {}",
                gap.key, gap.description
            )
        }));
    }

    evidence
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

fn review_authority_before_declaration_recommendation(
    fleet: &str,
    role: &str,
    authority_state: AdoptionAuthorityStateV1,
) -> AdoptionRecommendationV1 {
    AdoptionRecommendationV1 {
        kind: "review_authority_before_declaration".to_string(),
        severity: AdoptionRecommendationSeverityV1::Warning,
        description: format!(
            "review {fleet}.{role} authority before declaring observed role candidate ({})",
            adoption_authority_state_label(authority_state)
        ),
        suggested_action: None,
        suggested_action_effect: AdoptionSuggestedActionEffectV1::ReadOnly,
        suggested_action_support: AdoptionSuggestedActionSupportV1::SupportedByAdoption,
        suggested_action_availability: AdoptionSuggestedActionAvailabilityV1::AllowedIn0500,
        operator_action_requirement: AdoptionOperatorActionRequirementV1::Required,
    }
}

const fn adoption_authority_state_label(authority_state: AdoptionAuthorityStateV1) -> &'static str {
    match authority_state {
        AdoptionAuthorityStateV1::CanicAuthorized => "canic-authorized",
        AdoptionAuthorityStateV1::UserControlled => "user-controlled",
        AdoptionAuthorityStateV1::External => "external",
        AdoptionAuthorityStateV1::Unknown => "unknown",
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
        "artifact registry import",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[cfg(test)]
mod tests;
