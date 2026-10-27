//! Lossless transport conversions; no policy, authority reconstruction or storage mutation.

use crate::{
    dto::{component_directory as wire, component_registry as registry},
    ids,
};

impl From<ids::FleetCoordinatorBinding> for wire::FleetCoordinatorBinding {
    fn from(value: ids::FleetCoordinatorBinding) -> Self {
        Self {
            fleet: value.fleet,
            coordinator_subnet: value.coordinator_subnet.into_principal().into(),
            coordinator: value.coordinator.into(),
            recovery_controllers: value
                .recovery_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<wire::FleetCoordinatorBinding> for ids::FleetCoordinatorBinding {
    fn from(value: wire::FleetCoordinatorBinding) -> Self {
        Self {
            fleet: value.fleet,
            coordinator_subnet: ids::SubnetId::from_principal(value.coordinator_subnet.into()),
            coordinator: value.coordinator.into(),
            recovery_controllers: value
                .recovery_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<ids::FleetRegistryAuthority> for wire::FleetRegistryAuthority {
    fn from(value: ids::FleetRegistryAuthority) -> Self {
        Self {
            binding: value.binding.into(),
            epoch: value.epoch,
        }
    }
}

impl From<wire::FleetRegistryAuthority> for ids::FleetRegistryAuthority {
    fn from(value: wire::FleetRegistryAuthority) -> Self {
        Self {
            binding: value.binding.into(),
            epoch: value.epoch,
        }
    }
}

impl From<ids::ComponentBinding> for wire::ComponentBinding {
    fn from(value: ids::ComponentBinding) -> Self {
        Self {
            authority: value.authority.into(),
            component: value.component,
            component_spec: value.component_spec,
            spec_hash: value.spec_hash,
            role: value.role,
            placement_subnet: value.placement_subnet.into_principal().into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            canister_id: value.canister_id.into(),
        }
    }
}

impl From<wire::ComponentBinding> for ids::ComponentBinding {
    fn from(value: wire::ComponentBinding) -> Self {
        Self {
            authority: value.authority.into(),
            component: value.component,
            component_spec: value.component_spec,
            spec_hash: value.spec_hash,
            role: value.role,
            placement_subnet: ids::SubnetId::from_principal(value.placement_subnet.into()),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            canister_id: value.canister_id.into(),
        }
    }
}

impl From<ids::ComponentChildBinding> for wire::ComponentChildBinding {
    fn from(value: ids::ComponentChildBinding) -> Self {
        Self {
            component: value.component.into(),
            parent_canister_id: value.parent_canister_id.into(),
            role: value.role,
            canister_id: value.canister_id.into(),
        }
    }
}

impl From<wire::ComponentChildBinding> for ids::ComponentChildBinding {
    fn from(value: wire::ComponentChildBinding) -> Self {
        Self {
            component: value.component.into(),
            parent_canister_id: value.parent_canister_id.into(),
            role: value.role,
            canister_id: value.canister_id.into(),
        }
    }
}

impl From<registry::ComponentDirectoryProvenance> for wire::ComponentDirectoryProvenance {
    fn from(value: registry::ComponentDirectoryProvenance) -> Self {
        Self {
            component: value.component.into(),
            source_fleet_subnet_root: value.source_fleet_subnet_root.into(),
            component_registry_revision: value.component_registry_revision,
            component_registry_content_hash: value.component_registry_content_hash,
            synchronized_at_ns: value.synchronized_at_ns,
        }
    }
}

impl From<wire::ComponentDirectoryProvenance> for registry::ComponentDirectoryProvenance {
    fn from(value: wire::ComponentDirectoryProvenance) -> Self {
        Self {
            component: value.component.into(),
            source_fleet_subnet_root: value.source_fleet_subnet_root.into(),
            component_registry_revision: value.component_registry_revision,
            component_registry_content_hash: value.component_registry_content_hash,
            synchronized_at_ns: value.synchronized_at_ns,
        }
    }
}

impl From<registry::ComponentDirectoryHead> for wire::ComponentDirectoryHead {
    fn from(value: registry::ComponentDirectoryHead) -> Self {
        Self {
            provenance: value.provenance.into(),
            descendant_count: value.descendant_count,
        }
    }
}

impl From<wire::ComponentDirectoryHead> for registry::ComponentDirectoryHead {
    fn from(value: wire::ComponentDirectoryHead) -> Self {
        Self {
            provenance: value.provenance.into(),
            descendant_count: value.descendant_count,
        }
    }
}

impl From<registry::ComponentDirectoryPageRequest> for wire::ComponentDirectoryPageRequest {
    fn from(value: registry::ComponentDirectoryPageRequest) -> Self {
        Self {
            directory: value.directory.into(),
            parent_canister_id: value.parent_canister_id.map(Into::into),
            role: value.role,
            status: value.status,
            cursor: value.cursor,
            limit: value.limit,
        }
    }
}

impl From<wire::ComponentDirectoryPageRequest> for registry::ComponentDirectoryPageRequest {
    fn from(value: wire::ComponentDirectoryPageRequest) -> Self {
        Self {
            directory: value.directory.into(),
            parent_canister_id: value.parent_canister_id.map(Into::into),
            role: value.role,
            status: value.status,
            cursor: value.cursor,
            limit: value.limit,
        }
    }
}

impl From<registry::ComponentDirectoryChildEntry> for wire::ComponentDirectoryChildEntry {
    fn from(value: registry::ComponentDirectoryChildEntry) -> Self {
        Self {
            binding: value.binding.into(),
            kind: value.kind,
            installed_artifact_hash: value.installed_artifact_hash,
            protocol_profile_digest: value.protocol_profile_digest,
            status: value.status,
        }
    }
}

impl From<wire::ComponentDirectoryChildEntry> for registry::ComponentDirectoryChildEntry {
    fn from(value: wire::ComponentDirectoryChildEntry) -> Self {
        Self {
            binding: value.binding.into(),
            kind: value.kind,
            installed_artifact_hash: value.installed_artifact_hash,
            protocol_profile_digest: value.protocol_profile_digest,
            status: value.status,
        }
    }
}

impl From<registry::ComponentDirectoryPageResponse> for wire::ComponentDirectoryPageResponse {
    fn from(value: registry::ComponentDirectoryPageResponse) -> Self {
        Self {
            directory: value.directory.into(),
            entries: value.entries.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor,
        }
    }
}

impl From<wire::ComponentDirectoryPageResponse> for registry::ComponentDirectoryPageResponse {
    fn from(value: wire::ComponentDirectoryPageResponse) -> Self {
        Self {
            directory: value.directory.into(),
            entries: value.entries.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor,
        }
    }
}

impl From<crate::ids::ManagedCanisterBinding>
    for crate::dto::component_directory::ManagedCanisterBinding
{
    fn from(value: crate::ids::ManagedCanisterBinding) -> Self {
        match value {
            crate::ids::ManagedCanisterBinding::Component(value) => Self::Component(value.into()),
            crate::ids::ManagedCanisterBinding::ComponentChild(value) => {
                Self::ComponentChild(value.into())
            }
        }
    }
}
impl From<crate::dto::component_directory::ManagedCanisterBinding>
    for crate::ids::ManagedCanisterBinding
{
    fn from(value: crate::dto::component_directory::ManagedCanisterBinding) -> Self {
        match value {
            crate::dto::component_directory::ManagedCanisterBinding::Component(value) => {
                Self::Component(value.into())
            }
            crate::dto::component_directory::ManagedCanisterBinding::ComponentChild(value) => {
                Self::ComponentChild(value.into())
            }
        }
    }
}
