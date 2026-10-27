//! Complete native Component partition evidence; storage and authority hashing remain unchanged.

/// The complete current partition, including its original provisioning authority.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Partition {
    pub head: crate::dto::component_registry::ComponentRegistryHead,
    pub binding: crate::dto::component_directory::ComponentBinding,
    pub protocol_profile_digest: crate::protocol::profile::ProtocolProfileDigest,
    pub provisioning_origin: ProvisioningOrigin,
    pub release_set: crate::ids::FleetSubnetRootReleaseSet,
    pub status: crate::dto::component_registry::ComponentLifecycleStatus,
    pub reserved_descendants: u32,
    pub committed_descendants: u32,
    pub encoded_bytes: u64,
}

/// Immutable preparation/activation evidence alongside the current partition.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ActivePartition {
    pub allocation_operation_id: [u8; 32],
    pub prepared: Partition,
    pub activation: Partition,
    pub current: Partition,
}

/// Exact remote service member authority retained by a provisioning operation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ServiceRequester {
    pub service: crate::ids::FleetServiceId,
    pub member_purpose: crate::deployment::FleetServiceMemberPurpose,
    pub group_placement: crate::ids::ComponentGroupPlacementId,
    pub member_path: crate::ids::ComponentGroupMemberPath,
    pub component: crate::dto::component_directory::ComponentBinding,
}

/// Authenticated causal authority, preserving every supported provisioning origin.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum ProvisioningOrigin {
    FleetAdministrator {
        caller: ic_platform::Principal,
    },
    Component {
        requester: Box<crate::dto::component_directory::ComponentBinding>,
        grant: Box<crate::deployment::ComponentProvisioningGrant>,
    },
    FleetServiceComponent {
        requester: Box<ServiceRequester>,
        registry: Box<crate::dto::registry_discovery::RegistryVersion>,
        grant: Box<crate::deployment::ComponentProvisioningGrant>,
    },
    ComponentGroup {
        operation_id: [u8; 32],
        plan_hash: [u8; 32],
        group_placement: crate::ids::ComponentGroupPlacementId,
        member_path: crate::ids::ComponentGroupMemberPath,
    },
}
