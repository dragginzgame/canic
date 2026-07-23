use std::collections::{BTreeMap, BTreeSet};

use canic_core::ids::CanisterRole;

use crate::deployment_truth::DeploymentInventoryV1;

use super::{
    evidence::{
        artifact_state_from_observed, authority_state_for_control_class, combined_authority_state,
        observation_state, package_state,
    },
    model::{
        AdoptionArtifactStateV1, AdoptionAuthorityStateV1, AdoptionClassificationV1,
        AdoptionDeclarationStateV1, AdoptionMatchConfidenceV1, AdoptionObservedCanisterFindingV1,
        AdoptionPackageMetadataV1, AdoptionPackageStateV1, AdoptionProfileV1,
        AdoptionRoleFindingV1, AdoptionTopologyStateV1,
    },
    recommendations::{
        attach_later_recommendation, is_leaf_only_authority_sensitive_role,
        observed_only_recommendations, observed_only_warnings,
    },
};

pub(super) struct DeclaredRoleFindingInput<'a> {
    pub(super) profile: AdoptionProfileV1,
    pub(super) app: &'a str,
    pub(super) role: &'a CanisterRole,
    pub(super) package: &'a str,
    pub(super) attached: bool,
    pub(super) observed: Option<&'a [&'a crate::deployment_truth::ObservedCanisterV1]>,
    pub(super) duplicate_observation: bool,
    pub(super) packages_by_path: &'a BTreeMap<String, AdoptionPackageMetadataV1>,
    pub(super) artifact_state: Option<AdoptionArtifactStateV1>,
    pub(super) artifact_conflict: bool,
    pub(super) artifact_evidence: Option<&'a [String]>,
}

pub(super) struct ObservedOnlyRoleFindingInput<'a> {
    pub(super) profile: AdoptionProfileV1,
    pub(super) app: &'a str,
    pub(super) role: &'a str,
    pub(super) observed: &'a [&'a crate::deployment_truth::ObservedCanisterV1],
    pub(super) duplicate_observation: bool,
    pub(super) artifact_state: Option<AdoptionArtifactStateV1>,
    pub(super) artifact_conflict: bool,
    pub(super) artifact_evidence: Option<&'a [String]>,
}

pub(super) fn role_finding_for_declared_role(
    input: DeclaredRoleFindingInput<'_>,
) -> AdoptionRoleFindingV1 {
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
            recommendations.push(attach_later_recommendation(input.app, &role_name));
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

    let package_state = package_state(input.package, input.app, &role_name, input.packages_by_path);
    if matches!(
        package_state,
        AdoptionPackageStateV1::MissingApp
            | AdoptionPackageStateV1::MissingRole
            | AdoptionPackageStateV1::Mismatch
    ) {
        classifications.insert(AdoptionClassificationV1::EvidenceConflict);
        warnings.push("package metadata does not match declared app role".to_string());
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
        app: input.app.to_string(),
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

pub(super) fn role_finding_for_observed_only_role(
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
        app: input.app.to_string(),
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
            input.app,
            input.role,
            authority_state,
        ),
        warnings,
    }
}

pub(super) fn observed_canister_findings(
    profile: AdoptionProfileV1,
    app: &str,
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
            matched_app: role.map(|_| app.to_string()),
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
                    app,
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
            matched_app: pool.role.as_ref().map(|_| app.to_string()),
            matched_role: pool.role.clone(),
            confidence: AdoptionMatchConfidenceV1::Candidate,
            classifications: vec![AdoptionClassificationV1::ImportedPoolCandidate],
            controllers: Vec::new(),
            wasm_evidence: None,
            deployment_target_evidence: Some(format!("pool={}", pool.pool)),
            recommendations: Vec::new(),
            warnings: vec!["pool import is outside the current adoption workflow".to_string()],
        });
    }

    findings.sort_by(|left, right| left.canister_id.cmp(&right.canister_id));
    findings
}
