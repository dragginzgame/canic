//! Module: view::fleet_service_peer
//!
//! Responsibility: model one Registry-derived remote Fleet-service peer requester.
//! Does not own: Registry validation, grant policy, persistence, or lifecycle effects.
//! Boundary: Fleet-service peer ops construct this read-only authority for workflow use.

use canic_core::{
    dto::component_registry::FleetServiceComponentRequester, ids::FleetSubnetRootBinding,
};

///
/// FleetServicePeerRequesterView
///
/// Exact remote requester identity plus its current protected owning-root binding.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetServicePeerRequesterView {
    pub requester: FleetServiceComponentRequester,
    pub root: FleetSubnetRootBinding,
}
