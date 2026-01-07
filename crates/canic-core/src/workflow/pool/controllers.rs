use super::PoolWorkflow;
use crate::{
    Error,
    ops::{config::ConfigOps, ic::IcOps},
    workflow::prelude::*,
};

/// Return the controller set for pool canisters.
///
/// Mechanical helper used by workflow when creating or resetting
/// pool canisters.
///
/// Guarantees:
/// - Includes all configured controllers from `Config`
/// - Always includes the root canister as a controller
/// - Deduplicates the root if already present
///
/// This function:
/// - Does NOT perform authorization checks
/// - Does NOT mutate state
/// - Does NOT make IC calls
///
/// Policy decisions about *who* should control pool canisters
/// are assumed to be encoded in configuration.
impl PoolWorkflow {
    pub fn pool_controllers() -> Result<Vec<Principal>, Error> {
        let mut controllers = ConfigOps::controllers()?;

        let root = IcOps::canister_self();
        if !controllers.contains(&root) {
            controllers.push(root);
        }

        Ok(controllers)
    }
}
