//! Module: api::fleet_coordinator
//!
//! Responsibility: adapt Coordinator lifecycle and Registry endpoint calls to workflow.
//! Does not own: authorization policy, stable state, or Registry validation.
//! Boundary: the dedicated facade macro is the only canister export authority.

use crate::{
    dto::fleet_coordinator::FleetCoordinatorInitArgs,
    workflow::fleet_coordinator::FleetCoordinatorWorkflow,
};
use canic_core::{
    api::runtime::MemoryRuntimeApi,
    dto::{
        error::Error,
        fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
    },
};
use ic_cdk::api::{canister_self, is_controller, msg_caller};

///
/// FleetCoordinatorApi
///
/// Public adapter used by the built-in Fleet Coordinator canister exports.
///

pub struct FleetCoordinatorApi;

impl FleetCoordinatorApi {
    /// Restore memory invariants and synchronously commit fresh genesis during install.
    pub fn init(args: FleetCoordinatorInitArgs) {
        MemoryRuntimeApi::bootstrap_registry()
            .unwrap_or_else(|error| ic_cdk::trap(format!("memory bootstrap failed: {error}")));
        let caller = msg_caller();
        FleetCoordinatorWorkflow::initialize(args, caller, is_controller(&caller), canister_self())
            .unwrap_or_else(|error| {
                ic_cdk::trap(format!("Fleet Coordinator init failed: {error}"))
            });
    }

    pub fn registry() -> Result<FleetRegistry, Error> {
        FleetCoordinatorWorkflow::registry().map_err(Into::into)
    }

    pub fn manifest() -> Result<FleetRegistryManifest, Error> {
        FleetCoordinatorWorkflow::manifest().map_err(Into::into)
    }

    pub fn version() -> Result<FleetRegistryVersion, Error> {
        FleetCoordinatorWorkflow::version().map_err(Into::into)
    }
}
