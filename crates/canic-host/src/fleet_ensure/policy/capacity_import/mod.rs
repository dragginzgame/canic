//! Pure admission and conservation checks for exact-ID Fleet capacity enrollment.
//! Observations, evidence verification, record construction and IC effects belong to ops.

pub mod bootstrap;
#[cfg(test)]
pub(crate) mod tests;

use crate::fleet_ensure::{
    model::{
        FLEET_ENSURE_SCHEMA_VERSION,
        capacity_import::{
            CapacityImportAuthority, CapacityImportDisposition, CapacityImportPlanRecord,
            CapacityImportSourceRecord,
        },
    },
    view::capacity_import::{
        CapacityImportDestinationView, CapacityImportOwnershipView, CapacityImportSourceView,
    },
};
use candid::Principal;
use canic_core::ids::{MAX_FLEET_CAPACITY_IMPORT_SOURCES, SubnetId};
use std::collections::BTreeSet;
use thiserror::Error;

/// Typed reasons a capacity review cannot authorize a handoff or wipe.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CapacityImportPolicyError {
    #[error("capacity import authority or controller set is invalid")]
    InvalidAuthority,
    #[error("capacity import requires a nonempty bounded set of distinct source IDs")]
    InvalidSources,
    #[error("source {canister} has no remaining canister-version space for a handoff")]
    VersionExhausted { canister: Principal },
    #[error("source {canister} must be stopped before importing its installed code")]
    SourceRunning { canister: Principal },
    #[error("source {canister} retains snapshots; retire them before capacity import")]
    SourceSnapshots { canister: Principal },
    #[error("capacity import destination is not ready or has a competing operation")]
    DestinationUnavailable,
    #[error("capacity import destination authority changed")]
    DestinationChanged,
    #[error("no unique destination Root on subnet {subnet:?}; select a Root explicitly")]
    DestinationAmbiguous { subnet: SubnetId },
    #[error("selected Root is not a destination on subnet {subnet:?}")]
    DestinationMissing { subnet: SubnetId },
    #[error("capacity import exceeds the destination's available physical capacity")]
    CapacityExceeded,
    #[error("source {canister} belongs to another role or its ownership is unresolved")]
    SourceAssigned { canister: Principal },
    #[error("source {canister} is on a different subnet")]
    WrongSubnet { canister: Principal },
    #[error("selected operator is not a controller of source {canister}")]
    OperatorNotController { canister: Principal },
    #[error("source {canister} authority changed since review")]
    SourceChanged { canister: Principal },
    #[error("source {canister} has no exact verified absence/retirement disposition")]
    DispositionUnresolved { canister: Principal },
    #[error("cycle bounds overflow or contain no effect allowance")]
    InvalidCycleBounds,
    #[error("canister {canister} lacks the reviewed readiness and debit headroom")]
    InsufficientCycles { canister: Principal },
    #[error("source {canister} cycle change exceeds its original reviewed debit")]
    ConservationUnproven { canister: Principal },
}

/// Select by verified placement; selection never substitutes another physical Root.
pub fn select_destination(
    roots: &[CapacityImportDestinationView],
    subnet: SubnetId,
    explicit_root: Option<Principal>,
) -> Result<&CapacityImportDestinationView, CapacityImportPolicyError> {
    let mut matches = roots.iter().filter(|root| {
        root.authority.subnet == subnet
            && explicit_root.is_none_or(|selected| selected == root.authority.root)
    });
    let selected = matches
        .next()
        .ok_or(CapacityImportPolicyError::DestinationMissing { subnet })?;
    if matches.next().is_some() {
        return Err(CapacityImportPolicyError::DestinationAmbiguous { subnet });
    }
    Ok(selected)
}

/// Validate the reviewed values without treating a stored digest as authenticated evidence.
pub fn validate_plan(plan: &CapacityImportPlanRecord) -> Result<(), CapacityImportPolicyError> {
    if plan.schema_version != FLEET_ENSURE_SCHEMA_VERSION {
        return Err(CapacityImportPolicyError::InvalidAuthority);
    }
    validate_authority(&plan.authority)?;
    if plan.sources.is_empty() || plan.sources.len() > MAX_FLEET_CAPACITY_IMPORT_SOURCES {
        return Err(CapacityImportPolicyError::InvalidSources);
    }
    let final_set = final_controllers(&plan.authority);
    let mut transitional = final_set.clone();
    transitional.insert(plan.authority.operator);
    if !canonical_controllers(&plan.final_controllers)
        || !canonical_controllers(&plan.transitional_controllers)
        || plan
            .final_controllers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != final_set
        || plan
            .transitional_controllers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            != transitional
    {
        return Err(CapacityImportPolicyError::InvalidAuthority);
    }
    if plan.root_budget.maximum_debit_cycles == 0
        || plan.root_budget.maximum_paid_calls == 0
        || plan.root_budget.minimum_retained_cycles == 0
    {
        return Err(CapacityImportPolicyError::InvalidCycleBounds);
    }
    require_headroom(
        plan.authority.root,
        plan.root_budget.observed_cycles,
        plan.root_budget.minimum_retained_cycles,
        plan.root_budget.maximum_debit_cycles,
    )?;
    source_total(
        plan.root_budget.observed_cycles,
        plan.root_budget.observed_reserved_cycles,
    )?;
    let mut seen = BTreeSet::new();
    let mut maximum_debit = plan.root_budget.maximum_debit_cycles;
    for source in &plan.sources {
        let canister = source.binding.canister_id;
        if !valid_principal(canister) || !seen.insert(canister) {
            return Err(CapacityImportPolicyError::InvalidSources);
        }
        if source.binding.snapshots_size_bytes != 0 {
            return Err(CapacityImportPolicyError::SourceSnapshots { canister });
        }
        source_total(source.observed_cycles, source.observed_reserved_cycles)?;
        if source.binding.module_sha256.is_some() && !source.binding.stopped {
            return Err(CapacityImportPolicyError::SourceRunning { canister });
        }
        if source.binding.canister_version.checked_add(3).is_none() {
            return Err(CapacityImportPolicyError::VersionExhausted { canister });
        }
        if canister == plan.authority.root
            || canister == plan.authority.coordinator
            || canister == plan.authority.operator
            || plan.authority.recovery_controllers.contains(&canister)
        {
            return Err(CapacityImportPolicyError::SourceAssigned { canister });
        }
        if source.binding.subnet != plan.authority.subnet {
            return Err(CapacityImportPolicyError::WrongSubnet { canister });
        }
        if !canonical_controllers(&source.binding.controllers) {
            return Err(CapacityImportPolicyError::InvalidAuthority);
        }
        if !source
            .binding
            .controllers
            .contains(&plan.authority.operator)
        {
            return Err(CapacityImportPolicyError::OperatorNotController { canister });
        }
        if disposition_digest(source) == [0; 32] {
            return Err(CapacityImportPolicyError::DispositionUnresolved { canister });
        }
        if source.minimum_ready_cycles == 0 || source.maximum_debit_cycles == 0 {
            return Err(CapacityImportPolicyError::InvalidCycleBounds);
        }
        require_headroom(
            canister,
            source.observed_cycles,
            source.minimum_ready_cycles,
            source.maximum_debit_cycles,
        )?;
        maximum_debit = maximum_debit
            .checked_add(source.maximum_debit_cycles)
            .ok_or(CapacityImportPolicyError::InvalidCycleBounds)?;
    }
    Ok(())
}

/// Admit only a complete fresh observation of the same reviewed destination and sources.
/// Subsequent effects must use their own retained intent and Root completion evidence.
pub fn admit_handoffs(
    plan: &CapacityImportPlanRecord,
    destination: &CapacityImportDestinationView,
    sources: &[CapacityImportSourceView],
) -> Result<(), CapacityImportPolicyError> {
    validate_plan(plan)?;
    if plan.authority != destination.authority
        || plan.root_budget.minimum_retained_cycles != destination.minimum_retained_cycles
    {
        return Err(CapacityImportPolicyError::DestinationChanged);
    }
    if !destination.ready || destination.draining || destination.competing_operation {
        return Err(CapacityImportPolicyError::DestinationUnavailable);
    }
    let additional = u32::try_from(plan.sources.len())
        .map_err(|_| CapacityImportPolicyError::CapacityExceeded)?;
    if destination
        .occupied_capacity
        .checked_add(additional)
        .is_none_or(|total| total > destination.maximum_capacity)
    {
        return Err(CapacityImportPolicyError::CapacityExceeded);
    }
    let root_debit = source_total(
        plan.root_budget.observed_cycles,
        plan.root_budget.observed_reserved_cycles,
    )?
    .checked_sub(source_total(
        destination.controlled_cycles,
        destination.reserved_cycles,
    )?)
    .ok_or(CapacityImportPolicyError::InvalidCycleBounds)?;
    if root_debit > plan.root_budget.maximum_debit_cycles {
        return Err(CapacityImportPolicyError::InvalidCycleBounds);
    }
    require_headroom(
        plan.authority.root,
        destination.controlled_cycles,
        destination.minimum_retained_cycles,
        plan.root_budget.maximum_debit_cycles - root_debit,
    )?;
    if sources.len() != plan.sources.len() {
        return Err(CapacityImportPolicyError::InvalidSources);
    }
    let mut seen = BTreeSet::new();
    for observed in sources {
        let canister = observed.binding.canister_id;
        if !seen.insert(canister) {
            return Err(CapacityImportPolicyError::InvalidSources);
        }
        let reviewed = plan
            .sources
            .iter()
            .find(|source| source.binding.canister_id == canister)
            .ok_or(CapacityImportPolicyError::SourceChanged { canister })?;
        if destination.assigned_canisters.contains(&canister) {
            return Err(CapacityImportPolicyError::SourceAssigned { canister });
        }
        admit_source_handoff(reviewed, observed)?;
    }
    Ok(())
}

/// Recheck one unissued source while the destination holds its exact reservation.
pub fn admit_source_handoff(
    reviewed: &CapacityImportSourceRecord,
    observed: &CapacityImportSourceView,
) -> Result<(), CapacityImportPolicyError> {
    let canister = reviewed.binding.canister_id;
    if reviewed.binding != observed.binding {
        return Err(CapacityImportPolicyError::SourceChanged { canister });
    }
    if observed.ownership != CapacityImportOwnershipView::Unassigned {
        return Err(CapacityImportPolicyError::SourceAssigned { canister });
    }
    if observed.disposition_evidence_sha256 != Some(disposition_digest(reviewed)) {
        return Err(CapacityImportPolicyError::DispositionUnresolved { canister });
    }
    // Observation debit spends the original review's allowance. A separate
    // funding review is required before admitting any additional credit.
    retained_source_debit(reviewed, observed.cycles, observed.reserved_cycles)?;
    Ok(())
}

/// Account from the original review, including after process restart.
pub fn retained_source_debit(
    reviewed: &CapacityImportSourceRecord,
    retained_cycles: u128,
    retained_reserved_cycles: u128,
) -> Result<u128, CapacityImportPolicyError> {
    let canister = reviewed.binding.canister_id;
    let total_before = source_total(reviewed.observed_cycles, reviewed.observed_reserved_cycles)?;
    let total_after = source_total(retained_cycles, retained_reserved_cycles)?;
    let debit = total_before
        .checked_sub(total_after)
        .filter(|debit| *debit <= reviewed.maximum_debit_cycles)
        .ok_or(CapacityImportPolicyError::ConservationUnproven { canister })?;
    if retained_cycles < reviewed.minimum_ready_cycles {
        return Err(CapacityImportPolicyError::InsufficientCycles { canister });
    }
    Ok(debit)
}

fn validate_authority(
    authority: &CapacityImportAuthority,
) -> Result<(), CapacityImportPolicyError> {
    let principals = [authority.operator, authority.coordinator, authority.root];
    let principals_valid = valid_principal(authority.subnet.into_principal())
        && principals
            .iter()
            .all(|principal| valid_principal(*principal));
    let roles_distinct = principals.iter().collect::<BTreeSet<_>>().len() == principals.len();
    let digests_present =
        authority.network_root_key_sha256 != [0; 32] && authority.root_authority_sha256 != [0; 32];
    let recovery_valid = authority
        .recovery_controllers
        .windows(2)
        .all(|pair| pair[0] < pair[1])
        && authority
            .recovery_controllers
            .iter()
            .all(|principal| valid_principal(*principal));
    if !principals_valid || !roles_distinct || !digests_present || !recovery_valid {
        return Err(CapacityImportPolicyError::InvalidAuthority);
    }
    Ok(())
}

fn final_controllers(authority: &CapacityImportAuthority) -> BTreeSet<Principal> {
    authority
        .recovery_controllers
        .iter()
        .copied()
        .chain([authority.root])
        .collect()
}

pub(in crate::fleet_ensure) const fn disposition_digest(
    source: &CapacityImportSourceRecord,
) -> [u8; 32] {
    match source.disposition {
        CapacityImportDisposition::AbsenceEvidence { evidence_sha256 }
        | CapacityImportDisposition::Retired { evidence_sha256 } => evidence_sha256,
    }
}

fn valid_principal(principal: Principal) -> bool {
    principal != Principal::anonymous() && principal != Principal::management_canister()
}

fn canonical_controllers(controllers: &[Principal]) -> bool {
    !controllers.is_empty()
        && controllers.len() <= 10
        && controllers
            .iter()
            .all(|principal| valid_principal(*principal))
        && controllers.windows(2).all(|pair| pair[0] < pair[1])
}

fn require_headroom(
    canister: Principal,
    cycles: u128,
    minimum: u128,
    maximum_debit: u128,
) -> Result<(), CapacityImportPolicyError> {
    let required = minimum
        .checked_add(maximum_debit)
        .ok_or(CapacityImportPolicyError::InvalidCycleBounds)?;
    if cycles < required {
        return Err(CapacityImportPolicyError::InsufficientCycles { canister });
    }
    Ok(())
}

/// Account for controlled native and reserved balances without treating reserve as liquid.
pub fn source_total(
    cycles: u128,
    reserved_cycles: u128,
) -> Result<u128, CapacityImportPolicyError> {
    cycles
        .checked_add(reserved_cycles)
        .ok_or(CapacityImportPolicyError::InvalidCycleBounds)
}
