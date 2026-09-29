//! Effect-free destination quotes for the complete import call envelope.

use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError,
        ops::ic::{
            mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
            nns::NnsRegistryOps,
        },
    },
};

/// Bound encoded target sizes without requiring source custody or an outbound observation.
pub fn maximum_call_debit(controllers: Vec<Principal>) -> Result<u128, InternalError> {
    let canister_id = Principal::from_slice(&[0; 29]);
    let settings = UpdateSettingsArgs {
        canister_id,
        settings: CanisterSettings {
            controllers: Some(controllers),
            ..CanisterSettings::default()
        },
        sender_canister_version: None,
    };
    let costs = [
        MgmtOps::canister_inspection_reserve(canister_id)?.required_liquid_cycles,
        MgmtOps::canister_history_call_cost(canister_id)?,
        MgmtOps::stop_canister_call_cost(canister_id)?,
        MgmtOps::uninstall_code_call_cost(canister_id)?,
        MgmtOps::update_settings_call_cost(&settings)?,
        NnsRegistryOps::subnet_lookup_call_cost(canister_id)?,
    ];
    costs
        .into_iter()
        .max()
        .filter(|cost| *cost > 0)
        .ok_or_else(InternalError::invariant)
}
