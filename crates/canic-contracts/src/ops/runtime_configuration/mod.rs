//! Lossless managed runtime transport conversions; no authority or lifecycle policy.
use crate::dto::runtime_configuration as native;

impl From<crate::dto::component_registry::ComponentRuntimeDirectoryPreparationRequest>
    for native::ComponentRuntimeDirectoryPreparationRequest
{
    fn from(
        value: crate::dto::component_registry::ComponentRuntimeDirectoryPreparationRequest,
    ) -> Self {
        Self {
            operation_id: value.operation_id,
            authority: value.authority.into(),
            direct_children: value.direct_children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::ComponentRuntimeDirectoryPreparationRequest>
    for crate::dto::component_registry::ComponentRuntimeDirectoryPreparationRequest
{
    fn from(value: native::ComponentRuntimeDirectoryPreparationRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            authority: value.authority.into(),
            direct_children: value.direct_children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::dto::component_registry::ComponentRuntimeDirectoryAuthority>
    for native::ComponentRuntimeDirectoryAuthority
{
    fn from(value: crate::dto::component_registry::ComponentRuntimeDirectoryAuthority) -> Self {
        Self {
            fleet: value.fleet.into(),
            component: value.component.into(),
            component_group: value.component_group.map(Into::into),
        }
    }
}

impl From<native::ComponentRuntimeDirectoryAuthority>
    for crate::dto::component_registry::ComponentRuntimeDirectoryAuthority
{
    fn from(value: native::ComponentRuntimeDirectoryAuthority) -> Self {
        Self {
            fleet: value.fleet.into(),
            component: value.component.into(),
            component_group: value.component_group.map(Into::into),
        }
    }
}

impl From<crate::dto::component_registry::ComponentRuntimeDirectChild>
    for native::ComponentRuntimeDirectChild
{
    fn from(value: crate::dto::component_registry::ComponentRuntimeDirectChild) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            allocation_operation_id: value.allocation_operation_id,
            role: value.role,
            protocol_profile_digest: value.protocol_profile_digest,
        }
    }
}

impl From<native::ComponentRuntimeDirectChild>
    for crate::dto::component_registry::ComponentRuntimeDirectChild
{
    fn from(value: native::ComponentRuntimeDirectChild) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            allocation_operation_id: value.allocation_operation_id,
            role: value.role,
            protocol_profile_digest: value.protocol_profile_digest,
        }
    }
}

impl From<crate::dto::fleet_registry::FleetDirectorySnapshot> for native::FleetDirectorySnapshot {
    fn from(value: crate::dto::fleet_registry::FleetDirectorySnapshot) -> Self {
        Self {
            provenance: value.provenance.into(),
            fleet_subnet_roots: value
                .fleet_subnet_roots
                .into_iter()
                .map(Into::into)
                .collect(),
            services: value.services.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::FleetDirectorySnapshot> for crate::dto::fleet_registry::FleetDirectorySnapshot {
    fn from(value: native::FleetDirectorySnapshot) -> Self {
        Self {
            provenance: value.provenance.into(),
            fleet_subnet_roots: value
                .fleet_subnet_roots
                .into_iter()
                .map(Into::into)
                .collect(),
            services: value.services.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::dto::fleet_registry::FleetDirectoryProvenance>
    for native::FleetDirectoryProvenance
{
    fn from(value: crate::dto::fleet_registry::FleetDirectoryProvenance) -> Self {
        Self {
            registry: value.registry.into(),
            source_fleet_subnet_root: value.source_fleet_subnet_root.into(),
        }
    }
}

impl From<native::FleetDirectoryProvenance>
    for crate::dto::fleet_registry::FleetDirectoryProvenance
{
    fn from(value: native::FleetDirectoryProvenance) -> Self {
        Self {
            registry: value.registry.into(),
            source_fleet_subnet_root: value.source_fleet_subnet_root.into(),
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootDirectoryEntry>
    for native::FleetSubnetRootDirectoryEntry
{
    fn from(value: crate::dto::fleet_registry::FleetSubnetRootDirectoryEntry) -> Self {
        Self {
            placement_subnet: value.placement_subnet.into_principal().into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            status: value.status,
        }
    }
}

impl From<native::FleetSubnetRootDirectoryEntry>
    for crate::dto::fleet_registry::FleetSubnetRootDirectoryEntry
{
    fn from(value: native::FleetSubnetRootDirectoryEntry) -> Self {
        Self {
            placement_subnet: crate::ids::SubnetId::from_principal(value.placement_subnet.into()),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            status: value.status,
        }
    }
}

impl From<crate::dto::fleet_registry::FleetDirectoryService> for native::FleetDirectoryService {
    fn from(value: crate::dto::fleet_registry::FleetDirectoryService) -> Self {
        Self {
            service: value.service,
            role: value.role,
            component_spec: value.component_spec,
            mode: value.mode,
            placement: value.placement,
            members: value.members.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::FleetDirectoryService> for crate::dto::fleet_registry::FleetDirectoryService {
    fn from(value: native::FleetDirectoryService) -> Self {
        Self {
            service: value.service,
            role: value.role,
            component_spec: value.component_spec,
            mode: value.mode,
            placement: value.placement,
            members: value.members.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::dto::fleet_registry::FleetDirectoryServiceComponent>
    for native::FleetDirectoryServiceComponent
{
    fn from(value: crate::dto::fleet_registry::FleetDirectoryServiceComponent) -> Self {
        Self {
            member_purpose: value.member_purpose,
            component: value.component,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            canister_id: value.canister_id.into(),
            group_placement: value.group_placement,
            member_path: value.member_path,
        }
    }
}

impl From<native::FleetDirectoryServiceComponent>
    for crate::dto::fleet_registry::FleetDirectoryServiceComponent
{
    fn from(value: native::FleetDirectoryServiceComponent) -> Self {
        Self {
            member_purpose: value.member_purpose,
            component: value.component,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            canister_id: value.canister_id.into(),
            group_placement: value.group_placement,
            member_path: value.member_path,
        }
    }
}

impl From<crate::dto::component_provisioning::ComponentGroupDirectory>
    for native::ComponentGroupDirectory
{
    fn from(value: crate::dto::component_provisioning::ComponentGroupDirectory) -> Self {
        Self {
            provenance: value.provenance.into(),
            members: value.members.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::ComponentGroupDirectory>
    for crate::dto::component_provisioning::ComponentGroupDirectory
{
    fn from(value: native::ComponentGroupDirectory) -> Self {
        Self {
            provenance: value.provenance.into(),
            members: value.members.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::dto::component_provisioning::ComponentGroupDirectoryProvenance>
    for native::ComponentGroupDirectoryProvenance
{
    fn from(value: crate::dto::component_provisioning::ComponentGroupDirectoryProvenance) -> Self {
        Self {
            authority: value.authority.into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            group_placement: value.group_placement,
            component_group: value.component_group,
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            placement_receipt_content_hash: value.placement_receipt_content_hash,
        }
    }
}

impl From<native::ComponentGroupDirectoryProvenance>
    for crate::dto::component_provisioning::ComponentGroupDirectoryProvenance
{
    fn from(value: native::ComponentGroupDirectoryProvenance) -> Self {
        Self {
            authority: value.authority.into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            group_placement: value.group_placement,
            component_group: value.component_group,
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            placement_receipt_content_hash: value.placement_receipt_content_hash,
        }
    }
}

impl From<crate::dto::component_provisioning::ComponentGroupDirectoryMember>
    for native::ComponentGroupDirectoryMember
{
    fn from(value: crate::dto::component_provisioning::ComponentGroupDirectoryMember) -> Self {
        Self {
            member_path: value.member_path,
            component_spec: value.component_spec,
            purpose: value.purpose,
            labels: value.labels,
            binding: value.binding.into(),
        }
    }
}

impl From<native::ComponentGroupDirectoryMember>
    for crate::dto::component_provisioning::ComponentGroupDirectoryMember
{
    fn from(value: native::ComponentGroupDirectoryMember) -> Self {
        Self {
            member_path: value.member_path,
            component_spec: value.component_spec,
            purpose: value.purpose,
            labels: value.labels,
            binding: value.binding.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dto::fleet_registry, ids::SubnetId};
    use ic_platform::principal::ExternalPrincipal;

    #[test]
    fn root_directory_wire_round_trip_preserves_distinct_subnet_and_canister() {
        let subnet = ExternalPrincipal::from_slice(&[1; 29]);
        let root = ExternalPrincipal::from_slice(&[2; 29]);
        for status in [
            fleet_registry::FleetSubnetRootStatus::Joining,
            fleet_registry::FleetSubnetRootStatus::Active,
            fleet_registry::FleetSubnetRootStatus::Draining,
            fleet_registry::FleetSubnetRootStatus::Removed,
        ] {
            let original = fleet_registry::FleetSubnetRootDirectoryEntry {
                placement_subnet: SubnetId::from_principal(subnet),
                fleet_subnet_root: root,
                status,
            };
            let native = native::FleetSubnetRootDirectoryEntry::from(original.clone());
            assert_eq!(native.placement_subnet.as_slice(), subnet.as_slice());
            assert_eq!(native.fleet_subnet_root.as_slice(), root.as_slice());
            let restored = fleet_registry::FleetSubnetRootDirectoryEntry::from(native);
            assert_eq!(restored, original);
        }
    }
}
