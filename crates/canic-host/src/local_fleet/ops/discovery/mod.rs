//! Join maintained terminal Fleet inventory to this instance's observed placement.

use crate::local_fleet::{
    LocalFleetError,
    view::{LocalFleetDiscoveryView, LocalFleetView, LocalRoleView},
};
use candid::Principal;
use std::{collections::BTreeMap, path::Path};

/// Refuse any retained current role not present in the exact local allocation set.
pub fn resolve(
    workspace: &Path,
    fleet: &str,
    local: LocalFleetView,
) -> Result<LocalFleetDiscoveryView, LocalFleetError> {
    crate::component_operation::policy::validate_label(fleet)
        .map_err(|_| LocalFleetError::Configuration)?;
    let current = crate::fleet_ensure::resolve_current_fleet(workspace, &local.environment, fleet)
        .map_err(|error| LocalFleetError::Preparation(error.to_string()))?;
    let allocated = local
        .canisters
        .iter()
        .filter(|entry| entry.exists)
        .filter_map(|entry| entry.canister_id.map(|id| (id, entry.subnet_id)))
        .collect::<BTreeMap<_, _>>();
    let roles = current
        .registry
        .entries
        .into_iter()
        .map(|entry| {
            let id = Principal::from_text(&entry.pid).map_err(|_| LocalFleetError::Identity)?;
            Ok(LocalRoleView {
                canister_id: id,
                role: entry.role,
                parent_canister_id: entry.parent_pid,
                subnet_id: *allocated.get(&id).ok_or(LocalFleetError::Identity)?,
                module_sha256: entry.module_hash,
            })
        })
        .collect::<Result<Vec<_>, LocalFleetError>>()?;
    Ok(LocalFleetDiscoveryView {
        local,
        fleet: fleet.into(),
        roles,
    })
}
