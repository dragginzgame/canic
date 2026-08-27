//! Module: release_set::application
//!
//! Responsibility: compile one Fleet-wide application artifact union and project root release sets.
//! Does not own: Cargo execution, infrastructure Wasms, Store publication, or Registry mutation.
//! Boundary: one qualified build is projected only through validated Component admissions.

mod persistence;
#[cfg(test)]
mod tests;

use crate::{
    component_topology::PlannedFleetSubnetRootTopology,
    release_set::{
        GZIP_MAGIC, WASM_MAGIC, valid_package_name, validate_release_artifact_relative_path,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

use canic_core::{
    bootstrap::compiled::{ComponentTopology, ComponentTopologyError},
    cdk::utils::hash::{decode_hex, sha256_hex},
    ids::{
        CanisterRole, ComponentSpecId, ComponentTopologyDigest, FleetSubnetRootBinding,
        ReleaseBuildId, ReleaseSetDigest,
    },
    role_contract::ProtocolProfileDigest,
};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

pub use persistence::{
    ApplicationArtifactFileBuildOutput, ApplicationArtifactUnionPersistenceError,
    PersistedApplicationArtifactUnion, compile_and_persist_application_artifact_union,
    load_persisted_application_artifact_union,
};

const MAX_ARTIFACT_PATH_BYTES: usize = 4_096;
const SHA_256_HEX_BYTES: usize = 64;

///
/// ApplicationArtifactBuildTarget
///
/// One topology role's package and output paths admitted before its build starts.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationArtifactBuildTarget {
    pub role: CanisterRole,
    pub package: String,
    pub wasm_relative_path: String,
    pub wasm_gz_relative_path: String,
}

///
/// ApplicationArtifactBuildOutput
///
/// Exact package, build identity, paths, and bytes returned for one topology role.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationArtifactBuildOutput {
    pub role: CanisterRole,
    pub package: String,
    pub release_build_id: ReleaseBuildId,
    pub wasm_relative_path: String,
    pub wasm: Vec<u8>,
    pub wasm_gz_relative_path: String,
    pub wasm_gz: Vec<u8>,
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

///
/// ApplicationArtifactEntry
///
/// Qualified immutable artifact evidence for one deduplicated application role.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationArtifactEntry {
    pub role: CanisterRole,
    pub package: String,
    pub release_build_id: ReleaseBuildId,
    pub wasm_relative_path: String,
    pub wasm_size_bytes: u64,
    pub wasm_sha256_hex: String,
    pub wasm_gz_relative_path: String,
    pub wasm_gz_size_bytes: u64,
    pub wasm_gz_sha256_hex: String,
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

///
/// ApplicationArtifactUnion
///
/// Canonical Fleet-wide Component and Component Child artifact set for one build.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationArtifactUnion {
    pub release_build_id: ReleaseBuildId,
    pub fleet_component_topology_digest: ComponentTopologyDigest,
    pub entries: Vec<ApplicationArtifactEntry>,
}

impl ApplicationArtifactUnion {
    /// Compile the exact deduplicated role union admitted by one Component Topology.
    pub fn compile(
        topology: &ComponentTopology,
        release_build_id: ReleaseBuildId,
        targets: &[ApplicationArtifactBuildTarget],
        outputs: &[ApplicationArtifactBuildOutput],
    ) -> Result<Self, ApplicationReleaseSetError> {
        Self::validate_build_targets(topology, targets)?;
        let targets = targets
            .iter()
            .map(|target| (&target.role, target))
            .collect::<BTreeMap<_, _>>();

        let mut output_roles = outputs
            .iter()
            .map(|output| output.role.clone())
            .collect::<Vec<_>>();
        Self::validate_build_output_roles(topology, &mut output_roles)?;

        let mut entries = outputs
            .iter()
            .map(|output| {
                let target = targets.get(&output.role).copied().ok_or_else(|| {
                    ApplicationReleaseSetError::MissingBuildTarget {
                        role: output.role.clone(),
                    }
                })?;
                compile_entry(release_build_id, target, output)
            })
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort_unstable_by(|left, right| left.role.cmp(&right.role));
        let union = Self {
            release_build_id,
            fleet_component_topology_digest: topology.digest()?,
            entries,
        };
        union.validate_against(topology)?;
        Ok(union)
    }

    fn validate_build_targets(
        topology: &ComponentTopology,
        targets: &[ApplicationArtifactBuildTarget],
    ) -> Result<(), ApplicationReleaseSetError> {
        let expected_roles = topology_roles(topology);
        let mut target_roles = targets
            .iter()
            .map(|target| target.role.clone())
            .collect::<Vec<_>>();
        target_roles.sort();
        if target_roles != expected_roles {
            return Err(ApplicationReleaseSetError::BuildTargetRoleSet {
                expected: expected_roles,
                actual: target_roles,
            });
        }
        Ok(())
    }

    fn validate_build_output_roles(
        topology: &ComponentTopology,
        actual: &mut Vec<CanisterRole>,
    ) -> Result<(), ApplicationReleaseSetError> {
        let expected = topology_roles(topology);
        actual.sort();
        if *actual != expected {
            return Err(ApplicationReleaseSetError::BuildOutputRoleSet {
                expected,
                actual: actual.clone(),
            });
        }
        Ok(())
    }

    /// Validate the canonical union shape and its exact Fleet-wide topology binding.
    pub fn validate_against(
        &self,
        topology: &ComponentTopology,
    ) -> Result<(), ApplicationReleaseSetError> {
        self.validate_retained_shape()?;
        let digest = topology.digest()?;
        if self.fleet_component_topology_digest != digest {
            return Err(ApplicationReleaseSetError::UnionTopologyDigestMismatch {
                expected: digest,
                actual: self.fleet_component_topology_digest,
            });
        }

        let expected = topology_roles(topology);
        let actual = self
            .entries
            .iter()
            .map(|entry| entry.role.clone())
            .collect::<Vec<_>>();
        if actual != expected {
            return Err(ApplicationReleaseSetError::ArtifactUnionRoleSet { expected, actual });
        }
        Ok(())
    }

    /// Encode the topology-validated union into deterministic compact JSON bytes.
    pub fn canonical_bytes(
        &self,
        topology: &ComponentTopology,
    ) -> Result<Vec<u8>, ApplicationReleaseSetError> {
        self.validate_against(topology)?;
        serde_json::to_vec(self).map_err(ApplicationReleaseSetError::Serialization)
    }

    /// Hash the exact canonical application artifact union bytes.
    pub fn digest(
        &self,
        topology: &ComponentTopology,
    ) -> Result<[u8; 32], ApplicationReleaseSetError> {
        Ok(Sha256::digest(self.canonical_bytes(topology)?).into())
    }

    /// Validate the canonical persisted shape without requiring caller-workspace topology.
    ///
    /// Recovery uses this only after a Fleet plan has bound the exact canonical union digest.
    /// Normal compilation and loading must continue to use [`Self::validate_against`].
    pub(crate) fn validate_retained_shape(&self) -> Result<(), ApplicationReleaseSetError> {
        if self.entries.is_empty() {
            return Err(ApplicationReleaseSetError::EmptyArtifactUnion);
        }

        let mut previous: Option<&CanisterRole> = None;
        let mut paths = BTreeSet::new();
        for entry in &self.entries {
            if let Some(previous) = previous
                && previous >= &entry.role
            {
                return Err(ApplicationReleaseSetError::NonCanonicalArtifactOrder {
                    previous: previous.clone(),
                    current: entry.role.clone(),
                });
            }
            previous = Some(&entry.role);
            validate_entry(self.release_build_id, entry)?;
            for path in [&entry.wasm_relative_path, &entry.wasm_gz_relative_path] {
                if !paths.insert(path.as_str()) {
                    return Err(ApplicationReleaseSetError::DuplicateArtifactPath {
                        path: path.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn get(&self, role: &CanisterRole) -> Option<&ApplicationArtifactEntry> {
        self.entries
            .binary_search_by(|entry| entry.role.cmp(role))
            .ok()
            .map(|index| &self.entries[index])
    }
}

///
/// ApplicationReleaseSetEntryKind
///
/// Structural use of one artifact within one exact Component Spec closure.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationReleaseSetEntryKind {
    Component,
    ComponentChild,
}

///
/// FleetSubnetRootReleaseSetEntry
///
/// One Spec-scoped Component or Component Child authorization and its qualified artifact.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootReleaseSetEntry {
    pub component_spec: ComponentSpecId,
    pub kind: ApplicationReleaseSetEntryKind,
    pub artifact: ApplicationArtifactEntry,
}

///
/// FleetSubnetRootReleaseSetManifest
///
/// Canonical root-local application artifact closure projected from immutable admissions.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSubnetRootReleaseSetManifest {
    pub release_build_id: ReleaseBuildId,
    pub component_topology_digest: ComponentTopologyDigest,
    pub entries: Vec<FleetSubnetRootReleaseSetEntry>,
}

impl FleetSubnetRootReleaseSetManifest {
    /// Project this host authority into the passive runtime decoding shape.
    #[must_use]
    pub fn root_store_manifest(&self) -> canic_core::dto::root_store::RootStoreReleaseSetManifest {
        use canic_core::dto::root_store::{
            RootStoreArtifact, RootStoreReleaseSetEntry, RootStoreReleaseSetEntryKind,
            RootStoreReleaseSetManifest,
        };

        RootStoreReleaseSetManifest {
            release_build_id: self.release_build_id,
            component_topology_digest: self.component_topology_digest,
            entries: self
                .entries
                .iter()
                .map(|entry| RootStoreReleaseSetEntry {
                    component_spec: entry.component_spec.clone(),
                    kind: match entry.kind {
                        ApplicationReleaseSetEntryKind::Component => {
                            RootStoreReleaseSetEntryKind::Component
                        }
                        ApplicationReleaseSetEntryKind::ComponentChild => {
                            RootStoreReleaseSetEntryKind::ComponentChild
                        }
                    },
                    artifact: RootStoreArtifact {
                        role: entry.artifact.role.clone(),
                        package: entry.artifact.package.clone(),
                        release_build_id: entry.artifact.release_build_id,
                        wasm_relative_path: entry.artifact.wasm_relative_path.clone(),
                        wasm_size_bytes: entry.artifact.wasm_size_bytes,
                        wasm_sha256_hex: entry.artifact.wasm_sha256_hex.clone(),
                        wasm_gz_relative_path: entry.artifact.wasm_gz_relative_path.clone(),
                        wasm_gz_size_bytes: entry.artifact.wasm_gz_size_bytes,
                        wasm_gz_sha256_hex: entry.artifact.wasm_gz_sha256_hex.clone(),
                        candid_sha256: entry.artifact.candid_sha256,
                        protocol_profile_digest: entry.artifact.protocol_profile_digest,
                    },
                })
                .collect(),
        }
    }

    /// Project one root's exact Spec-scoped artifact closure from the Fleet-wide union.
    pub fn project(
        topology: &ComponentTopology,
        binding: &FleetSubnetRootBinding,
        union: &ApplicationArtifactUnion,
    ) -> Result<Self, ApplicationReleaseSetError> {
        Self::project_for_root(
            topology,
            &binding.component_admissions,
            binding.component_topology_digest,
            binding.limits.maximum_wasm_store_bytes,
            union,
        )
    }

    /// Project one pre-creation root plan without fabricating a root principal.
    pub fn project_planned(
        topology: &ComponentTopology,
        root: &PlannedFleetSubnetRootTopology,
        union: &ApplicationArtifactUnion,
    ) -> Result<Self, ApplicationReleaseSetError> {
        Self::project_for_root(
            topology,
            &root.component_admissions,
            root.component_topology_digest,
            root.limits.maximum_wasm_store_bytes,
            union,
        )
    }

    fn project_for_root(
        topology: &ComponentTopology,
        component_admissions: &[canic_core::ids::ComponentSpecAdmission],
        component_topology_digest: ComponentTopologyDigest,
        maximum_wasm_store_bytes: u64,
        union: &ApplicationArtifactUnion,
    ) -> Result<Self, ApplicationReleaseSetError> {
        union.validate_against(topology)?;
        let projected = topology.project_for_admissions(component_admissions)?;
        let digest = projected.digest()?;
        if digest != component_topology_digest {
            return Err(ApplicationReleaseSetError::RootTopologyDigestMismatch {
                expected: digest,
                actual: component_topology_digest,
            });
        }

        let mut entries = Vec::new();
        for component_spec in &projected.component_specs {
            entries.push(project_entry(
                &component_spec.component_spec,
                ApplicationReleaseSetEntryKind::Component,
                &component_spec.component_role,
                union,
            )?);
            for child in &component_spec.children {
                entries.push(project_entry(
                    &component_spec.component_spec,
                    ApplicationReleaseSetEntryKind::ComponentChild,
                    &child.role,
                    union,
                )?);
            }
        }

        let manifest = Self {
            release_build_id: union.release_build_id,
            component_topology_digest: digest,
            entries,
        };
        manifest.validate_for_root(
            topology,
            component_admissions,
            component_topology_digest,
            maximum_wasm_store_bytes,
            union,
        )?;
        Ok(manifest)
    }

    /// Validate this manifest against its exact topology, root binding, and artifact union.
    pub fn validate_against(
        &self,
        topology: &ComponentTopology,
        binding: &FleetSubnetRootBinding,
        union: &ApplicationArtifactUnion,
    ) -> Result<(), ApplicationReleaseSetError> {
        self.validate_for_root(
            topology,
            &binding.component_admissions,
            binding.component_topology_digest,
            binding.limits.maximum_wasm_store_bytes,
            union,
        )
    }

    /// Validate this manifest against one exact pre-creation root plan.
    pub fn validate_against_planned(
        &self,
        topology: &ComponentTopology,
        root: &PlannedFleetSubnetRootTopology,
        union: &ApplicationArtifactUnion,
    ) -> Result<(), ApplicationReleaseSetError> {
        self.validate_for_root(
            topology,
            &root.component_admissions,
            root.component_topology_digest,
            root.limits.maximum_wasm_store_bytes,
            union,
        )
    }

    fn validate_for_root(
        &self,
        topology: &ComponentTopology,
        component_admissions: &[canic_core::ids::ComponentSpecAdmission],
        component_topology_digest: ComponentTopologyDigest,
        maximum_wasm_store_bytes: u64,
        union: &ApplicationArtifactUnion,
    ) -> Result<(), ApplicationReleaseSetError> {
        union.validate_against(topology)?;
        if self.release_build_id != union.release_build_id {
            return Err(ApplicationReleaseSetError::ManifestBuildMismatch {
                expected: union.release_build_id,
                actual: self.release_build_id,
            });
        }

        let projected = topology.project_for_admissions(component_admissions)?;
        let digest = projected.digest()?;
        if component_topology_digest != digest {
            return Err(ApplicationReleaseSetError::RootTopologyDigestMismatch {
                expected: digest,
                actual: component_topology_digest,
            });
        }
        if self.component_topology_digest != digest {
            return Err(ApplicationReleaseSetError::ManifestTopologyDigestMismatch {
                expected: digest,
                actual: self.component_topology_digest,
            });
        }

        let expected = expected_projection_entries(&projected, union)?;
        if self.entries != expected {
            return Err(ApplicationReleaseSetError::ProjectionMismatch);
        }

        let mut unique_artifacts = BTreeSet::new();
        let total_bytes = self.entries.iter().try_fold(0_u64, |total, entry| {
            let artifact = &entry.artifact;
            if unique_artifacts.insert((
                artifact.wasm_size_bytes,
                artifact.wasm_sha256_hex.as_str(),
                artifact.wasm_gz_size_bytes,
                artifact.wasm_gz_sha256_hex.as_str(),
            )) {
                total
                    .checked_add(artifact.wasm_gz_size_bytes)
                    .ok_or(ApplicationReleaseSetError::StoreBytesOverflow)
            } else {
                Ok(total)
            }
        })?;
        if total_bytes > maximum_wasm_store_bytes {
            return Err(ApplicationReleaseSetError::WasmStoreLimitExceeded {
                maximum_bytes: maximum_wasm_store_bytes,
                required_bytes: total_bytes,
            });
        }

        Ok(())
    }

    /// Encode the fully validated root manifest into deterministic compact JSON bytes.
    pub fn canonical_bytes(
        &self,
        topology: &ComponentTopology,
        binding: &FleetSubnetRootBinding,
        union: &ApplicationArtifactUnion,
    ) -> Result<Vec<u8>, ApplicationReleaseSetError> {
        self.validate_against(topology, binding, union)?;
        serde_json::to_vec(self).map_err(ApplicationReleaseSetError::Serialization)
    }

    /// Hash the exact canonical root release-set manifest bytes.
    pub fn digest(
        &self,
        topology: &ComponentTopology,
        binding: &FleetSubnetRootBinding,
        union: &ApplicationArtifactUnion,
    ) -> Result<ReleaseSetDigest, ApplicationReleaseSetError> {
        Ok(ReleaseSetDigest::from_bytes(
            Sha256::digest(self.canonical_bytes(topology, binding, union)?).into(),
        ))
    }

    /// Encode one pre-creation root plan's fully validated manifest.
    pub fn canonical_bytes_planned(
        &self,
        topology: &ComponentTopology,
        root: &PlannedFleetSubnetRootTopology,
        union: &ApplicationArtifactUnion,
    ) -> Result<Vec<u8>, ApplicationReleaseSetError> {
        self.validate_against_planned(topology, root, union)?;
        serde_json::to_vec(self).map_err(ApplicationReleaseSetError::Serialization)
    }

    /// Hash one pre-creation root plan's exact canonical manifest bytes.
    pub fn digest_planned(
        &self,
        topology: &ComponentTopology,
        root: &PlannedFleetSubnetRootTopology,
        union: &ApplicationArtifactUnion,
    ) -> Result<ReleaseSetDigest, ApplicationReleaseSetError> {
        Ok(ReleaseSetDigest::from_bytes(
            Sha256::digest(self.canonical_bytes_planned(topology, root, union)?).into(),
        ))
    }
}

///
/// ApplicationReleaseSetError
///
/// Typed rejection at the application artifact-union and root-projection boundary.
///

#[derive(Debug, ThisError)]
pub enum ApplicationReleaseSetError {
    #[error("application artifact {role} {kind} size cannot be represented")]
    ArtifactSizeOverflow {
        role: CanisterRole,
        kind: &'static str,
    },

    #[error("application artifact union role set differs from Component Topology")]
    ArtifactUnionRoleSet {
        expected: Vec<CanisterRole>,
        actual: Vec<CanisterRole>,
    },

    #[error("application build output {role} package {actual} does not match target {expected}")]
    BuildOutputPackageMismatch {
        role: CanisterRole,
        expected: String,
        actual: String,
    },

    #[error(
        "application build output {role} {kind} path {actual} does not match target {expected}"
    )]
    BuildOutputPathMismatch {
        role: CanisterRole,
        kind: &'static str,
        expected: String,
        actual: String,
    },

    #[error("application build output role set differs from Component Topology")]
    BuildOutputRoleSet {
        expected: Vec<CanisterRole>,
        actual: Vec<CanisterRole>,
    },

    #[error("application build target role set differs from Component Topology")]
    BuildTargetRoleSet {
        expected: Vec<CanisterRole>,
        actual: Vec<CanisterRole>,
    },

    #[error("application artifact path is duplicated: {path}")]
    DuplicateArtifactPath { path: String },

    #[error("application artifact union must not be empty")]
    EmptyArtifactUnion,

    #[error("application artifact {role} has an empty {kind} payload")]
    EmptyArtifact {
        role: CanisterRole,
        kind: &'static str,
    },

    #[error("application artifact {role} has invalid gzip Wasm: {source}")]
    InvalidGzip {
        role: CanisterRole,
        #[source]
        source: std::io::Error,
    },

    #[error("application artifact {role} has an invalid package identity: {package}")]
    InvalidPackage { role: CanisterRole, package: String },

    #[error("application artifact {role} has an invalid {kind} path: {path}")]
    InvalidPath {
        role: CanisterRole,
        kind: &'static str,
        path: String,
    },

    #[error("application artifact {role} has an invalid {kind} SHA-256: {value}")]
    InvalidSha256 {
        role: CanisterRole,
        kind: &'static str,
        value: String,
    },

    #[error("application artifact {role} does not begin with the Wasm magic bytes")]
    InvalidWasm { role: CanisterRole },

    #[error(
        "application release-set manifest build {actual} does not match artifact union build {expected}"
    )]
    ManifestBuildMismatch {
        expected: ReleaseBuildId,
        actual: ReleaseBuildId,
    },

    #[error("application release-set manifest topology digest differs from its root projection")]
    ManifestTopologyDigestMismatch {
        expected: ComponentTopologyDigest,
        actual: ComponentTopologyDigest,
    },

    #[error("application artifact union has no build target for role {role}")]
    MissingBuildTarget { role: CanisterRole },

    #[error("application artifact union has no qualified artifact for role {role}")]
    MissingArtifact { role: CanisterRole },

    #[error("application artifact order is not canonical: {previous} before {current}")]
    NonCanonicalArtifactOrder {
        previous: CanisterRole,
        current: CanisterRole,
    },

    #[error("application release-set manifest differs from the exact admitted projection")]
    ProjectionMismatch,

    #[error(
        "application artifact {role} release build {actual} does not match union build {expected}"
    )]
    ReleaseBuildMismatch {
        role: CanisterRole,
        expected: ReleaseBuildId,
        actual: ReleaseBuildId,
    },

    #[error("application artifact {role} raw and gzip Wasm representations differ")]
    RepresentationMismatch { role: CanisterRole },

    #[error("Fleet Subnet Root topology digest differs from its admitted projection")]
    RootTopologyDigestMismatch {
        expected: ComponentTopologyDigest,
        actual: ComponentTopologyDigest,
    },

    #[error("failed to serialize application release-set evidence: {0}")]
    Serialization(serde_json::Error),

    #[error("root release-set artifact byte total overflowed")]
    StoreBytesOverflow,

    #[error(transparent)]
    Topology(#[from] ComponentTopologyError),

    #[error("application artifact union topology digest differs from Component Topology")]
    UnionTopologyDigestMismatch {
        expected: ComponentTopologyDigest,
        actual: ComponentTopologyDigest,
    },

    #[error(
        "root release set requires {required_bytes} Wasm Store bytes, exceeding limit {maximum_bytes}"
    )]
    WasmStoreLimitExceeded {
        maximum_bytes: u64,
        required_bytes: u64,
    },

    #[error("application artifact {role} {kind} size must be nonzero")]
    ZeroSize {
        role: CanisterRole,
        kind: &'static str,
    },
}

fn topology_roles(topology: &ComponentTopology) -> Vec<CanisterRole> {
    topology
        .component_specs
        .iter()
        .flat_map(|spec| {
            std::iter::once(spec.component_role.clone())
                .chain(spec.children.iter().map(|child| child.role.clone()))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn compile_entry(
    release_build_id: ReleaseBuildId,
    target: &ApplicationArtifactBuildTarget,
    output: &ApplicationArtifactBuildOutput,
) -> Result<ApplicationArtifactEntry, ApplicationReleaseSetError> {
    if output.package != target.package {
        return Err(ApplicationReleaseSetError::BuildOutputPackageMismatch {
            role: output.role.clone(),
            expected: target.package.clone(),
            actual: output.package.clone(),
        });
    }
    if output.wasm_relative_path != target.wasm_relative_path {
        return Err(ApplicationReleaseSetError::BuildOutputPathMismatch {
            role: output.role.clone(),
            kind: "raw Wasm",
            expected: target.wasm_relative_path.clone(),
            actual: output.wasm_relative_path.clone(),
        });
    }
    if output.wasm_gz_relative_path != target.wasm_gz_relative_path {
        return Err(ApplicationReleaseSetError::BuildOutputPathMismatch {
            role: output.role.clone(),
            kind: "gzip Wasm",
            expected: target.wasm_gz_relative_path.clone(),
            actual: output.wasm_gz_relative_path.clone(),
        });
    }
    if output.release_build_id != release_build_id {
        return Err(ApplicationReleaseSetError::ReleaseBuildMismatch {
            role: output.role.clone(),
            expected: release_build_id,
            actual: output.release_build_id,
        });
    }
    if output.wasm.is_empty() {
        return Err(ApplicationReleaseSetError::EmptyArtifact {
            role: output.role.clone(),
            kind: "raw Wasm",
        });
    }
    if !output.wasm.starts_with(&WASM_MAGIC) {
        return Err(ApplicationReleaseSetError::InvalidWasm {
            role: output.role.clone(),
        });
    }
    if output.wasm_gz.is_empty() {
        return Err(ApplicationReleaseSetError::EmptyArtifact {
            role: output.role.clone(),
            kind: "gzip Wasm",
        });
    }
    if !output.wasm_gz.starts_with(&GZIP_MAGIC) {
        return Err(ApplicationReleaseSetError::InvalidGzip {
            role: output.role.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "missing gzip header"),
        });
    }

    let wasm_size_bytes = u64::try_from(output.wasm.len()).map_err(|_| {
        ApplicationReleaseSetError::ArtifactSizeOverflow {
            role: output.role.clone(),
            kind: "raw Wasm",
        }
    })?;
    let wasm_gz_size_bytes = u64::try_from(output.wasm_gz.len()).map_err(|_| {
        ApplicationReleaseSetError::ArtifactSizeOverflow {
            role: output.role.clone(),
            kind: "gzip Wasm",
        }
    })?;
    let mut decoded = Vec::new();
    GzDecoder::new(output.wasm_gz.as_slice())
        .take(wasm_size_bytes.saturating_add(1))
        .read_to_end(&mut decoded)
        .map_err(|source| ApplicationReleaseSetError::InvalidGzip {
            role: output.role.clone(),
            source,
        })?;
    if decoded != output.wasm {
        return Err(ApplicationReleaseSetError::RepresentationMismatch {
            role: output.role.clone(),
        });
    }

    let entry = ApplicationArtifactEntry {
        role: output.role.clone(),
        package: output.package.clone(),
        release_build_id: output.release_build_id,
        wasm_relative_path: output.wasm_relative_path.clone(),
        wasm_size_bytes,
        wasm_sha256_hex: sha256_hex(&output.wasm),
        wasm_gz_relative_path: output.wasm_gz_relative_path.clone(),
        wasm_gz_size_bytes,
        wasm_gz_sha256_hex: sha256_hex(&output.wasm_gz),
        candid_sha256: output.candid_sha256,
        protocol_profile_digest: output.protocol_profile_digest,
    };
    validate_entry(release_build_id, &entry)?;
    Ok(entry)
}

fn validate_entry(
    release_build_id: ReleaseBuildId,
    entry: &ApplicationArtifactEntry,
) -> Result<(), ApplicationReleaseSetError> {
    if entry.release_build_id != release_build_id {
        return Err(ApplicationReleaseSetError::ReleaseBuildMismatch {
            role: entry.role.clone(),
            expected: release_build_id,
            actual: entry.release_build_id,
        });
    }
    if !valid_package_name(&entry.package) {
        return Err(ApplicationReleaseSetError::InvalidPackage {
            role: entry.role.clone(),
            package: entry.package.clone(),
        });
    }
    validate_path(&entry.role, "raw Wasm", &entry.wasm_relative_path)?;
    validate_path(&entry.role, "gzip Wasm", &entry.wasm_gz_relative_path)?;
    if entry.wasm_size_bytes == 0 {
        return Err(ApplicationReleaseSetError::ZeroSize {
            role: entry.role.clone(),
            kind: "raw Wasm",
        });
    }
    if entry.wasm_gz_size_bytes == 0 {
        return Err(ApplicationReleaseSetError::ZeroSize {
            role: entry.role.clone(),
            kind: "gzip Wasm",
        });
    }
    validate_sha256(&entry.role, "raw Wasm", &entry.wasm_sha256_hex)?;
    validate_sha256(&entry.role, "gzip Wasm", &entry.wasm_gz_sha256_hex)
}

fn validate_path(
    role: &CanisterRole,
    kind: &'static str,
    path: &str,
) -> Result<(), ApplicationReleaseSetError> {
    if path.len() > MAX_ARTIFACT_PATH_BYTES
        || validate_release_artifact_relative_path(path).is_err()
    {
        return Err(ApplicationReleaseSetError::InvalidPath {
            role: role.clone(),
            kind,
            path: path.to_string(),
        });
    }
    Ok(())
}

fn validate_sha256(
    role: &CanisterRole,
    kind: &'static str,
    value: &str,
) -> Result<(), ApplicationReleaseSetError> {
    let canonical = value.len() == SHA_256_HEX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && decode_hex(value).is_ok();
    if canonical {
        Ok(())
    } else {
        Err(ApplicationReleaseSetError::InvalidSha256 {
            role: role.clone(),
            kind,
            value: value.to_string(),
        })
    }
}

fn project_entry(
    component_spec: &ComponentSpecId,
    kind: ApplicationReleaseSetEntryKind,
    role: &CanisterRole,
    union: &ApplicationArtifactUnion,
) -> Result<FleetSubnetRootReleaseSetEntry, ApplicationReleaseSetError> {
    let artifact = union
        .get(role)
        .ok_or_else(|| ApplicationReleaseSetError::MissingArtifact { role: role.clone() })?;
    Ok(FleetSubnetRootReleaseSetEntry {
        component_spec: component_spec.clone(),
        kind,
        artifact: artifact.clone(),
    })
}

fn expected_projection_entries(
    topology: &ComponentTopology,
    union: &ApplicationArtifactUnion,
) -> Result<Vec<FleetSubnetRootReleaseSetEntry>, ApplicationReleaseSetError> {
    let mut entries = Vec::new();
    for component_spec in &topology.component_specs {
        entries.push(project_entry(
            &component_spec.component_spec,
            ApplicationReleaseSetEntryKind::Component,
            &component_spec.component_role,
            union,
        )?);
        for child in &component_spec.children {
            entries.push(project_entry(
                &component_spec.component_spec,
                ApplicationReleaseSetEntryKind::ComponentChild,
                &child.role,
                union,
            )?);
        }
    }
    Ok(entries)
}
