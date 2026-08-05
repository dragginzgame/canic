//! Module: config::component_group_deployment::canonical
//!
//! Responsibility: encode the exact canonical flattened-deployment semantic section.
//! Does not own: source parsing, topology validation, hashing, or protected persistence.
//! Boundary: validated deployment projections become schema-v1 domain-separated bytes.

use crate::{
    config::{
        ComponentDeploymentLabel, ComponentDeploymentLimits, ComponentDeploymentMemberLimit,
        ComponentDeploymentPurpose, ComponentDeploymentSpawnGrantLimit,
        ComponentGroupDeploymentSpec, ComponentGroupDeploymentTopology, FleetServiceMemberPurpose,
        canonical::CanonicalEncoder,
    },
    ids::ComponentGroupMemberPath,
};

const COMPONENT_GROUP_DEPLOYMENT_TOPOLOGY_DOMAIN: &[u8] =
    b"canic/component-group-deployment-topology/v1";
const COMPONENT_GROUP_DEPLOYMENT_TOPOLOGY_SCHEMA_VERSION: u32 = 1;

/// Maximum canonical bytes for all flattened Component Group deployments.
pub const MAX_COMPONENT_GROUP_DEPLOYMENT_TOPOLOGY_CANONICAL_BYTES: usize = 2_097_152;

pub(super) fn encode(topology: &ComponentGroupDeploymentTopology) -> Vec<u8> {
    let mut encoder = CanonicalEncoder::new(
        COMPONENT_GROUP_DEPLOYMENT_TOPOLOGY_DOMAIN,
        COMPONENT_GROUP_DEPLOYMENT_TOPOLOGY_SCHEMA_VERSION,
    );
    encoder.u64(topology.component_group_deployments.len() as u64);
    for deployment in &topology.component_group_deployments {
        encode_deployment(&mut encoder, deployment);
    }
    encoder.finish()
}

fn encode_deployment(encoder: &mut CanonicalEncoder, deployment: &ComponentGroupDeploymentSpec) {
    encoder.string(deployment.deployment.as_str());
    encoder.string(deployment.component_group.as_str());
    encode_optional_purpose(encoder, deployment.service_purpose);
    encode_labels(encoder, &deployment.labels);
    encoder.u64(deployment.member_limits.len() as u64);
    for limit in &deployment.member_limits {
        encode_member_limit(encoder, limit);
    }
    encoder.u32(deployment.initial_placements);
    encoder.u32(deployment.maximum_placements);
    encoder.u32(deployment.placement.maximum_per_root);
    encoder.u32(deployment.placement.minimum_distinct_roots);
    encoder.u64(deployment.members.len() as u64);
    for member in &deployment.members {
        encode_member_path(encoder, &member.member_path);
        encoder.string(member.component_spec.as_str());
        encoder.bytes(&member.component_spec_hash);
        encode_deployment_purpose(encoder, &member.purpose);
        encode_labels(encoder, &member.labels);
        encode_effective_limits(encoder, &member.limits);
    }
}

fn encode_member_limit(encoder: &mut CanonicalEncoder, limit: &ComponentDeploymentMemberLimit) {
    encode_member_path(encoder, &limit.member);
    encode_optional_u32(encoder, limit.maximum_descendants);
    encode_optional_u64(encoder, limit.maximum_registry_bytes);
    encode_spawn_grants(encoder, &limit.spawn_grants);
}

fn encode_effective_limits(encoder: &mut CanonicalEncoder, limits: &ComponentDeploymentLimits) {
    encoder.u32(limits.maximum_descendants);
    encoder.u64(limits.maximum_registry_bytes);
    encode_spawn_grants(encoder, &limits.spawn_grant_reductions);
}

fn encode_spawn_grants(
    encoder: &mut CanonicalEncoder,
    grants: &[ComponentDeploymentSpawnGrantLimit],
) {
    encoder.u64(grants.len() as u64);
    for grant in grants {
        encoder.string(grant.parent_role.as_str());
        encoder.string(grant.child_role.as_str());
        encoder.u32(grant.maximum_instances_per_parent);
    }
}

fn encode_deployment_purpose(encoder: &mut CanonicalEncoder, purpose: &ComponentDeploymentPurpose) {
    match purpose {
        ComponentDeploymentPurpose::Ordinary => encoder.u8(0),
        ComponentDeploymentPurpose::FleetServiceMember {
            service,
            member_purpose,
        } => {
            encoder.u8(1);
            encoder.string(service.as_str());
            encode_purpose(encoder, *member_purpose);
        }
    }
}

fn encode_optional_purpose(
    encoder: &mut CanonicalEncoder,
    purpose: Option<FleetServiceMemberPurpose>,
) {
    match purpose {
        None => encoder.u8(0),
        Some(purpose) => {
            encoder.u8(1);
            encode_purpose(encoder, purpose);
        }
    }
}

fn encode_purpose(encoder: &mut CanonicalEncoder, purpose: FleetServiceMemberPurpose) {
    encoder.u8(match purpose {
        FleetServiceMemberPurpose::Authority => 0,
        FleetServiceMemberPurpose::Replica => 1,
        FleetServiceMemberPurpose::PoolMember => 2,
    });
}

fn encode_labels(encoder: &mut CanonicalEncoder, labels: &[ComponentDeploymentLabel]) {
    encoder.u64(labels.len() as u64);
    for label in labels {
        encoder.string(label.key.as_str());
        encoder.string(label.value.as_str());
    }
}

fn encode_member_path(encoder: &mut CanonicalEncoder, path: &ComponentGroupMemberPath) {
    encoder.u64(path.len() as u64);
    for member in path.as_slice() {
        encoder.string(member.as_str());
    }
}

fn encode_optional_u32(encoder: &mut CanonicalEncoder, value: Option<u32>) {
    match value {
        None => encoder.u8(0),
        Some(value) => {
            encoder.u8(1);
            encoder.u32(value);
        }
    }
}

fn encode_optional_u64(encoder: &mut CanonicalEncoder, value: Option<u64>) {
    match value {
        None => encoder.u8(0),
        Some(value) => {
            encoder.u8(1);
            encoder.u64(value);
        }
    }
}
