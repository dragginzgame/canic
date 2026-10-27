//! Lossless registry-read boundary conversions; no authority or lifecycle decisions.
use crate::dto::{component_registry as stored, registry_read as wire};
impl From<stored::ComponentRegistryPartitionResponse> for wire::Partition {
    fn from(value: stored::ComponentRegistryPartitionResponse) -> Self {
        Self {
            head: value.head,
            binding: value.binding.into(),
            protocol_profile_digest: value.protocol_profile_digest,
            provisioning_origin: value.provisioning_origin.into(),
            release_set: value.release_set,
            status: value.status,
            reserved_descendants: value.reserved_descendants,
            committed_descendants: value.committed_descendants,
            encoded_bytes: value.encoded_bytes,
        }
    }
}
impl From<wire::Partition> for stored::ComponentRegistryPartitionResponse {
    fn from(value: wire::Partition) -> Self {
        Self {
            head: value.head,
            binding: value.binding.into(),
            protocol_profile_digest: value.protocol_profile_digest,
            provisioning_origin: value.provisioning_origin.into(),
            release_set: value.release_set,
            status: value.status,
            reserved_descendants: value.reserved_descendants,
            committed_descendants: value.committed_descendants,
            encoded_bytes: value.encoded_bytes,
        }
    }
}
impl From<stored::ComponentRegistryActivePartitionResponse> for wire::ActivePartition {
    fn from(value: stored::ComponentRegistryActivePartitionResponse) -> Self {
        Self {
            allocation_operation_id: value.allocation_operation_id,
            prepared: value.prepared.into(),
            activation: value.activation.into(),
            current: value.current.into(),
        }
    }
}
impl From<wire::ActivePartition> for stored::ComponentRegistryActivePartitionResponse {
    fn from(value: wire::ActivePartition) -> Self {
        Self {
            allocation_operation_id: value.allocation_operation_id,
            prepared: value.prepared.into(),
            activation: value.activation.into(),
            current: value.current.into(),
        }
    }
}
impl From<stored::FleetServiceComponentRequester> for wire::ServiceRequester {
    fn from(value: stored::FleetServiceComponentRequester) -> Self {
        Self {
            service: value.service,
            member_purpose: value.member_purpose,
            group_placement: value.group_placement,
            member_path: value.member_path,
            component: value.component.into(),
        }
    }
}
impl From<wire::ServiceRequester> for stored::FleetServiceComponentRequester {
    fn from(value: wire::ServiceRequester) -> Self {
        Self {
            service: value.service,
            member_purpose: value.member_purpose,
            group_placement: value.group_placement,
            member_path: value.member_path,
            component: value.component.into(),
        }
    }
}
impl From<stored::ComponentProvisioningOrigin> for wire::ProvisioningOrigin {
    fn from(value: stored::ComponentProvisioningOrigin) -> Self {
        match value {
            stored::ComponentProvisioningOrigin::FleetAdministrator { caller } => {
                Self::FleetAdministrator {
                    caller: caller.into(),
                }
            }
            stored::ComponentProvisioningOrigin::Component { requester, grant } => {
                Self::Component {
                    requester: Box::new((*requester).into()),
                    grant,
                }
            }
            stored::ComponentProvisioningOrigin::FleetServiceComponent {
                requester,
                registry,
                grant,
            } => Self::FleetServiceComponent {
                requester: Box::new((*requester).into()),
                registry: Box::new((*registry).into()),
                grant,
            },
            stored::ComponentProvisioningOrigin::ComponentGroup {
                operation_id,
                plan_hash,
                group_placement,
                member_path,
            } => Self::ComponentGroup {
                operation_id,
                plan_hash,
                group_placement,
                member_path,
            },
        }
    }
}
impl From<wire::ProvisioningOrigin> for stored::ComponentProvisioningOrigin {
    fn from(value: wire::ProvisioningOrigin) -> Self {
        match value {
            wire::ProvisioningOrigin::FleetAdministrator { caller } => Self::FleetAdministrator {
                caller: caller.into(),
            },
            wire::ProvisioningOrigin::Component { requester, grant } => Self::Component {
                requester: Box::new((*requester).into()),
                grant,
            },
            wire::ProvisioningOrigin::FleetServiceComponent {
                requester,
                registry,
                grant,
            } => Self::FleetServiceComponent {
                requester: Box::new((*requester).into()),
                registry: Box::new((*registry).into()),
                grant,
            },
            wire::ProvisioningOrigin::ComponentGroup {
                operation_id,
                plan_hash,
                group_placement,
                member_path,
            } => Self::ComponentGroup {
                operation_id,
                plan_hash,
                group_placement,
                member_path,
            },
        }
    }
}
