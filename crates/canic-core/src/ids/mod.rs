//! Module: ids
//!
//! Responsibility: layer-neutral identifiers and boundary-safe primitives.
//! Does not own: business logic, policy decisions, or storage-backed types.
//! Boundary: exposes pure IDs, enums, and newtypes across ops, workflow, and API.

mod app;
mod build_network;
mod canister;
pub mod capability;
mod component;
mod component_deployment;
mod endpoint;
mod fleet;
mod fleet_topology;
mod intent;
mod metrics;
mod network;
mod release_build;
mod release_set;
mod subnet;

pub use app::AppId;
pub use build_network::BuildNetwork;
pub use canister::CanisterRole;
pub use capability as cap;
pub use component::{
    ComponentInstanceId, ComponentInstanceIdParseError, ComponentSpecId, ComponentSpecIdParseError,
};
pub use component_deployment::{
    COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS, ComponentDeploymentConfigurationDigest,
    ComponentDeploymentIdParseError, ComponentGroupDeploymentId, ComponentGroupMemberId,
    ComponentGroupMemberPath, ComponentGroupMemberPathError, ComponentGroupPlacementId,
    ComponentGroupSpecId, FleetServiceId,
};
pub use endpoint::{EndpointCall, EndpointCallKind, EndpointId};
pub use fleet::{
    FleetBinding, FleetId, FleetIdParseError, FleetKey, FleetName, FleetNameParseError,
};
pub use fleet_topology::{
    COORDINATOR_ROOT_FUNDING_EXECUTION_RESERVE_FLOOR_CYCLES, ComponentBinding,
    ComponentChildBinding, ComponentSpecAdmission, ComponentTopologyDigest, CyclesFundingBudget,
    FLEET_ROOT_FUNDING_CALL_RESERVATION_CYCLES, FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES,
    FLEET_SUBNET_ROOT_ICP_REFILL_FLOOR_CYCLES, FleetCoordinatorBinding,
    FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetRegistryAuthority,
    FleetSubnetCanisterPoolConfig, FleetSubnetRootAutomaticIcpRefillPolicy, FleetSubnetRootBinding,
    FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy, FleetSubnetRootIcpRefillPolicy,
    FleetSubnetRootLimits, FleetSubnetWasmStoreAuthority, MAX_FLEET_ROOT_FUNDING_SLOTS,
    ManagedCanisterBinding,
};
pub use intent::{IntentId, IntentResourceKey};
pub use metrics::{AccessMetricKind, SystemMetricKind};
pub use network::{
    CanonicalNetworkId, CanonicalNetworkIdParseError, CanonicalNetworkTrustAnchorError,
};
pub use release_build::{
    RELEASE_BUILD_ID_ENV, ReleaseBuildId, ReleaseBuildIdParseError, ReleaseBuildNonce,
};
pub use release_set::{FleetSubnetRootReleaseSet, ReleaseSetDigest};
pub use subnet::SubnetId;
