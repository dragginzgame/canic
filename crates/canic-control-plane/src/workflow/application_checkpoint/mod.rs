//! Coordinate bounded Root checkpoint hooks over the complete active local unit.
//!
//! Ops owns records and conversions; applications own business freeze and release.

use crate::{
    ops::{
        application_checkpoint::RootCheckpointOps,
        component_registry::{ComponentRegistryOps, caller_authority::RootCallerOps},
    },
    view::component_registry::ComponentDirectoryPageSelection,
    workflow::root_authority::validated_root_authority,
};
use canic_contracts::{
    dto::{
        application_checkpoint::{
            RootCheckpointBeginRequest, RootCheckpointRequest, RootCheckpointStatus,
        },
        component_registry::ComponentLifecycleStatus,
        error::Error,
    },
    ids::{CallerReceiverAuthority, CallerRootAuthority, ManagedCanisterBinding},
};
use canic_core::control_plane_support::{
    error::InternalError, model::application_checkpoint::MAX_CHECKPOINT_PARTICIPANTS,
    workflow::runtime::fleet_activation::FleetActivationWorkflow,
};

pub fn begin(request: RootCheckpointBeginRequest) -> Result<RootCheckpointStatus, Error> {
    let root = issuer()?;
    if let Some(status) = RootCheckpointOps::status(&root)?
        && status.operation_id == request.operation_id
    {
        if request.root_install_id != status.root_install_id
            || request.expected_sequence.checked_add(1) != Some(status.sequence)
        {
            return Err(InternalError::conflict().into());
        }
        return Ok(status);
    }
    super::root_admission::require_catalog_mutation_allowed()?;
    let members = discover(&root)?;
    RootCheckpointOps::begin(&root, &request, members).map_err(Into::into)
}

pub fn status() -> Result<Option<RootCheckpointStatus>, Error> {
    RootCheckpointOps::status(&issuer()?).map_err(Into::into)
}

pub fn release(request: RootCheckpointRequest) -> Result<RootCheckpointStatus, Error> {
    // This checkpoint owner dispatches hooks only. Snapshot authority is not exposed;
    // the future paid-effect composition must gate release on original settlement.
    RootCheckpointOps::release(&issuer()?, &request).map_err(Into::into)
}

pub async fn advance(request: RootCheckpointRequest) -> Result<RootCheckpointStatus, Error> {
    let root = issuer()?;
    let Some(step) = RootCheckpointOps::next(&root, &request)? else {
        return RootCheckpointOps::status(&root)?
            .ok_or_else(|| InternalError::unavailable().into());
    };
    crate::ops::application_checkpoint::invoke(&step).await?;
    // Re-admit current installation and phase after await; a delayed prepare cannot
    // overwrite release intent or acknowledge a successor checkpoint.
    if issuer()? != root {
        return Err(InternalError::conflict().into());
    }
    RootCheckpointOps::acknowledge(&root, &request, &step.binding, step.expected)
        .map_err(Into::into)
}

fn issuer() -> Result<CallerRootAuthority, InternalError> {
    let (root, _) = validated_root_authority()?;
    Ok(CallerRootAuthority {
        registry: root.binding.authority,
        root: root.binding.fleet_subnet_root,
        install_id: FleetActivationWorkflow::status()?.identity.operation_id,
    })
}

fn discover(root: &CallerRootAuthority) -> Result<Vec<CallerReceiverAuthority>, InternalError> {
    crate::ops::component_provisioning::RootComponentProvisioningOps::require_ordinary_allocation_open()?;
    ComponentRegistryOps::require_admission_catalog_stable()?;
    let mut members = Vec::new();
    for partition in ComponentRegistryOps::root_component_partitions()? {
        if partition.status != ComponentLifecycleStatus::Active {
            return Err(InternalError::conflict());
        }
        members.push(receiver(
            root,
            ManagedCanisterBinding::Component(partition.binding.clone()),
        )?);
        let remaining = MAX_CHECKPOINT_PARTICIPANTS
            .checked_sub(members.len())
            .ok_or_else(InternalError::resource_exhausted)?;
        let page = ComponentRegistryOps::directory_page(
            partition.binding.component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: None,
                role: None,
                status: None,
                start_after: None,
            },
            remaining.saturating_add(1),
        )?;
        if page.next_cursor.is_some() || page.entries.len() > remaining {
            return Err(InternalError::resource_exhausted());
        }
        for entry in page.entries {
            if entry.status != ComponentLifecycleStatus::Active {
                return Err(InternalError::conflict());
            }
            members.push(receiver(
                root,
                ManagedCanisterBinding::ComponentChild(entry.binding),
            )?);
        }
    }
    Ok(members)
}

fn receiver(
    root: &CallerRootAuthority,
    binding: ManagedCanisterBinding,
) -> Result<CallerReceiverAuthority, InternalError> {
    let principal = match &binding {
        ManagedCanisterBinding::Component(member) => member.canister_id,
        ManagedCanisterBinding::ComponentChild(member) => member.canister_id,
    };
    let receiver =
        RootCallerOps::receiver_view(principal).ok_or_else(InternalError::unavailable)?;
    if receiver.retired
        || receiver.authority.issuer != *root
        || receiver.authority.receiver.binding != binding
    {
        return Err(InternalError::conflict());
    }
    if ComponentRegistryOps::managed_runtime_operation_id(&binding)?
        != receiver.authority.receiver.install_id
    {
        return Err(InternalError::conflict());
    }
    Ok(receiver.authority)
}
