//! Lossless transport conversions; stored allocation and installation records are unchanged.
use crate::dto::{
    component_provisioning as provisioning, component_read as native, component_registry as stored,
};
impl From<stored::RootComponentChildAllocationResponse> for native::ChildAllocation {
    fn from(value: stored::RootComponentChildAllocationResponse) -> Self {
        Self {
            last_failure: value.last_failure.map(Into::into),
            operation_id: value.operation_id,
            component: value.component,
            parent_canister_id: value.parent_canister_id.into(),
            parent_role: value.parent_role,
            child_role: value.child_role,
            child_kind: value.child_kind,
            maximum_instances_per_parent: value.maximum_instances_per_parent,
            maximum_descendants: value.maximum_descendants,
            maximum_registry_bytes: value.maximum_registry_bytes,
            reserved_against_registry: value.reserved_against_registry,
            release_set: value.release_set,
            phase: value.phase,
            creation: value.creation.map(Into::into),
            installation: value.installation.map(Into::into),
        }
    }
}

impl From<native::ChildAllocation> for stored::RootComponentChildAllocationResponse {
    fn from(value: native::ChildAllocation) -> Self {
        Self {
            last_failure: value.last_failure.map(Into::into),
            operation_id: value.operation_id,
            component: value.component,
            parent_canister_id: value.parent_canister_id.into(),
            parent_role: value.parent_role,
            child_role: value.child_role,
            child_kind: value.child_kind,
            maximum_instances_per_parent: value.maximum_instances_per_parent,
            maximum_descendants: value.maximum_descendants,
            maximum_registry_bytes: value.maximum_registry_bytes,
            reserved_against_registry: value.reserved_against_registry,
            release_set: value.release_set,
            phase: value.phase,
            creation: value.creation.map(Into::into),
            installation: value.installation.map(Into::into),
        }
    }
}

impl From<stored::RootComponentCreationEvidence> for native::Creation {
    fn from(value: stored::RootComponentCreationEvidence) -> Self {
        Self {
            wasm_store: value.wasm_store.into(),
            payload_hash: value.payload_hash,
            payload_size_bytes: value.payload_size_bytes,
            initial_cycles: value.initial_cycles,
            controller: value.controller.into(),
            canister: value.canister.map(Into::into),
        }
    }
}

impl From<native::Creation> for stored::RootComponentCreationEvidence {
    fn from(value: native::Creation) -> Self {
        Self {
            wasm_store: value.wasm_store.into(),
            payload_hash: value.payload_hash,
            payload_size_bytes: value.payload_size_bytes,
            initial_cycles: value.initial_cycles,
            controller: value.controller.into(),
            canister: value.canister.map(Into::into),
        }
    }
}

impl From<stored::RootComponentChildInstallEvidence> for native::ChildInstallation {
    fn from(value: stored::RootComponentChildInstallEvidence) -> Self {
        Self {
            raw_module_hash: value.raw_module_hash,
            chunk_hashes: value.chunk_hashes,
            binding: value.binding.into(),
        }
    }
}

impl From<native::ChildInstallation> for stored::RootComponentChildInstallEvidence {
    fn from(value: native::ChildInstallation) -> Self {
        Self {
            raw_module_hash: value.raw_module_hash,
            chunk_hashes: value.chunk_hashes,
            binding: value.binding.into(),
        }
    }
}

impl From<stored::RootComponentChildAllocationFailure> for native::ChildFailure {
    fn from(value: stored::RootComponentChildAllocationFailure) -> Self {
        Self {
            diagnostic_code: value.diagnostic_code,
            failed_at_ns: value.failed_at_ns,
            consecutive_failures: value.consecutive_failures,
            retry_at_ns: value.retry_at_ns,
            platform_rejection: value.platform_rejection,
        }
    }
}

impl From<native::ChildFailure> for stored::RootComponentChildAllocationFailure {
    fn from(value: native::ChildFailure) -> Self {
        Self {
            diagnostic_code: value.diagnostic_code,
            failed_at_ns: value.failed_at_ns,
            consecutive_failures: value.consecutive_failures,
            retry_at_ns: value.retry_at_ns,
            platform_rejection: value.platform_rejection,
        }
    }
}

impl From<provisioning::RootComponentProvisioningStatusResponse> for native::Provisioning {
    fn from(value: provisioning::RootComponentProvisioningStatusResponse) -> Self {
        Self {
            last_failure: value.last_failure.map(Into::into),
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            fleet_registry: value.fleet_registry.into(),
            configuration_digest: value.configuration_digest,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            estate_funding_required: value.estate_funding_required.map(Into::into),
            phase: value.phase,
            placement_count: value.placement_count,
            component_count: value.component_count,
            reserved_component_count: value.reserved_component_count,
            claimed_component_count: value.claimed_component_count,
            installed_component_count: value.installed_component_count,
            registry_committed_component_count: value.registry_committed_component_count,
            published_component_count: value.published_component_count,
            activated_component_count: value.activated_component_count,
            root_runtime_active: value.root_runtime_active,
            result: value.result.map(Into::into),
            publication: value.publication.map(Into::into),
            activation: value.activation,
            accepted_at_ns: value.accepted_at_ns,
            provisioned_at_ns: value.provisioned_at_ns,
            published_at_ns: value.published_at_ns,
            activation_started_at_ns: value.activation_started_at_ns,
            runtimes_activated_at_ns: value.runtimes_activated_at_ns,
            receipt_content_hash: value.receipt_content_hash,
        }
    }
}

impl From<native::Provisioning> for provisioning::RootComponentProvisioningStatusResponse {
    fn from(value: native::Provisioning) -> Self {
        Self {
            last_failure: value.last_failure.map(Into::into),
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            fleet_registry: value.fleet_registry.into(),
            configuration_digest: value.configuration_digest,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            estate_funding_required: value.estate_funding_required.map(Into::into),
            phase: value.phase,
            placement_count: value.placement_count,
            component_count: value.component_count,
            reserved_component_count: value.reserved_component_count,
            claimed_component_count: value.claimed_component_count,
            installed_component_count: value.installed_component_count,
            registry_committed_component_count: value.registry_committed_component_count,
            published_component_count: value.published_component_count,
            activated_component_count: value.activated_component_count,
            root_runtime_active: value.root_runtime_active,
            result: value.result.map(Into::into),
            publication: value.publication.map(Into::into),
            activation: value.activation,
            accepted_at_ns: value.accepted_at_ns,
            provisioned_at_ns: value.provisioned_at_ns,
            published_at_ns: value.published_at_ns,
            activation_started_at_ns: value.activation_started_at_ns,
            runtimes_activated_at_ns: value.runtimes_activated_at_ns,
            receipt_content_hash: value.receipt_content_hash,
        }
    }
}

impl From<provisioning::RootComponentProvisioningResult> for native::ProvisioningResult {
    fn from(value: provisioning::RootComponentProvisioningResult) -> Self {
        Self {
            placements: value.placements.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::ProvisioningResult> for provisioning::RootComponentProvisioningResult {
    fn from(value: native::ProvisioningResult) -> Self {
        Self {
            placements: value.placements.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<provisioning::RootProvisionedGroupPlacement> for native::Placement {
    fn from(value: provisioning::RootProvisionedGroupPlacement) -> Self {
        Self {
            group_placement: value.group_placement,
            component_group: value.component_group,
            members: value.members.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<native::Placement> for provisioning::RootProvisionedGroupPlacement {
    fn from(value: native::Placement) -> Self {
        Self {
            group_placement: value.group_placement,
            component_group: value.component_group,
            members: value.members.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<provisioning::RootProvisionedGroupMember> for native::Member {
    fn from(value: provisioning::RootProvisionedGroupMember) -> Self {
        Self {
            member_operation_id: value.member_operation_id,
            member_path: value.member_path,
            component_spec: value.component_spec,
            purpose: value.purpose,
            limits: value.limits,
            binding: value.binding.into(),
            component_registry_revision: value.component_registry_revision,
            component_registry_content_hash: value.component_registry_content_hash,
        }
    }
}

impl From<native::Member> for provisioning::RootProvisionedGroupMember {
    fn from(value: native::Member) -> Self {
        Self {
            member_operation_id: value.member_operation_id,
            member_path: value.member_path,
            component_spec: value.component_spec,
            purpose: value.purpose,
            limits: value.limits,
            binding: value.binding.into(),
            component_registry_revision: value.component_registry_revision,
            component_registry_content_hash: value.component_registry_content_hash,
        }
    }
}

impl From<provisioning::RootComponentPublicationEvidence> for native::Publication {
    fn from(value: provisioning::RootComponentPublicationEvidence) -> Self {
        Self {
            fleet_registry: value.fleet_registry.into(),
            fleet_directory_content_hash: value.fleet_directory_content_hash,
            component_directories: value.component_directories,
            component_group_directories: value.component_group_directories,
        }
    }
}

impl From<native::Publication> for provisioning::RootComponentPublicationEvidence {
    fn from(value: native::Publication) -> Self {
        Self {
            fleet_registry: value.fleet_registry.into(),
            fleet_directory_content_hash: value.fleet_directory_content_hash,
            component_directories: value.component_directories,
            component_group_directories: value.component_group_directories,
        }
    }
}

impl From<provisioning::RootEstateFundingRequired> for native::EstateFunding {
    fn from(value: provisioning::RootEstateFundingRequired) -> Self {
        Self {
            available: value.available,
            attempt_count: value.attempt_count,
            creation_amount: value.creation_amount,
            cycles_ledger: value.cycles_ledger.into(),
            execution_margin: value.execution_margin,
            last_attempt_at_ns: value.last_attempt_at_ns,
            ledger_fee: value.ledger_fee,
            management_creation_fee: value.management_creation_fee,
            operation_id: value.operation_id,
            readiness_floor: value.readiness_floor,
            required: value.required,
            retry_at_ns: value.retry_at_ns,
            root: value.root.into(),
            shortfall: value.shortfall,
        }
    }
}

impl From<native::EstateFunding> for provisioning::RootEstateFundingRequired {
    fn from(value: native::EstateFunding) -> Self {
        Self {
            available: value.available,
            attempt_count: value.attempt_count,
            creation_amount: value.creation_amount,
            cycles_ledger: value.cycles_ledger.into(),
            execution_margin: value.execution_margin,
            last_attempt_at_ns: value.last_attempt_at_ns,
            ledger_fee: value.ledger_fee,
            management_creation_fee: value.management_creation_fee,
            operation_id: value.operation_id,
            readiness_floor: value.readiness_floor,
            required: value.required,
            retry_at_ns: value.retry_at_ns,
            root: value.root.into(),
            shortfall: value.shortfall,
        }
    }
}

impl From<crate::dto::component_provisioning::FleetComponentProvisioningRootProgress>
    for crate::dto::component_read::CoordinatorRootProgress
{
    fn from(
        value: crate::dto::component_provisioning::FleetComponentProvisioningRootProgress,
    ) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_count: value.component_count,
            reserved_component_count: value.reserved_component_count,
            claimed_component_count: value.claimed_component_count,
            installed_component_count: value.installed_component_count,
            registry_committed_component_count: value.registry_committed_component_count,
        }
    }
}

impl From<crate::dto::component_read::CoordinatorRootProgress>
    for crate::dto::component_provisioning::FleetComponentProvisioningRootProgress
{
    fn from(value: crate::dto::component_read::CoordinatorRootProgress) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_count: value.component_count,
            reserved_component_count: value.reserved_component_count,
            claimed_component_count: value.claimed_component_count,
            installed_component_count: value.installed_component_count,
            registry_committed_component_count: value.registry_committed_component_count,
        }
    }
}

impl From<crate::dto::component_provisioning::FleetComponentPublicationRootProgress>
    for crate::dto::component_read::CoordinatorPublicationProgress
{
    fn from(
        value: crate::dto::component_provisioning::FleetComponentPublicationRootProgress,
    ) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_count: value.component_count,
            published_component_count: value.published_component_count,
        }
    }
}

impl From<crate::dto::component_read::CoordinatorPublicationProgress>
    for crate::dto::component_provisioning::FleetComponentPublicationRootProgress
{
    fn from(value: crate::dto::component_read::CoordinatorPublicationProgress) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_count: value.component_count,
            published_component_count: value.published_component_count,
        }
    }
}

impl From<crate::dto::component_provisioning::FleetComponentSynchronizationRootProgress>
    for crate::dto::component_read::CoordinatorSynchronizationProgress
{
    fn from(
        value: crate::dto::component_provisioning::FleetComponentSynchronizationRootProgress,
    ) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            affected_component_count: value.affected_component_count,
            synchronized_component_count: value.synchronized_component_count,
            complete: value.complete,
        }
    }
}

impl From<crate::dto::component_read::CoordinatorSynchronizationProgress>
    for crate::dto::component_provisioning::FleetComponentSynchronizationRootProgress
{
    fn from(value: crate::dto::component_read::CoordinatorSynchronizationProgress) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            affected_component_count: value.affected_component_count,
            synchronized_component_count: value.synchronized_component_count,
            complete: value.complete,
        }
    }
}

impl From<crate::dto::component_provisioning::FleetComponentActivationRootProgress>
    for crate::dto::component_read::CoordinatorActivationProgress
{
    fn from(
        value: crate::dto::component_provisioning::FleetComponentActivationRootProgress,
    ) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_count: value.component_count,
            activated_component_count: value.activated_component_count,
            root_runtime_active: value.root_runtime_active,
        }
    }
}

impl From<crate::dto::component_read::CoordinatorActivationProgress>
    for crate::dto::component_provisioning::FleetComponentActivationRootProgress
{
    fn from(value: crate::dto::component_read::CoordinatorActivationProgress) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_count: value.component_count,
            activated_component_count: value.activated_component_count,
            root_runtime_active: value.root_runtime_active,
        }
    }
}

impl From<crate::dto::component_provisioning::FleetComponentProvisioningRootFailure>
    for crate::dto::component_read::CoordinatorFailure
{
    fn from(
        value: crate::dto::component_provisioning::FleetComponentProvisioningRootFailure,
    ) -> Self {
        Self {
            origin: value.origin.map(Into::into),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            stage: value.stage,
            diagnostic_code: value.diagnostic_code,
            failed_at_ns: value.failed_at_ns,
        }
    }
}

impl From<crate::dto::component_read::CoordinatorFailure>
    for crate::dto::component_provisioning::FleetComponentProvisioningRootFailure
{
    fn from(value: crate::dto::component_read::CoordinatorFailure) -> Self {
        Self {
            origin: value.origin.map(Into::into),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            stage: value.stage,
            diagnostic_code: value.diagnostic_code,
            failed_at_ns: value.failed_at_ns,
        }
    }
}

impl From<crate::dto::component_provisioning::FleetComponentProvisioningStatusResponse>
    for crate::dto::component_read::CoordinatorProvisioning
{
    fn from(
        value: crate::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
    ) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            fleet_registry: value.fleet_registry.into(),
            configuration_digest: value.configuration_digest,
            operation: value.operation,
            phase: value.phase,
            directory_confirmation_root_count: value.directory_confirmation_root_count,
            root_batch_count: value.root_batch_count,
            accepted_root_count: value.accepted_root_count,
            acceptance_in_flight_root: value.acceptance_in_flight_root.map(Into::into),
            provisioned_root_count: value.provisioned_root_count,
            current_root: value.current_root.map(Into::into),
            provisioning_in_flight_root: value.provisioning_in_flight_root.map(Into::into),
            directory_confirmed_root_count: value.directory_confirmed_root_count,
            current_synchronization: value.current_synchronization.map(Into::into),
            current_publication: value.current_publication.map(Into::into),
            publication_in_flight_root: value.publication_in_flight_root.map(Into::into),
            runtime_activated_root_count: value.runtime_activated_root_count,
            current_activation: value.current_activation.map(Into::into),
            activation_in_flight_root: value.activation_in_flight_root.map(Into::into),
            pending_root_failure: value.pending_root_failure.map(Into::into),
            estate_funding_required: value.estate_funding_required.map(Into::into),
            group_placement_count: value.group_placement_count,
            component_count: value.component_count,
            planned_at_ns: value.planned_at_ns,
            roots_accepted_at_ns: value.roots_accepted_at_ns,
            components_provisioned_at_ns: value.components_provisioned_at_ns,
            published_fleet_registry: value.published_fleet_registry.map(Into::into),
            service_topology_published_at_ns: value.service_topology_published_at_ns,
            directories_confirmed_at_ns: value.directories_confirmed_at_ns,
            runtimes_activated_at_ns: value.runtimes_activated_at_ns,
        }
    }
}

impl From<crate::dto::component_read::CoordinatorProvisioning>
    for crate::dto::component_provisioning::FleetComponentProvisioningStatusResponse
{
    fn from(value: crate::dto::component_read::CoordinatorProvisioning) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            fleet_registry: value.fleet_registry.into(),
            configuration_digest: value.configuration_digest,
            operation: value.operation,
            phase: value.phase,
            directory_confirmation_root_count: value.directory_confirmation_root_count,
            root_batch_count: value.root_batch_count,
            accepted_root_count: value.accepted_root_count,
            acceptance_in_flight_root: value.acceptance_in_flight_root.map(Into::into),
            provisioned_root_count: value.provisioned_root_count,
            current_root: value.current_root.map(Into::into),
            provisioning_in_flight_root: value.provisioning_in_flight_root.map(Into::into),
            directory_confirmed_root_count: value.directory_confirmed_root_count,
            current_synchronization: value.current_synchronization.map(Into::into),
            current_publication: value.current_publication.map(Into::into),
            publication_in_flight_root: value.publication_in_flight_root.map(Into::into),
            runtime_activated_root_count: value.runtime_activated_root_count,
            current_activation: value.current_activation.map(Into::into),
            activation_in_flight_root: value.activation_in_flight_root.map(Into::into),
            pending_root_failure: value.pending_root_failure.map(Into::into),
            estate_funding_required: value.estate_funding_required.map(Into::into),
            group_placement_count: value.group_placement_count,
            component_count: value.component_count,
            planned_at_ns: value.planned_at_ns,
            roots_accepted_at_ns: value.roots_accepted_at_ns,
            components_provisioned_at_ns: value.components_provisioned_at_ns,
            published_fleet_registry: value.published_fleet_registry.map(Into::into),
            service_topology_published_at_ns: value.service_topology_published_at_ns,
            directories_confirmed_at_ns: value.directories_confirmed_at_ns,
            runtimes_activated_at_ns: value.runtimes_activated_at_ns,
        }
    }
}

impl From<crate::dto::component_provisioning::ProvisioningFailureOrigin>
    for crate::dto::component_read::FailureOrigin
{
    fn from(value: crate::dto::component_provisioning::ProvisioningFailureOrigin) -> Self {
        Self {
            dependency: value.dependency.map(Into::into),
            failed_at_ns: value.failed_at_ns,
            retry_at_ns: value.retry_at_ns,
            stage: value.stage,
            target: value.target.into(),
            operation_id: value.operation_id,
            diagnostic_code: value.diagnostic_code,
            retry_category: value.retry_category,
        }
    }
}

impl From<crate::dto::component_read::FailureOrigin>
    for crate::dto::component_provisioning::ProvisioningFailureOrigin
{
    fn from(value: crate::dto::component_read::FailureOrigin) -> Self {
        Self {
            dependency: value.dependency.map(Into::into),
            failed_at_ns: value.failed_at_ns,
            retry_at_ns: value.retry_at_ns,
            stage: value.stage,
            target: value.target.into(),
            operation_id: value.operation_id,
            diagnostic_code: value.diagnostic_code,
            retry_category: value.retry_category,
        }
    }
}

impl From<crate::dto::root::RootComponentOperationStatus> for native::ComponentAllocation {
    fn from(value: crate::dto::root::RootComponentOperationStatus) -> Self {
        Self {
            allocation: value.allocation.into(),
            complete: value.complete,
        }
    }
}

impl From<native::ComponentAllocation> for crate::dto::root::RootComponentOperationStatus {
    fn from(value: native::ComponentAllocation) -> Self {
        Self {
            allocation: value.allocation.into(),
            complete: value.complete,
        }
    }
}

impl From<stored::RootComponentAllocationResponse> for native::Allocation {
    fn from(value: stored::RootComponentAllocationResponse) -> Self {
        Self {
            application_initialization: value.application_initialization.map(Into::into),
            operation_id: value.operation_id,
            allocation_sequence: value.allocation_sequence,
            component: value.component,
            component_spec: value.component_spec,
            spec_hash: value.spec_hash,
            role: value.role,
            provisioning_origin: value.provisioning_origin.into(),
            release_set: value.release_set,
            phase: value.phase,
            creation: value.creation.map(Into::into),
            installation: value.installation.map(Into::into),
        }
    }
}

impl From<native::Allocation> for stored::RootComponentAllocationResponse {
    fn from(value: native::Allocation) -> Self {
        Self {
            application_initialization: value.application_initialization.map(Into::into),
            operation_id: value.operation_id,
            allocation_sequence: value.allocation_sequence,
            component: value.component,
            component_spec: value.component_spec,
            spec_hash: value.spec_hash,
            role: value.role,
            provisioning_origin: value.provisioning_origin.into(),
            release_set: value.release_set,
            phase: value.phase,
            creation: value.creation.map(Into::into),
            installation: value.installation.map(Into::into),
        }
    }
}

impl From<stored::RootComponentInstallEvidence> for native::Installation {
    fn from(value: stored::RootComponentInstallEvidence) -> Self {
        Self {
            raw_module_hash: value.raw_module_hash,
            chunk_hashes: value.chunk_hashes,
            binding: value.binding.into(),
        }
    }
}

impl From<native::Installation> for stored::RootComponentInstallEvidence {
    fn from(value: native::Installation) -> Self {
        Self {
            raw_module_hash: value.raw_module_hash,
            chunk_hashes: value.chunk_hashes,
            binding: value.binding.into(),
        }
    }
}
