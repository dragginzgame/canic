//! Module: ids::fleet_admission
//!
//! Responsibility: carry layer-neutral Fleet admission authority and selector identities.
//! Does not own: validation, hashing, persistence, mutation, or endpoint enforcement.
//! Boundary: protected host plans and canister protocols share these exact version-1 shapes.

use super::{
    ComponentInstanceId, ComponentSpecId, FleetBinding, FleetCoordinatorBinding,
    ManagedCanisterBinding, SubnetId,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Maintained generation and wire schema for Fleet admission authority.
pub const FLEET_ADMISSION_SCHEMA_VERSION: u16 = 1;
/// First policy generation installed into a fresh Fleet.
pub const FLEET_ADMISSION_INITIAL_GENERATION: u64 = 1;
/// Maximum Principals in the Fleet-wide admission set.
pub const MAX_FLEET_ADMISSION_PRINCIPALS: usize = 256;
/// Maximum narrower selectors retained by one policy.
pub const MAX_FLEET_ADMISSION_RULES: usize = 32;
/// Maximum Principal references across every narrower rule.
pub const MAX_FLEET_ADMISSION_RULE_PRINCIPAL_REFERENCES: usize = 128;
/// Maximum encoded managed projection record retained at memory ID 61.
pub const MAX_FLEET_ADMISSION_PROJECTION_RECORD_BYTES: u32 = 32 * 1024;
/// Maximum Principals returned by one protected local projection page.
pub const MAX_FLEET_ADMISSION_PROJECTION_PAGE: u64 = 128;

/// Exact policy scope selected by protected input or a controller mutation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum FleetAdmissionSelector {
    #[serde(rename = "fleet")]
    Fleet,
    #[serde(rename = "component_spec")]
    ComponentSpec(ComponentSpecId),
    #[serde(rename = "component_instance")]
    ComponentInstance(ComponentInstanceId),
    #[serde(rename = "fleet_subnet_root")]
    FleetSubnetRoot(SubnetId),
}

/// One canonical narrower admission rule.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetAdmissionRule {
    pub selector: FleetAdmissionSelector,
    pub principals: Vec<Principal>,
}

/// Protected generation-one policy before a fresh Fleet identity exists.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetAdmissionPolicyTemplate {
    pub schema_version: u16,
    pub fleet_principals: Vec<Principal>,
    pub rules: Vec<FleetAdmissionRule>,
    pub template_digest: [u8; 32],
}

/// One exact Coordinator-owned Fleet admission policy.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetAdmissionPolicy {
    pub schema_version: u16,
    pub fleet: FleetBinding,
    pub generation: u64,
    pub fleet_principals: Vec<Principal>,
    pub rules: Vec<FleetAdmissionRule>,
    pub policy_digest: [u8; 32],
}

/// Exact managed target facts used to derive one effective local projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionTarget {
    pub component_spec: ComponentSpecId,
    pub component_instance: Option<ComponentInstanceId>,
    pub fleet_subnet_root: SubnetId,
}

/// One complete target-bound local enforcement projection.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetAdmissionProjection {
    pub schema_version: u16,
    pub authority: FleetCoordinatorBinding,
    pub target: ManagedCanisterBinding,
    pub generation: u64,
    pub policy_digest: [u8; 32],
    pub projection_digest: [u8; 32],
    pub principals: Vec<Principal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};

    #[test]
    fn selector_candid_labels_round_trip_through_serde() {
        let encoded = Encode!(&FleetAdmissionSelector::Fleet).expect("encode Fleet selector");
        let decoded = Decode!(&encoded, FleetAdmissionSelector).expect("decode Fleet selector");
        assert_eq!(decoded, FleetAdmissionSelector::Fleet);
    }
}
