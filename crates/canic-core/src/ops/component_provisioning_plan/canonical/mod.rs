//! Module: ops::component_provisioning_plan::canonical
//!
//! Responsibility: encode one validated provisioning plan or root batch into canonical bytes.
//! Does not own: configuration compilation, authority validation, root selection, or hashing.
//! Boundary: callers validate first; this module preserves the frozen v1 domains and byte order.

use super::{
    ComponentProvisioningPlanOpsError, MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES,
    MAX_FLEET_SUBNET_ROOT_PROVISIONING_BATCH_CANONICAL_BYTES,
};
use crate::{
    cdk::types::Cycles,
    config::{ComponentDeploymentLimits, ComponentDeploymentPurpose, FleetServiceMemberPurpose},
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetSubnetRootProvisioningBatch,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{ComponentGroupMemberPath, FleetRegistryAuthority, FleetSubnetRootLimits},
};

const PLAN_DOMAIN: &[u8] = b"canic/fleet-component-provisioning-plan/v1";
const ROOT_BATCH_DOMAIN: &[u8] = b"canic/fleet-subnet-root-provisioning-batch/v1";
const PLAN_SCHEMA_VERSION: u32 = 1;

pub(super) fn plan_bytes(
    plan: &FleetComponentProvisioningPlan,
) -> Result<Vec<u8>, ComponentProvisioningPlanOpsError> {
    let mut encoder = CanonicalEncoder::new();
    encode_plan(&mut encoder, plan);
    encoder.finish()
}

pub(super) fn root_batch_bytes(
    batch: &FleetSubnetRootProvisioningBatch,
) -> Result<Vec<u8>, ComponentProvisioningPlanOpsError> {
    let mut encoder = CanonicalEncoder::with_domain(ROOT_BATCH_DOMAIN);
    encode_batch(&mut encoder, batch);
    encoder.finish_with_bound(MAX_FLEET_SUBNET_ROOT_PROVISIONING_BATCH_CANONICAL_BYTES)
}

fn encode_plan(encoder: &mut CanonicalEncoder, plan: &FleetComponentProvisioningPlan) {
    encode_fleet(encoder, &plan.fleet);
    encode_registry_version(encoder, &plan.fleet_registry);
    encoder.bytes(plan.configuration_digest.as_bytes());
    encode_operation(encoder, &plan.operation);
    encoder.u64(plan.directory_confirmation_roots.len() as u64);
    for root in &plan.directory_confirmation_roots {
        encoder.bytes(root.as_slice());
    }
    encoder.u64(plan.batches.len() as u64);
    for batch in &plan.batches {
        encode_batch(encoder, batch);
    }
}

fn encode_fleet(encoder: &mut CanonicalEncoder, fleet: &crate::ids::FleetBinding) {
    encoder.bytes(fleet.fleet.canonical_network_id.as_bytes());
    encoder.bytes(fleet.fleet.fleet_id.as_bytes());
    encoder.string(fleet.app.as_str());
}

fn encode_registry_version(encoder: &mut CanonicalEncoder, version: &FleetRegistryVersion) {
    encode_authority(encoder, &version.authority);
    encoder.u64(version.revision);
    encoder.bytes(&version.content_hash);
}

fn encode_authority(encoder: &mut CanonicalEncoder, authority: &FleetRegistryAuthority) {
    encode_fleet(encoder, &authority.binding.fleet);
    encoder.bytes(
        authority
            .binding
            .coordinator_subnet
            .as_principal()
            .as_slice(),
    );
    encoder.bytes(authority.binding.coordinator.as_slice());
    encoder.u64(authority.epoch);
}

fn encode_operation(
    encoder: &mut CanonicalEncoder,
    operation: &FleetComponentProvisioningOperation,
) {
    match operation {
        FleetComponentProvisioningOperation::FreshInstall => encoder.u8(0),
        FleetComponentProvisioningOperation::ScaleOut {
            deployment,
            previous_placements,
            requested_placements,
        } => {
            encoder.u8(1);
            encoder.string(deployment.as_str());
            encoder.u32(*previous_placements);
            encoder.u32(*requested_placements);
        }
    }
}

fn encode_batch(encoder: &mut CanonicalEncoder, batch: &FleetSubnetRootProvisioningBatch) {
    encode_authority(encoder, &batch.root.authority);
    encoder.bytes(batch.root.placement_subnet.as_principal().as_slice());
    encoder.bytes(batch.root.fleet_subnet_root.as_slice());
    encoder.u64(batch.root.component_admissions.len() as u64);
    for admission in &batch.root.component_admissions {
        encoder.string(admission.component_spec.as_str());
        encoder.bytes(&admission.spec_hash);
        encoder.u32(admission.maximum_root_instances);
    }
    encoder.bytes(batch.root.component_topology_digest.as_bytes());
    encode_root_limits(encoder, &batch.root.limits);
    encoder.bytes(batch.active_release_set.release_build_id.as_bytes());
    encoder.bytes(batch.active_release_set.manifest_digest.as_bytes());
    encoder.u64(batch.placements.len() as u64);
    for placement in &batch.placements {
        encode_placement(encoder, placement);
    }
}

fn encode_root_limits(encoder: &mut CanonicalEncoder, limits: &FleetSubnetRootLimits) {
    encoder.u32(limits.maximum_component_instances);
    encoder.u64(limits.maximum_registry_bytes);
    encoder.u64(limits.maximum_wasm_store_bytes);
    encoder.u32(limits.canister_pool.minimum_size);
    encoder.u32(limits.canister_pool.maximum_size);
    encode_cycles(encoder, &limits.canister_pool.canister_cycles);
    encoder.u64(limits.cycles_funding.window_secs);
    encode_cycles(encoder, &limits.cycles_funding.maximum_cycles);
    encoder.u32(limits.maximum_group_placements);
}

fn encode_cycles(encoder: &mut CanonicalEncoder, cycles: &Cycles) {
    encoder.u128(cycles.to_u128());
}

fn encode_placement(encoder: &mut CanonicalEncoder, placement: &ComponentGroupPlacementPlan) {
    encoder.string(placement.group_placement.deployment.as_str());
    encoder.u32(placement.group_placement.ordinal);
    encoder.string(placement.component_group.as_str());
    encoder.u64(placement.entries.len() as u64);
    for entry in &placement.entries {
        encode_entry(encoder, entry);
    }
}

fn encode_entry(encoder: &mut CanonicalEncoder, entry: &ComponentGroupPlanEntry) {
    encode_member_path(encoder, &entry.member_path);
    encoder.string(entry.component_spec.as_str());
    encoder.bytes(&entry.spec_hash);
    encode_purpose(encoder, &entry.purpose);
    encoder.u64(entry.labels.len() as u64);
    for label in &entry.labels {
        encoder.string(label.key.as_str());
        encoder.string(label.value.as_str());
    }
    encode_limits(encoder, &entry.limits);
}

fn encode_member_path(encoder: &mut CanonicalEncoder, path: &ComponentGroupMemberPath) {
    encoder.u64(path.len() as u64);
    for member in path.as_slice() {
        encoder.string(member.as_str());
    }
}

fn encode_purpose(encoder: &mut CanonicalEncoder, purpose: &ComponentDeploymentPurpose) {
    match purpose {
        ComponentDeploymentPurpose::Ordinary => encoder.u8(0),
        ComponentDeploymentPurpose::FleetServiceMember {
            service,
            member_purpose,
        } => {
            encoder.u8(1);
            encoder.string(service.as_str());
            encoder.u8(match member_purpose {
                FleetServiceMemberPurpose::Authority => 0,
                FleetServiceMemberPurpose::Replica => 1,
                FleetServiceMemberPurpose::PoolMember => 2,
            });
        }
    }
}

fn encode_limits(encoder: &mut CanonicalEncoder, limits: &ComponentDeploymentLimits) {
    encoder.u32(limits.maximum_descendants);
    encoder.u64(limits.maximum_registry_bytes);
    encoder.u64(limits.spawn_grant_reductions.len() as u64);
    for grant in &limits.spawn_grant_reductions {
        encoder.string(grant.parent_role.as_str());
        encoder.string(grant.child_role.as_str());
        encoder.u32(grant.maximum_instances_per_parent);
    }
}

struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    fn new() -> Self {
        Self::with_domain(PLAN_DOMAIN)
    }

    fn with_domain(domain: &[u8]) -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(domain);
        encoder.u32(PLAN_SCHEMA_VERSION);
        encoder
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn finish(self) -> Result<Vec<u8>, ComponentProvisioningPlanOpsError> {
        self.finish_with_bound(MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES)
    }

    fn finish_with_bound(
        self,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, ComponentProvisioningPlanOpsError> {
        if self.bytes.len() > maximum_bytes {
            return Err(ComponentProvisioningPlanOpsError::CanonicalBytesExceeded {
                actual_bytes: self.bytes.len(),
                maximum_bytes,
            });
        }
        Ok(self.bytes)
    }
}
