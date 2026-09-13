//! Join maintained terminal Fleet inventory to this instance's observed placement.

use crate::local_fleet::{
    LocalFleetError,
    view::{LocalFleetDiscoveryView, LocalFleetView, LocalRoleView},
};
use candid::Principal;
use canic_core::{cdk::utils::hash::hex_bytes, ids::CanonicalNetworkId};
use ic_testkit::pocket_ic::PocketIc;
use std::{collections::BTreeMap, path::Path};

/// Verify every terminal role in the owned instance, including imported Root-owned assets.
pub fn resolve(
    workspace: &Path,
    fleet: &str,
    local: LocalFleetView,
    pic: &PocketIc,
) -> Result<LocalFleetDiscoveryView, LocalFleetError> {
    crate::component_operation::policy::validate_label(fleet)
        .map_err(|_| LocalFleetError::Configuration)?;
    let current = crate::fleet_ensure::resolve_current_fleet(workspace, &local.environment, fleet)
        .map_err(|error| LocalFleetError::Preparation(error.to_string()))?;
    let key = super::runtime::guarded(|| pic.root_key())?.ok_or(LocalFleetError::Identity)?;
    let network = CanonicalNetworkId::from_der_root_trust_anchor(&key)
        .map_err(|_| LocalFleetError::Identity)?;
    let registry = current
        .initial_active_registry(fleet)
        .map_err(|error| LocalFleetError::Preparation(error.to_string()))?;
    if hex_bytes(&key) != local.root_key_der_hex
        || registry.authority.binding.fleet.fleet.canonical_network_id != network
        || crate::fleet_ensure::policy::expected_plan_sha256(&current.plan)
            != current.plan.plan_sha256
    {
        return Err(LocalFleetError::Identity);
    }
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
            let subnet_id = super::runtime::guarded(|| {
                pic.canister_exists(id)
                    .then(|| pic.get_subnet(id))
                    .flatten()
            })?
            .ok_or(LocalFleetError::Identity)?;
            if !local.application_subnets.contains(&subnet_id)
                || allocated
                    .get(&id)
                    .is_some_and(|expected| *expected != subnet_id)
            {
                return Err(LocalFleetError::Identity);
            }
            Ok(LocalRoleView {
                canister_id: id,
                role: entry.role,
                parent_canister_id: entry.parent_pid,
                subnet_id,
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
