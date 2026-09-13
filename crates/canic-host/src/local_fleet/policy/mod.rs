//! Pure local configuration, allocation and exact session admission.

use crate::local_fleet::{
    LocalFleetError,
    model::{LocalAllocationInput, LocalFleetConfig},
};

/// Bound topology, memory reservation and the server's hard lifetime before startup.
pub fn validate(config: &LocalFleetConfig) -> Result<(), LocalFleetError> {
    crate::component_operation::policy::validate_label(&config.name)
        .map_err(|_| LocalFleetError::Configuration)?;
    if config.schema_version != 1 || config.gateway_port == 0 || !config.server_binary.is_absolute()
    {
        return Err(LocalFleetError::Configuration);
    }
    if config.server_binary_sha256.len() != 64
        || !config
            .server_binary_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(LocalFleetError::Configuration);
    }
    if !(2..=8).contains(&config.application_subnets)
        || !(8..=64).contains(&config.maximum_canisters)
    {
        return Err(LocalFleetError::Configuration);
    }
    if !(64 * 1024 * 1024..=2 * 1024 * 1024 * 1024).contains(&config.canister_memory_bytes)
        || !config.canister_memory_bytes.is_multiple_of(65_536)
    {
        return Err(LocalFleetError::Configuration);
    }
    if config.allocation_debit_cycles == 0 {
        return Err(LocalFleetError::Configuration);
    }
    if !(1..=60).contains(&config.request_timeout_secs)
        || !(60..=604_800).contains(&config.server_lifetime_secs)
    {
        return Err(LocalFleetError::Configuration);
    }
    Ok(())
}

/// Allocation names and controllers identify newly owned resources, never an adopted canister.
pub fn validate_allocation(
    input: &LocalAllocationInput,
    config: &LocalFleetConfig,
) -> Result<(), LocalFleetError> {
    for name in [&input.name, &input.role] {
        crate::component_operation::policy::validate_label(name)
            .map_err(|_| LocalFleetError::Configuration)?;
    }
    if input.application_subnet >= config.application_subnets
        || input.controller == candid::Principal::anonymous()
        || input.controller == candid::Principal::management_canister()
    {
        return Err(LocalFleetError::Configuration);
    }
    Ok(())
}
