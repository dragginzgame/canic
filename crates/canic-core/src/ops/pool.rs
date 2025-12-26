use crate::{cdk::api::canister_self, config::Config};
use candid::Principal;

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
pub fn pool_controllers() -> Vec<Principal> {
    let mut controllers = Config::get().controllers.clone();

    let root = canister_self();
    if !controllers.contains(&root) {
        controllers.push(root);
    }

    controllers
}
