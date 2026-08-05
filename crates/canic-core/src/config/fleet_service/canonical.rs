//! Module: config::fleet_service::canonical
//!
//! Responsibility: encode the exact canonical Fleet-service-target semantic section.
//! Does not own: source parsing, relationship validation, hashing, or protected persistence.
//! Boundary: validated service targets become schema-v1 domain-separated bytes.

use crate::config::{
    FleetServiceTarget, FleetServiceTargetMode, FleetServiceTopology, canonical::CanonicalEncoder,
};

const FLEET_SERVICE_TOPOLOGY_DOMAIN: &[u8] = b"canic/fleet-service-topology/v1";
const FLEET_SERVICE_TOPOLOGY_SCHEMA_VERSION: u32 = 1;

/// Maximum canonical bytes for the complete Fleet-service target topology.
pub const MAX_FLEET_SERVICE_TOPOLOGY_CANONICAL_BYTES: usize = 2_097_152;

pub(super) fn encode(topology: &FleetServiceTopology) -> Vec<u8> {
    let mut encoder = CanonicalEncoder::new(
        FLEET_SERVICE_TOPOLOGY_DOMAIN,
        FLEET_SERVICE_TOPOLOGY_SCHEMA_VERSION,
    );
    encoder.u64(topology.targets.len() as u64);
    for target in &topology.targets {
        encode_target(&mut encoder, target);
    }
    encoder.finish()
}

fn encode_target(encoder: &mut CanonicalEncoder, target: &FleetServiceTarget) {
    encoder.string(target.service.as_str());
    encoder.string(target.role.as_str());
    encoder.string(target.component_spec.as_str());
    match &target.mode {
        FleetServiceTargetMode::AuthorityReplica {
            authority_deployment,
            authority_member,
        } => {
            encoder.u8(0);
            encoder.string(authority_deployment.as_str());
            encoder.u64(authority_member.len() as u64);
            for member in authority_member.as_slice() {
                encoder.string(member.as_str());
            }
        }
        FleetServiceTargetMode::ActivePool => encoder.u8(1),
    }
    encoder.u32(target.placement.maximum_members_per_root);
    encoder.u32(target.placement.minimum_distinct_roots);
}
