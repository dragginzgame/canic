use std::collections::{BTreeMap, BTreeSet};

use crate::deployment_truth::{
    ArtifactSourceV1, CanisterControlClassV1, DeploymentInventoryV1, RoleArtifactManifestV1,
};

use super::model::{
    AdoptionArtifactStateV1, AdoptionAuthorityStateV1, AdoptionObservationStateV1,
    AdoptionPackageMetadataV1, AdoptionPackageStateV1,
};

pub(super) fn package_state(
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

pub(super) fn observed_canisters_by_role(
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

pub(super) fn duplicate_observed_roles(
    observed_by_role: &BTreeMap<String, Vec<&crate::deployment_truth::ObservedCanisterV1>>,
) -> BTreeSet<String> {
    observed_by_role
        .iter()
        .filter(|(_, canisters)| canisters.len() > 1)
        .map(|(role, _)| role.clone())
        .collect()
}

pub(super) fn package_metadata_by_path(
    metadata: Vec<AdoptionPackageMetadataV1>,
) -> BTreeMap<String, AdoptionPackageMetadataV1> {
    metadata
        .into_iter()
        .map(|metadata| (metadata.package.clone(), metadata))
        .collect()
}

pub(super) fn artifact_states_by_role(
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

pub(super) fn artifact_conflict_roles(
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

pub(super) fn artifact_evidence_by_role(
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

pub(super) fn combined_authority_state(
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

pub(super) const fn authority_state_for_control_class(
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

pub(super) const fn observation_state(
    observed: bool,
    conflict: bool,
) -> AdoptionObservationStateV1 {
    match (observed, conflict) {
        (_, true) => AdoptionObservationStateV1::ConflictingMatch,
        (true, false) => AdoptionObservationStateV1::Observed,
        (false, false) => AdoptionObservationStateV1::Unobserved,
    }
}

pub(super) fn artifact_state_from_observed(
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

pub(super) fn missing_evidence(
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
