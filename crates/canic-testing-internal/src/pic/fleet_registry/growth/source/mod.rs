//! Module: pic::fleet_registry::growth::source
//!
//! Responsibility: project the prepared fixture's protected policy into generator inputs.
//! Boundary: test input construction only; production generation validates every binding.

use canic_host::fleet_ensure::model::DesiredFleet;
use serde_json::{Value, json};

pub(super) fn document(desired: &DesiredFleet) -> Value {
    let bootstrap = desired.bootstrap.as_ref().unwrap();
    let root = &bootstrap.roots[0];
    let coordinator = bootstrap.root_funding.as_ref().unwrap();
    let funding = &root.funding.root_funding;
    let admissions = root
        .component_admissions
        .iter()
        .map(|entry| {
            (
                entry.component_spec.to_string(),
                json!(entry.maximum_root_instances),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let mut placements = serde_json::Map::new();
    for placement in &desired
        .protocol
        .as_ref()
        .unwrap()
        .component_group_placements
    {
        placements
            .entry(placement.deployment.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!(placement.ordinal));
    }
    json!({
        "schema_version": 1,
        "funding_profile": funding.funding_profile,
        "operator": desired.operator,
        "admission": {"principals": bootstrap.admission.fleet_principals.iter().map(ToString::to_string).collect::<Vec<_>>()},
        "coordinator": {
            "subnet": {"kind": "explicit", "subnet": bootstrap.coordinator_subnet.to_string(), "acknowledge_fiduciary_cost": false},
            "creation_funding": {"kind": "cycles", "cycles": "500T"},
            "root_funding": {
                "minimum_reserve_cycles": coordinator.minimum_reserve_cycles.to_config_string(),
                "window_secs": coordinator.budget.window_secs,
                "maximum_cycles": coordinator.budget.maximum_cycles.to_config_string(),
                "maximum_automatic_grants": coordinator.maximum_automatic_grants,
                "maximum_automatic_cycles": coordinator.maximum_automatic_cycles.to_config_string(),
            },
        },
        "fleet_subnet_roots": [{
            "placement_subnet": root.placement_subnet.to_string(),
            "acknowledge_fiduciary_cost": false,
            "component_admissions": admissions,
            "component_group_placements": placements,
            "canister_pool": {
                "minimum_size": root.limits.canister_pool.minimum_size,
                "maximum_size": root.limits.canister_pool.maximum_size,
                "canister_cycles": root.limits.canister_pool.canister_cycles.to_config_string(),
            },
            "root_funding": {
                "request_threshold": funding.request_threshold.to_config_string(),
                "target_balance": funding.target_balance.to_config_string(),
                "cooldown_secs": funding.cooldown_secs,
                "window_secs": funding.budget.window_secs,
                "maximum_cycles": funding.budget.maximum_cycles.to_config_string(),
                "maximum_automatic_grants": funding.maximum_automatic_grants,
                "maximum_automatic_cycles": funding.maximum_automatic_cycles.to_config_string(),
            },
            "limits": {
                "maximum_component_instances": root.limits.maximum_component_instances,
                "maximum_registry_bytes": root.limits.maximum_registry_bytes,
                "maximum_wasm_store_bytes": root.limits.maximum_wasm_store_bytes,
                "maximum_group_placements": root.limits.maximum_group_placements,
                "cycles_funding": {"window_secs": root.limits.cycles_funding.window_secs, "maximum_cycles": root.limits.cycles_funding.maximum_cycles.to_config_string()},
            },
            "root_creation_funding": {"kind": "cycles", "cycles": "80T"},
            "wasm_store_creation_funding": {"kind": "cycles", "cycles": "80T"},
        }],
    })
}
