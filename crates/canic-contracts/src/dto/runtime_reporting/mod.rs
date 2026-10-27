//! Native retained runtime/activation evidence; no new lifecycle or storage ownership.
use crate::{
    dto::{
        component_registry::{ComponentRuntimeActivationEvidence, ComponentRuntimePhase},
        fixture_provisioning::FixtureDescriptor,
        fleet_activation::{
            FleetActivationIdentity, FleetActivationPhase, FleetCascadeActivationEvidence,
            FleetCredentialGenerationRef,
        },
    },
    ids::{FleetKey, ReleaseBuildId},
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Complete native ComponentRuntimeStatusResponse projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentRuntimeStatusResponse {
    pub fixture: Option<Box<FixtureAssignment>>,
    pub operation_id: [u8; 32],
    pub binding: crate::dto::component_directory::ManagedCanisterBinding,
    pub deployment: Box<ProtectedComponentDeployment>,
    pub phase: ComponentRuntimePhase,
    pub authority: Option<crate::dto::runtime_configuration::ComponentRuntimeDirectoryAuthority>,
    pub authority_hash: Option<[u8; 32]>,
    pub direct_children_hash: Option<[u8; 32]>,
    pub activation: Option<ComponentRuntimeActivationEvidence>,
}

/// Complete native ComponentRuntimeOperationStatus projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentRuntimeOperationStatus {
    pub operation_id: [u8; 32],
    pub fleet_activation: FleetActivationStatusResponse,
    pub runtime: ComponentRuntimeStatusResponse,
}

/// Complete native FleetCascadeManifestEntry projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetCascadeManifestEntry {
    pub principal: Principal,
    pub state_snapshot_hash: [u8; 32],
    pub topology_snapshot_hash: [u8; 32],
}

/// Complete native FleetCredentialManifestEntry projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetCredentialManifestEntry {
    pub root_issuer: Principal,
    pub subject_canister: Principal,
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub key_identity_hash: [u8; 32],
    pub cert_hash: [u8; 32],
    pub proof_hash: [u8; 32],
    pub bundle_hash: [u8; 32],
}

/// Complete native FleetCredentialManifest projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetCredentialManifest {
    pub fleet: FleetKey,
    pub activation_id: [u8; 32],
    pub generation: u64,
    pub root_policy_set_hash: [u8; 32],
    pub renewal_template_set_hash: [u8; 32],
    pub entries: Vec<FleetCredentialManifestEntry>,
}

/// Complete native FleetActivationStatusResponse projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetActivationStatusResponse {
    pub phase: FleetActivationPhase,
    pub identity: FleetActivationIdentity,
    pub cascade: Option<FleetCascadeActivationEvidence>,
    pub cascade_manifest: Option<Vec<FleetCascadeManifestEntry>>,
    pub credential: Option<FleetCredentialGenerationRef>,
    pub credential_manifest: Option<FleetCredentialManifest>,
    pub activated_at_ns: Option<u64>,
}

/// Complete native FixtureTargetBinding projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FixtureTargetBinding {
    pub target: crate::dto::component_directory::ManagedCanisterBinding,
    pub installation: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub content_id: [u8; 32],
}

/// Complete native FixtureGrant projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FixtureGrant {
    pub revision: u64,
    pub binding: FixtureTargetBinding,
    pub enabled: bool,
}

/// Complete native FixtureAssignment projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FixtureAssignment {
    pub store: Principal,
    pub grant: FixtureGrant,
    pub descriptor: FixtureDescriptor,
}

/// Immutable deployment context under which the runtime was installed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
#[cfg_attr(
    target_pointer_width = "64",
    expect(
        clippy::large_enum_variant,
        reason = "complete protected deployment preserves its explicit variant shape"
    )
)]
pub enum ProtectedComponentDeployment {
    UngroupedOrdinary {
        binding: crate::dto::component_directory::ComponentBinding,
    },
    GroupMember {
        binding: crate::dto::component_directory::ComponentBinding,
        configuration_digest: crate::ids::ComponentDeploymentConfigurationDigest,
        group_placement: crate::ids::ComponentGroupPlacementId,
        component_group: crate::ids::ComponentGroupSpecId,
        member_path: crate::ids::ComponentGroupMemberPath,
        purpose: crate::deployment::ComponentDeploymentPurpose,
        labels: Vec<crate::deployment::ComponentDeploymentLabel>,
        limits: crate::deployment::ComponentDeploymentLimits,
    },
}
