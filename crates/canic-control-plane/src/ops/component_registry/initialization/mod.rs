//! Module: ops::component_registry::initialization
//!
//! Responsibility: freeze bounded application bytes against an allocated identity.
//! Does not own: application encoding, authentication, or installation effects.
//! Boundary: exact replay is effect-free; changed bytes never rewrite an intent.

use super::*;
use canic_core::dto::component_registry::{
    ComponentApplicationInitialization, MAX_COMPONENT_APPLICATION_INIT_BYTES,
    RootComponentInitializationRequest,
};

impl ComponentRegistryOps {
    pub(crate) fn application_init_hash(
        initialization: Option<&ComponentApplicationInitialization>,
    ) -> Result<Option<[u8; 32]>, InternalError> {
        initialization
            .map(|initialization| {
                let encoded = candid::encode_one(initialization)
                    .map_err(|_| InternalError::invalid_input())?;
                Ok(Sha256::digest(encoded).into())
            })
            .transpose()
    }

    pub(crate) fn application_init_arguments(
        allocation: &RootComponentAllocationView,
        target: Principal,
    ) -> Result<Option<Vec<u8>>, InternalError> {
        let topology =
            canic_core::control_plane_support::ops::config::ConfigOps::component_topology()?;
        let spec = topology
            .get(&allocation.component_spec)
            .ok_or_else(InternalError::invariant)?;
        match &allocation.application_initialization {
            Some(initialization) => {
                validate_initialization(initialization, target)?;
                Ok(Some(initialization.arguments.clone()))
            }
            None if spec.application_init_required => Err(InternalError::unavailable()),
            None => Ok(None),
        }
    }

    pub(crate) fn bind_application_initialization(
        request: RootComponentInitializationRequest,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(request.operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let target = match &record.progress {
            RootComponentAllocationProgressRecord::Created { canister, .. }
            | RootComponentAllocationProgressRecord::InstallIntent { canister, .. }
            | RootComponentAllocationProgressRecord::Installed { canister, .. }
            | RootComponentAllocationProgressRecord::Verified { canister, .. }
            | RootComponentAllocationProgressRecord::Committed { canister, .. }
            | RootComponentAllocationProgressRecord::Removed { canister, .. } => *canister,
            _ => return Err(InternalError::conflict()),
        };
        validate_initialization(&request.initialization, target)?;
        if let Some(retained) = &record.application_initialization {
            return if retained == &request.initialization {
                Ok(allocation_record_to_view(record))
            } else {
                Err(InternalError::conflict())
            };
        }
        let RootComponentAllocationProgressRecord::Created { effect, .. } = &record.progress else {
            return Err(InternalError::conflict());
        };
        let charged = effect.charged_entry_bytes;
        let mut next = record.clone();
        next.application_initialization = Some(request.initialization);
        let additional = RootComponentRegistryStore::allocation_entry_bytes(&next)
            .checked_sub(RootComponentRegistryStore::allocation_entry_bytes(&record))
            .ok_or_else(InternalError::invariant)?;
        let next_charge = charged
            .checked_add(additional)
            .ok_or_else(InternalError::resource_exhausted)?;
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = current
            .encoded_bytes
            .checked_add(additional)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next_meta.encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }
        let RootComponentAllocationProgressRecord::Created { effect, .. } = &mut next.progress
        else {
            return Err(InternalError::invariant());
        };
        effect.charged_entry_bytes = next_charge;
        validate_charged_record_size(&next, next_charge)?;
        if RootComponentRegistryStore::allocation_entry_bytes(&next)
            > RootComponentRegistryStore::allocation_record_max_bytes()
        {
            return Err(InternalError::resource_exhausted());
        }
        RootComponentRegistryStore::replace_allocation(&current, next_meta, &record, next.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next))
    }
}

fn validate_initialization(
    initialization: &ComponentApplicationInitialization,
    target: Principal,
) -> Result<(), InternalError> {
    if initialization.target_canister != target || target == Principal::anonymous() {
        return Err(InternalError::conflict());
    }
    if initialization.arguments.is_empty()
        || initialization.arguments.len() > MAX_COMPONENT_APPLICATION_INIT_BYTES
    {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}
