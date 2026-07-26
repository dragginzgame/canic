//! Module: fleet_install_plan
//!
//! Responsibility: expose immutable pre-effect multi-root Fleet installation planning.
//! Does not own: placement selection, Canister creation, installation, or Registry mutation.
//! Boundary: callers supply exact resolved placement/funding input before external effects.

mod model;
mod persistence;
#[cfg(test)]
mod tests;

pub use model::{
    FleetInstallPlan, FleetInstallPlanError, FleetInstallPlanRequest, PersistedFleetInstallPlan,
    PersistedFleetSubnetRootReleaseSet, PlannedCanisterCreationFunding, PlannedFleetCoordinator,
    PlannedFleetSubnetRoot, PlannedFleetSubnetRootInput,
};
pub use persistence::{compile_and_persist_fleet_install_plan, load_persisted_fleet_install_plan};

#[cfg(test)]
use persistence::{FLEET_INSTALL_PLAN_FILE, fleet_install_plan_path, root_release_set_path};
