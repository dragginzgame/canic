//! Correlate authenticated Coordinator registration and Root authority before import effects.
//!
//! Transport verifies replies; this owner compares their complete current-contract bindings.

#[cfg(test)]
pub(in crate::fleet_ensure::ops::capacity_import) mod tests;

use crate::fleet_ensure::{
    model::capacity_import::{CapacityImportAuthority, CapacityImportPlanRecord},
    ops::capacity_import::verify_review,
};
use candid::Principal;
use canic_control_plane::api::canister_pool::CanisterPoolApi;
use canic_core::{
    dto::{
        fleet_registry::{FleetRegistry, FleetSubnetRootStatus},
        pool_import::{PoolImportContext, PoolImportIdentity},
    },
    ids::{
        ComponentSpecAdmission, ComponentTopologyDigest, FleetBinding,
        FleetSubnetRootFundingAuthority, FleetSubnetRootLimits, SubnetId,
    },
};
use thiserror::Error;

/// A missing or changed infrastructure prerequisite never authorizes implicit setup.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CapacityImportPrerequisiteError {
    #[error("Root bootstrap holds different supplied capacity or operator authority")]
    BootstrapReservationMismatch,
    #[error("Coordinator authority differs from the reviewed Fleet; verify operator setup")]
    CoordinatorAuthorityMismatch,
    #[error("Root authority differs from the reviewed import")]
    RootAuthorityMismatch,
    #[error("Root is not Active in the Coordinator registry; complete registration and activation")]
    RootNotActive,
    #[error("Root has no unique Coordinator registration; verify Fleet setup")]
    RootNotRegistered,
    #[error("Root policy or placement differs from its Coordinator registration")]
    RootRegistrationMismatch,
    #[error(
        "Root no longer retains this exact import reservation; preserve the original handoff intent"
    )]
    RootReservationMissing,
}

/// Validate exact reviewed identity and the Root's current registration without IC effects.
pub fn validate_destination_authority(
    plan: &CapacityImportPlanRecord,
    context: &PoolImportContext,
    registry: &FleetRegistry,
) -> Result<(), CapacityImportPrerequisiteError> {
    verify_review(plan, plan.plan_sha256)
        .map_err(|_| CapacityImportPrerequisiteError::RootAuthorityMismatch)?;
    validate_authority(&plan.authority, context, registry)?;
    if let Some(hold) = &context.bootstrap {
        let ids = plan
            .sources
            .iter()
            .map(|source| source.binding.canister_id)
            .collect::<Vec<_>>();
        if hold.operator != plan.authority.operator || hold.sources != ids {
            return Err(CapacityImportPrerequisiteError::BootstrapReservationMismatch);
        }
    }
    Ok(())
}

/// Check typed destination bindings before an initial review has balance samples or a digest.
pub(in crate::fleet_ensure::ops::capacity_import) fn validate_authority(
    reviewed: &CapacityImportAuthority,
    context: &PoolImportContext,
    registry: &FleetRegistry,
) -> Result<(), CapacityImportPrerequisiteError> {
    let binding = &context.binding;
    let observed = &binding.authority.binding;
    let expected = DestinationIdentity {
        fleet: &reviewed.fleet,
        coordinator: reviewed.coordinator,
        root: reviewed.root,
        subnet: reviewed.subnet,
        recovery_controllers: &reviewed.recovery_controllers,
    };
    let actual = DestinationIdentity {
        fleet: &observed.fleet,
        coordinator: observed.coordinator,
        root: binding.fleet_subnet_root,
        subnet: binding.placement_subnet,
        recovery_controllers: &observed.recovery_controllers,
    };
    let hash = CanisterPoolApi::import_authority_hash(binding)
        .map_err(|_| CapacityImportPrerequisiteError::RootAuthorityMismatch)?;
    if expected != actual
        || hash != reviewed.root_authority_sha256
        || hash != context.root_authority_sha256
    {
        return Err(CapacityImportPrerequisiteError::RootAuthorityMismatch);
    }
    if registry.authority != binding.authority {
        return Err(CapacityImportPrerequisiteError::CoordinatorAuthorityMismatch);
    }
    let mut entries = registry
        .fleet_subnet_roots
        .iter()
        .filter(|entry| entry.fleet_subnet_root == reviewed.root);
    let entry = entries
        .next()
        .ok_or(CapacityImportPrerequisiteError::RootNotRegistered)?;
    if entries.next().is_some() {
        return Err(CapacityImportPrerequisiteError::RootNotRegistered);
    }
    if entry.status != FleetSubnetRootStatus::Active {
        return Err(CapacityImportPrerequisiteError::RootNotActive);
    }
    let expected = RootRegistration {
        subnet: binding.placement_subnet,
        admissions: &binding.component_admissions,
        topology: binding.component_topology_digest,
        limits: &binding.limits,
        funding: &binding.funding,
    };
    let actual = RootRegistration {
        subnet: entry.placement_subnet,
        admissions: &entry.component_admissions,
        topology: entry.component_topology_digest,
        limits: &entry.limits,
        funding: &entry.funding,
    };
    if actual != expected {
        return Err(CapacityImportPrerequisiteError::RootRegistrationMismatch);
    }
    Ok(())
}

/// A retained host receipt cannot substitute for the Root's current allocation fence.
pub(super) fn require_active_reservation(
    plan: &CapacityImportPlanRecord,
    context: &PoolImportContext,
) -> Result<(), CapacityImportPrerequisiteError> {
    let expected = PoolImportIdentity {
        sequence: plan.authority.import_sequence,
        plan_sha256: plan.plan_sha256,
    };
    if context.active_import != Some(expected) {
        return Err(CapacityImportPrerequisiteError::RootReservationMissing);
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct DestinationIdentity<'a> {
    fleet: &'a FleetBinding,
    coordinator: Principal,
    root: Principal,
    subnet: SubnetId,
    recovery_controllers: &'a [Principal],
}

#[derive(Eq, PartialEq)]
struct RootRegistration<'a> {
    subnet: SubnetId,
    admissions: &'a [ComponentSpecAdmission],
    topology: ComponentTopologyDigest,
    limits: &'a FleetSubnetRootLimits,
    funding: &'a FleetSubnetRootFundingAuthority,
}
