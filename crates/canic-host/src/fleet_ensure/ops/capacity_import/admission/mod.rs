//! Bind exact operator declarations and current infrastructure to the import review.
//!
//! Declarations attest to outside obligations; they do not prove them through IC queries.

mod declarations;
pub mod observer;
pub(crate) mod review;
pub mod survey;
#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::capacity_import::{
        CapacityImportPlanRecord,
        admission::{CapacityImportAdmissionRecord, CapacityImportInfrastructureKind},
    },
    ops::capacity_import::CapacityImportReviewError,
};
use candid::Principal;
use canic_core::{cdk::utils::hash::decode_hex, dto::fleet_registry::FleetRegistry};
use sha2_host::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub use declarations::{
    CapacityImportDeclaration, CapacityImportDeclarations, CapacityImportDispositionKind,
};

const MAXIMUM_DECLARATION_BYTES: usize = 256 * 1024;
const MAXIMUM_REGISTRY_BYTES: usize = 256 * 1024;
pub(super) const MAXIMUM_ROOTS: usize = 256;

/// Validate retained evidence without granting authenticity to its local digest.
pub fn validate(plan: &CapacityImportPlanRecord) -> Result<(), CapacityImportReviewError> {
    let Some(admission) = &plan.admission else {
        return Ok(());
    };
    let registry = registry(admission)?;
    validate_infrastructure(plan, admission, &registry)?;
    let declarations = declarations::parse(&admission.declarations_toml)?;
    let digest: [u8; 32] = Sha256::digest(admission.declarations_toml.as_bytes()).into();
    if digest != admission.declarations_sha256 {
        return Err(CapacityImportReviewError::AdmissionInvalid);
    }
    declarations::validate(plan, &declarations, digest)
}

/// Decode the exact current Registry retained with the review, within its response bound.
pub(super) fn registry(
    admission: &CapacityImportAdmissionRecord,
) -> Result<FleetRegistry, CapacityImportReviewError> {
    if admission.registry_candid_hex.len() > MAXIMUM_REGISTRY_BYTES * 2 {
        return Err(CapacityImportReviewError::AdmissionInvalid);
    }
    let bytes = decode_hex(&admission.registry_candid_hex)
        .map_err(|_| CapacityImportReviewError::AdmissionInvalid)?;
    candid::decode_one(&bytes).map_err(|_| CapacityImportReviewError::AdmissionInvalid)
}

fn validate_infrastructure(
    plan: &CapacityImportPlanRecord,
    admission: &CapacityImportAdmissionRecord,
    registry: &FleetRegistry,
) -> Result<(), CapacityImportReviewError> {
    let invalid = || CapacityImportReviewError::AdmissionInvalid;
    let authority = &plan.authority;
    let binding = &registry.authority.binding;
    // Compare membership without changing the retained Registry bytes or review hash.
    let mut recovery_controllers = binding.recovery_controllers.clone();
    recovery_controllers.sort_unstable();
    if (&binding.fleet, binding.coordinator, &recovery_controllers)
        != (
            &authority.fleet,
            authority.coordinator,
            &authority.recovery_controllers,
        )
    {
        return Err(invalid());
    }
    let roots = registry
        .fleet_subnet_roots
        .iter()
        .map(|entry| (entry.fleet_subnet_root, entry.placement_subnet))
        .collect::<BTreeMap<_, _>>();
    if roots.is_empty()
        || roots.len() > MAXIMUM_ROOTS
        || roots.len() != registry.fleet_subnet_roots.len()
        || roots.contains_key(&binding.coordinator)
        || roots.get(&authority.root) != Some(&authority.subnet)
    {
        return Err(invalid());
    }
    let destination = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == authority.root)
        .ok_or_else(invalid)?;
    if plan.root_budget.minimum_retained_cycles
        != destination.funding.root_funding.request_threshold.to_u128()
        || plan.sources.iter().any(|source| {
            source.minimum_ready_cycles
                != destination.limits.canister_pool.canister_cycles.to_u128()
        })
    {
        return Err(invalid());
    }
    let mut expected = roots.keys().copied().collect::<BTreeSet<_>>();
    expected.insert(binding.coordinator);
    let mut stores = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for entry in &admission.infrastructure {
        if !seen.insert(entry.principal) || entry.module_sha256 == [0; 32] {
            return Err(invalid());
        }
        let (subnet, owner) = match entry.kind {
            CapacityImportInfrastructureKind::Coordinator
                if entry.principal == binding.coordinator =>
            {
                (binding.coordinator_subnet, authority.operator)
            }
            CapacityImportInfrastructureKind::Root => (
                *roots.get(&entry.principal).ok_or_else(invalid)?,
                authority.operator,
            ),
            CapacityImportInfrastructureKind::Store { root } => {
                if !stores.insert(root) || expected.contains(&entry.principal) {
                    return Err(invalid());
                }
                (*roots.get(&root).ok_or_else(invalid)?, root)
            }
            CapacityImportInfrastructureKind::Coordinator => return Err(invalid()),
        };
        if entry.subnet != subnet
            || !canonical_controllers(&entry.controllers)
            || !entry.controllers.contains(&owner)
            || !authority
                .recovery_controllers
                .iter()
                .all(|controller| entry.controllers.contains(controller))
        {
            return Err(invalid());
        }
    }
    if stores.len() != roots.len()
        || seen.len() != expected.len() + stores.len()
        || !expected.is_subset(&seen)
        || plan
            .sources
            .iter()
            .any(|source| seen.contains(&source.binding.canister_id))
    {
        return Err(invalid());
    }
    Ok(())
}

fn canonical_controllers(controllers: &[Principal]) -> bool {
    !controllers.is_empty()
        && controllers.len() <= 10
        && controllers.windows(2).all(|pair| pair[0] < pair[1])
        && controllers.iter().all(|principal| {
            *principal != Principal::anonymous() && *principal != Principal::management_canister()
        })
}

/// Reject an incomplete reservation envelope before any controller or stop effect.
pub(super) fn validate_budget(
    request: &crate::fleet_ensure::dto::capacity_import::CapacityImportReviewRequest,
    maximum_call_cycles: u128,
) -> Result<(), crate::fleet_ensure::ops::capacity_import::journal::CapacityImportJournalError> {
    use crate::fleet_ensure::ops::capacity_import::journal::CapacityImportJournalError;
    use canic_core::control_plane_support::policy::pool_import;
    let minimum_calls = pool_import::minimum_calls(request.canisters.len())
        .ok_or(CapacityImportJournalError::RequestInvalid)?;
    let required_debit_cycles =
        pool_import::required_debit(maximum_call_cycles, request.maximum_root_paid_calls)
            .ok_or(CapacityImportJournalError::RequestInvalid)?;
    if request.maximum_root_paid_calls < minimum_calls
        || request.maximum_root_debit_cycles < required_debit_cycles
    {
        return Err(CapacityImportJournalError::InsufficientRootBudget {
            paid_calls: request.maximum_root_paid_calls,
            minimum_calls,
            maximum_debit_cycles: request.maximum_root_debit_cycles,
            required_debit_cycles,
        });
    }
    Ok(())
}
