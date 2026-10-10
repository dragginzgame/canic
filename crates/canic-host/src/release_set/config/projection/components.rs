//! Project the configured deployable roles into deterministic display/build order.

// Enumerate deployable roles across all Component Specs except implicit Wasm stores.

use canic_contracts::ids::CanisterRole;
use canic_core::bootstrap::compiled::ConfigModel;

pub(in crate::release_set) fn configured_deployable_roles_from_config(
    config: &ConfigModel,
) -> Vec<String> {
    sort_component_roles(
        config
            .deployable_roles()
            .into_iter()
            .map(|role| role.as_str().to_string())
            .collect(),
    )
}

// Sort display/build roles deterministically, keeping `root` first when present.
fn sort_component_roles(mut roles: Vec<String>) -> Vec<String> {
    roles.sort_by(|left, right| {
        match (
            left == CanisterRole::ROOT.as_str(),
            right == CanisterRole::ROOT.as_str(),
        ) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => left.cmp(right),
        }
    });
    roles
}
