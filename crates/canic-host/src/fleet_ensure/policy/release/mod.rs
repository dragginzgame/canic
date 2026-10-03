//! Pure whole-Fleet release admission; authenticated observations belong to ops.
//!
//! Passing review checks neither quiesces a producer nor authorizes an IC effect.

#[cfg(test)]
pub(in crate::fleet_ensure) mod tests;

pub(in crate::fleet_ensure) mod funding;
pub(in crate::fleet_ensure) mod pool;
pub(in crate::fleet_ensure) mod provisioning;
pub(in crate::fleet_ensure) mod receipts;
pub(in crate::fleet_ensure) mod snapshots;

use crate::fleet_ensure::{
    model::{
        MAX_FLEET_ENSURE_CANISTERS,
        capacity_import::survey::CapacityImportSampleRecord,
        release::{FleetReleaseDestination, FleetReleaseReviewRecord, FleetReleaseRole},
    },
    view::release::FleetReleaseObservation,
};
use candid::Principal;
use canic_core::ids::SubnetId;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Admission failures preserve the original owners and all unfinished effects.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum FleetReleaseError {
    #[error("Fleet release authority or reviewed evidence differs")]
    Authority,
    #[error("Fleet release requires the complete distinct physical inventory")]
    Inventory,
    #[error("Fleet release owner {owner} still has active producers or unresolved effects")]
    Unfinished { owner: Principal },
    #[error("Fleet release source {canister} has changed custody or no controlled recovery path")]
    Custody { canister: Principal },
    #[error("Fleet release source {canister} has an invalid destination")]
    Destination { canister: Principal },
    #[error("Fleet release destination {root} lacks capacity for all selected sources")]
    Capacity { root: Principal },
    #[error("Fleet release source {canister} has insufficient call or debit allowance")]
    Budget { canister: Principal },
    #[error(
        "release observation of {canister} needs {requested_calls} calls and {required_debit_cycles} cycles of allowance; {remaining_calls} calls and {remaining_debit_cycles} cycles remain"
    )]
    ObservationBudget {
        canister: Principal,
        requested_calls: u32,
        remaining_calls: u32,
        required_debit_cycles: u128,
        remaining_debit_cycles: u128,
    },
    #[error("Fleet release source {canister} violates retained-cycle conservation")]
    Conservation { canister: Principal },
    #[error("Fleet release external account inventory or recovery authority is incomplete")]
    Accounts,
}

/// Reserve finite paid reads without reclaiming uncertain earlier attempts.
pub(in crate::fleet_ensure) fn reserve_observation_calls(
    source: &crate::fleet_ensure::model::release::FleetReleaseSourceRecord,
    reserved: u32,
    requested: u32,
) -> Result<u32, FleetReleaseError> {
    let invalid = || FleetReleaseError::Budget {
        canister: source.binding.canister_id,
    };
    if source.maximum_call_debit_cycles == 0 {
        return Err(invalid());
    }
    let spent = source
        .maximum_call_debit_cycles
        .checked_mul(u128::from(reserved))
        .ok_or_else(invalid)?;
    let remaining_calls = source
        .maximum_paid_calls
        .checked_sub(reserved)
        .ok_or_else(invalid)?;
    let remaining_debit_cycles = source
        .maximum_debit_cycles
        .checked_sub(spent)
        .ok_or_else(invalid)?;
    let required_debit_cycles = source
        .maximum_call_debit_cycles
        .checked_mul(u128::from(requested))
        .ok_or_else(invalid)?;
    if requested > remaining_calls || required_debit_cycles > remaining_debit_cycles {
        return Err(FleetReleaseError::ObservationBudget {
            canister: source.binding.canister_id,
            requested_calls: requested,
            remaining_calls,
            required_debit_cycles,
            remaining_debit_cycles,
        });
    }
    reserved.checked_add(requested).ok_or_else(invalid)
}

/// Require consistent physical samples under stopped independent custody.
/// Native debit between reads is measured by the caller; reserved cycles are never liquid funding.
pub(in crate::fleet_ensure) fn validate_physical_sample(
    operator: Principal,
    subnet: SubnetId,
    before: &CapacityImportSampleRecord,
    after: &CapacityImportSampleRecord,
    before_snapshots: &[Vec<u8>],
    after_snapshots: &[Vec<u8>],
) -> Result<(), FleetReleaseError> {
    let binding = &after.binding;
    let custody = binding == &before.binding
        && binding.controllers == [operator]
        && binding.subnet == subnet
        && binding.stopped;
    let snapshots = before_snapshots == after_snapshots
        && after_snapshots.iter().all(|id| !id.is_empty())
        && after_snapshots.windows(2).all(|pair| pair[0] < pair[1]);
    if !custody || !snapshots {
        return Err(FleetReleaseError::Custody {
            canister: binding.canister_id,
        });
    }
    Ok(())
}

/// Require complete observed ownership and sufficient bounds before retaining a review.
pub fn validate_review(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<(), FleetReleaseError> {
    let authority = &review.authority;
    let identities_valid = [
        authority.operator,
        authority.coordinator,
        authority.cycles_ledger,
    ]
    .into_iter()
    .all(principal)
        && authority.operator != authority.coordinator;
    let digests_present = ![
        authority.operation_id,
        authority.network_root_key_sha256,
        authority.release_build_sha256,
    ]
    .contains(&[0; 32]);
    if review.schema_version != 1
        || authority != &observed.authority
        || !identities_valid
        || !digests_present
    {
        return Err(FleetReleaseError::Authority);
    }
    let sources = source_ids(review)?;
    if sources.contains(&authority.operator) || sources.contains(&authority.cycles_ledger) {
        return Err(FleetReleaseError::Authority);
    }
    let live = observed
        .sources
        .iter()
        .map(|source| (source.binding.canister_id, source))
        .collect::<BTreeMap<_, _>>();
    if live.len() != observed.sources.len()
        || live.keys().copied().collect::<BTreeSet<_>>() != sources
    {
        return Err(FleetReleaseError::Inventory);
    }
    validate_owners(review, observed)?;
    for source in &review.sources {
        let id = source.binding.canister_id;
        let actual = live[&id];
        let controllers = &source.binding.controllers;
        let controlled = match source.role {
            FleetReleaseRole::Coordinator | FleetReleaseRole::Root => {
                controllers.contains(&authority.operator)
            }
            FleetReleaseRole::Child { root } | FleetReleaseRole::Store { root } => {
                source.binding.stopped
                    && (controllers.contains(&root) || controllers.contains(&authority.operator))
            }
        };
        let binding_valid = principal(source.binding.subnet.into_principal())
            && canonical_controllers(controllers)
            && controlled;
        let snapshots_valid = source.snapshots == actual.snapshots
            && source.snapshots.iter().all(|id| !id.is_empty())
            && source.snapshots.windows(2).all(|pair| pair[0] < pair[1]);
        if source.binding != actual.binding
            || !binding_valid
            || !snapshots_valid
            || source.disposition_sha256 == [0; 32]
        {
            return Err(FleetReleaseError::Custody { canister: id });
        }
        let required = actual
            .maximum_call_debit_cycles
            .checked_mul(u128::from(source.maximum_paid_calls));
        let headroom = source
            .minimum_retained_cycles
            .checked_add(source.maximum_debit_cycles);
        let calls_fit = actual.required_paid_calls > 0
            && actual.maximum_call_debit_cycles > 0
            && source.maximum_call_debit_cycles == actual.maximum_call_debit_cycles
            && source.maximum_paid_calls >= actual.required_paid_calls;
        let cycles_fit = required.is_some_and(|required| required <= source.maximum_debit_cycles)
            && headroom.is_some_and(|required| required <= source.observed_cycles);
        if !calls_fit || !cycles_fit {
            return Err(FleetReleaseError::Budget { canister: id });
        }
        if source.observed_cycles != actual.cycles
            || source.observed_reserved_cycles != actual.reserved_cycles
            || actual.cycles.checked_add(actual.reserved_cycles).is_none()
        {
            return Err(FleetReleaseError::Conservation { canister: id });
        }
    }
    validate_accounts(review, observed)?;
    validate_destinations(review, observed, &sources)
}

fn source_ids(review: &FleetReleaseReviewRecord) -> Result<BTreeSet<Principal>, FleetReleaseError> {
    let ids = review
        .sources
        .iter()
        .map(|source| source.binding.canister_id)
        .collect::<BTreeSet<_>>();
    if ids.is_empty()
        || ids.len() > MAX_FLEET_ENSURE_CANISTERS
        || ids.len() != review.sources.len()
        || !ids.iter().copied().all(principal)
    {
        return Err(FleetReleaseError::Inventory);
    }
    Ok(ids)
}

/// Exact reviewed ownership closure, independent of producer or paid-effect quiescence.
pub(in crate::fleet_ensure) fn expected_ownership(
    review: &FleetReleaseReviewRecord,
) -> Result<BTreeMap<Principal, BTreeSet<Principal>>, FleetReleaseError> {
    source_ids(review)?;
    let mut expected = BTreeMap::<Principal, BTreeSet<Principal>>::new();
    let coordinator = review.authority.coordinator;
    expected.insert(coordinator, BTreeSet::new());
    let coordinators = review
        .sources
        .iter()
        .filter(|source| source.role == FleetReleaseRole::Coordinator)
        .collect::<Vec<_>>();
    if coordinators.len() != 1 || coordinators[0].binding.canister_id != coordinator {
        return Err(FleetReleaseError::Inventory);
    }
    let roots = review
        .sources
        .iter()
        .filter(|source| source.role == FleetReleaseRole::Root)
        .map(|source| (source.binding.canister_id, source.binding.subnet))
        .collect::<BTreeMap<_, _>>();
    if roots.is_empty() {
        return Err(FleetReleaseError::Inventory);
    }
    for root in roots.keys() {
        expected
            .get_mut(&coordinator)
            .expect("Coordinator inserted")
            .insert(*root);
        expected.insert(*root, BTreeSet::new());
    }
    let mut stores = BTreeSet::new();
    for source in &review.sources {
        let root = match source.role {
            FleetReleaseRole::Store { root } => {
                if !stores.insert(root) {
                    return Err(FleetReleaseError::Inventory);
                }
                root
            }
            FleetReleaseRole::Child { root } => root,
            FleetReleaseRole::Coordinator | FleetReleaseRole::Root => continue,
        };
        if roots.get(&root) != Some(&source.binding.subnet) {
            return Err(FleetReleaseError::Inventory);
        }
        expected
            .get_mut(&root)
            .ok_or(FleetReleaseError::Inventory)?
            .insert(source.binding.canister_id);
    }
    if stores != roots.keys().copied().collect() {
        return Err(FleetReleaseError::Inventory);
    }
    Ok(expected)
}

fn validate_owners(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<(), FleetReleaseError> {
    let expected = expected_ownership(review)?;
    if expected.len() != observed.owners.len() {
        return Err(FleetReleaseError::Inventory);
    }
    let mut seen = BTreeSet::new();
    for owner in &observed.owners {
        let children = owner.children.iter().copied().collect::<BTreeSet<_>>();
        if !seen.insert(owner.owner)
            || children.len() != owner.children.len()
            || expected.get(&owner.owner) != Some(&children)
        {
            return Err(FleetReleaseError::Inventory);
        }
        if !owner.producers_quiescent || !owner.unresolved_operations.is_empty() {
            return Err(FleetReleaseError::Unfinished { owner: owner.owner });
        }
    }
    Ok(())
}

fn validate_accounts(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<(), FleetReleaseError> {
    validate_account_inventory(review)?;
    let accounts = review
        .accounts
        .iter()
        .map(|account| (account_key(account), account))
        .collect::<BTreeMap<_, _>>();
    let actual = observed
        .accounts
        .iter()
        .map(|account| (account_key(account), account))
        .collect::<BTreeMap<_, _>>();
    if actual.len() != observed.accounts.len() || actual.len() != accounts.len() {
        return Err(FleetReleaseError::Accounts);
    }
    for (key, expected) in accounts {
        let current = actual.get(&key).ok_or(FleetReleaseError::Accounts)?;
        if current.recovery_artifact_sha256 != expected.recovery_artifact_sha256
            || current.observed_balance != expected.observed_balance
        {
            return Err(FleetReleaseError::Accounts);
        }
    }
    Ok(())
}

/// Bound declared accounts and require recoverable custody before any Ledger reads.
/// This cannot discover undeclared application accounts or qualify recovery artifacts.
pub(in crate::fleet_ensure) fn validate_account_inventory(
    review: &FleetReleaseReviewRecord,
) -> Result<(), FleetReleaseError> {
    if review.accounts.len() > MAX_FLEET_ENSURE_CANISTERS {
        return Err(FleetReleaseError::Accounts);
    }
    let mut accounts = BTreeMap::new();
    for account in &review.accounts {
        let controlled = review.sources.iter().any(|source| {
            source.binding.canister_id == account.owner
                && source.destination == FleetReleaseDestination::OperatorHeld
        });
        let custody_valid =
            principal(account.ledger) && controlled && account.recovery_artifact_sha256 != [0; 32];
        if !custody_valid || accounts.insert(account_key(account), account).is_some() {
            return Err(FleetReleaseError::Accounts);
        }
    }
    // Every Root and Coordinator has a known native Cycles Ledger account.
    // Matching empty lists cannot establish absence of these external balances.
    for source in &review.sources {
        if matches!(
            source.role,
            FleetReleaseRole::Coordinator | FleetReleaseRole::Root
        ) && !accounts.contains_key(&(
            review.authority.cycles_ledger,
            source.binding.canister_id,
            [0; 32],
        )) {
            return Err(FleetReleaseError::Accounts);
        }
    }
    Ok(())
}

fn account_key(
    account: &crate::fleet_ensure::model::release::FleetReleaseAccountRecord,
) -> (Principal, Principal, [u8; 32]) {
    (
        account.ledger,
        account.owner,
        account.subaccount.unwrap_or([0; 32]),
    )
}

fn validate_destinations(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
    sources: &BTreeSet<Principal>,
) -> Result<(), FleetReleaseError> {
    let mut counts = BTreeMap::<Principal, usize>::new();
    let destinations = observed
        .destinations
        .iter()
        .map(|destination| (destination.authority.root, destination))
        .collect::<BTreeMap<_, _>>();
    if destinations.len() != observed.destinations.len() {
        return Err(FleetReleaseError::Inventory);
    }
    for source in &review.sources {
        let FleetReleaseDestination::IndependentPool { root } = source.destination else {
            continue;
        };
        let invalid = || FleetReleaseError::Destination {
            canister: source.binding.canister_id,
        };
        let destination = destinations.get(&root).ok_or_else(invalid)?;
        let expected = DestinationBinding {
            operator: review.authority.operator,
            network: review.authority.network_root_key_sha256,
            subnet: source.binding.subnet,
        };
        let actual = DestinationBinding {
            operator: destination.authority.operator,
            network: destination.authority.network_root_key_sha256,
            subnet: destination.authority.subnet,
        };
        let independent = !sources.contains(&root)
            && !sources.contains(&destination.authority.coordinator)
            && destination.authority.fleet != review.authority.fleet;
        let ready = destination.ready && !destination.draining && !destination.competing_operation;
        let authority_valid = principal(root)
            && principal(destination.authority.coordinator)
            && root != destination.authority.coordinator
            && destination.authority.root_authority_sha256 != [0; 32];
        if expected != actual
            || !independent
            || !ready
            || !authority_valid
            || !destination.assigned_canisters.is_disjoint(sources)
        {
            return Err(invalid());
        }
        *counts.entry(root).or_default() += 1;
    }
    for (root, count) in counts {
        let destination = destinations[&root];
        let available = destination
            .maximum_capacity
            .checked_sub(destination.occupied_capacity);
        if available.is_none_or(|available| u64::from(available) < count as u64) {
            return Err(FleetReleaseError::Capacity { root });
        }
    }
    Ok(())
}

/// Verify the empty operator-held capacity after reset, without querying cleared role state.
/// This boundary precedes destination pool enrollment; it is not its admission receipt.
pub fn validate_held_capacity(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<(), FleetReleaseError> {
    validate_retained_capacity(review, observed, CapacityBoundary::Empty)
}

/// Require every source to be stopped under independent operator custody before wiping owners.
/// Handoff may change versions; reviewed code and snapshots must still match exactly.
pub fn validate_reset_custody(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
) -> Result<(), FleetReleaseError> {
    validate_retained_capacity(review, observed, CapacityBoundary::BeforeReset)
}

enum CapacityBoundary {
    BeforeReset,
    Empty,
}

fn validate_retained_capacity(
    review: &FleetReleaseReviewRecord,
    observed: &FleetReleaseObservation,
    boundary: CapacityBoundary,
) -> Result<(), FleetReleaseError> {
    if observed.authority != review.authority {
        return Err(FleetReleaseError::Authority);
    }
    let ids = source_ids(review)?;
    let live = observed
        .sources
        .iter()
        .map(|source| (source.binding.canister_id, source))
        .collect::<BTreeMap<_, _>>();
    if live.len() != observed.sources.len() || live.keys().copied().collect::<BTreeSet<_>>() != ids
    {
        return Err(FleetReleaseError::Inventory);
    }
    for source in &review.sources {
        let id = source.binding.canister_id;
        let actual = live[&id];
        let held = actual.binding.controllers == [review.authority.operator]
            && actual.binding.subnet == source.binding.subnet;
        let content_matches = match boundary {
            CapacityBoundary::Empty => {
                actual.binding.module_sha256.is_none()
                    && actual.binding.snapshots_size_bytes == 0
                    && actual.snapshots.is_empty()
            }
            CapacityBoundary::BeforeReset => {
                actual.binding.module_sha256 == source.binding.module_sha256
                    && actual.binding.snapshots_size_bytes == source.binding.snapshots_size_bytes
                    && actual.snapshots == source.snapshots
            }
        };
        if !held || !actual.binding.stopped || !content_matches {
            return Err(FleetReleaseError::Custody { canister: id });
        }
        let before = source
            .observed_cycles
            .checked_add(source.observed_reserved_cycles);
        let after = actual.cycles.checked_add(actual.reserved_cycles);
        let debit = before
            .zip(after)
            .map(|(before, after)| before.saturating_sub(after));
        if actual.cycles < source.minimum_retained_cycles
            || debit.is_none_or(|debit| debit > source.maximum_debit_cycles)
        {
            return Err(FleetReleaseError::Conservation { canister: id });
        }
    }
    validate_accounts(review, observed)
}

#[derive(Eq, PartialEq)]
struct DestinationBinding {
    operator: Principal,
    network: [u8; 32],
    subnet: SubnetId,
}

fn principal(id: Principal) -> bool {
    id != Principal::anonymous() && id != Principal::management_canister()
}

fn canonical_controllers(controllers: &[Principal]) -> bool {
    !controllers.is_empty()
        && controllers.len() <= 10
        && controllers.iter().copied().all(principal)
        && controllers.windows(2).all(|pair| pair[0] < pair[1])
}
