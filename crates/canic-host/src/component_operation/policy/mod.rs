//! Pure authority and monotonic-progress checks for operator Component requests.

use crate::component_operation::{
    ComponentOperationError,
    model::{ComponentAuthorityRecord, ComponentPhase, ComponentProgressRecord},
};

/// Confine local operation names and target labels to individual path components.
pub fn validate_label(value: &str) -> Result<(), ComponentOperationError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ComponentOperationError::InvalidLabel(value.to_string()));
    }
    Ok(())
}

/// A review never follows changed release, controller, placement or spec authority.
pub fn validate_authority(
    expected: &ComponentAuthorityRecord,
    observed: &ComponentAuthorityRecord,
) -> Result<(), ComponentOperationError> {
    if expected != observed {
        return Err(ComponentOperationError::Authority {
            field: "reviewed binding",
        });
    }
    Ok(())
}

/// Preserve the first allocation identity and reject backward or unrelated status.
pub fn validate_progress(
    authority: &ComponentAuthorityRecord,
    previous: Option<&ComponentProgressRecord>,
    observed: &ComponentProgressRecord,
) -> Result<(), ComponentOperationError> {
    let allocation_id = canic_core::ids::ComponentInstanceId::from_root_allocation(
        authority.binding.authority.binding.fleet.fleet,
        authority.binding.authority.epoch,
        authority.binding.fleet_subnet_root,
        observed.allocation_sequence,
    );
    if observed.component != allocation_id {
        return Err(ComponentOperationError::Progress);
    }
    if observed.phase == ComponentPhase::Removed || observed.allocation_sequence == 0 {
        return Err(ComponentOperationError::Progress);
    }
    if let Some(previous) = previous {
        let same_allocation = previous.component == observed.component
            && previous.allocation_sequence == observed.allocation_sequence;
        let monotonic =
            observed.phase >= previous.phase && (!previous.complete || observed.complete);
        let same_binding = previous
            .binding
            .as_ref()
            .is_none_or(|binding| observed.binding.as_ref() == Some(binding));
        if !(same_allocation && monotonic && same_binding) {
            return Err(ComponentOperationError::Progress);
        }
    }
    if let Some(binding) = &observed.binding {
        if binding.canister_id == candid::Principal::anonymous()
            || binding.canister_id == candid::Principal::management_canister()
        {
            return Err(ComponentOperationError::Progress);
        }
        let same_identity = binding.authority == authority.binding.authority
            && binding.fleet_subnet_root == authority.binding.fleet_subnet_root
            && binding.placement_subnet == authority.binding.placement_subnet
            && binding.component == observed.component;
        let same_spec = binding.component_spec == authority.component_spec
            && binding.spec_hash == authority.spec_hash
            && binding.role == authority.role;
        if !(same_identity && same_spec) {
            return Err(ComponentOperationError::Progress);
        }
    }
    if observed.phase >= ComponentPhase::Installed && observed.binding.is_none() {
        return Err(ComponentOperationError::Progress);
    }
    if observed.complete
        && (observed.phase != ComponentPhase::Committed || observed.binding.is_none())
    {
        return Err(ComponentOperationError::Progress);
    }
    Ok(())
}
