//! Module: fleet_ensure::policy::reinstall::terminal
//!
//! Responsibility: admit a bounded completed-operation retirement against fresh conservation.
//! Does not own: source decoding, storage, transport or effects.
//! Boundary: only completed native funding/install/protocol operations without estate creation qualify.

use crate::fleet_ensure::{
    model::{ActualCycleConservation, EnsureAction, FleetEnsureStateRecord, FleetObservation},
    view::terminal_source::TerminalSourceView,
};
use std::collections::{BTreeMap, BTreeSet};

/// Account for every recorded native payment and reject unexplained Ledger movement, net loss or missing assets.
pub(in crate::fleet_ensure) fn conservation(
    source: &TerminalSourceView,
    observation: &FleetObservation,
    controlled_cycles: u128,
) -> Option<ActualCycleConservation> {
    let mut funding = 0_u128;
    let mut payments = 0_u128;
    for action in &source.actions {
        if let EnsureAction::Fund { amount, ledger, .. } = action {
            if ledger != &source.reviewed_desired.desired().cycles_ledger {
                return None;
            }
            funding = funding.checked_add(*amount)?;
            payments = payments.checked_add(1)?;
        }
    }
    let fees = super::super::cycle_bounds(source.reviewed_desired.desired())
        .ok()?
        .ledger_fee
        .checked_mul(payments)?;
    let debit = funding.checked_add(fees)?;
    let bounds = &source.conservation;
    if bounds.scheduled_transfer_cycles != 0
        || funding != bounds.maximum_new_funding_cycles
        || fees != bounds.maximum_unavoidable_fee_cycles
        || debit != bounds.maximum_operator_debit_cycles
        || source
            .journal
            .initial_operator_cycles
            .checked_sub(observation.operator_cycles)?
            != debit
        || !domains_match(source, observation)
    {
        return None;
    }
    let accounted = source
        .journal
        .initial_controlled_cycles
        .checked_add(funding)?;
    let burn = accounted.saturating_sub(controlled_cycles);
    if burn > bounds.maximum_execution_burn_cycles {
        return None;
    }
    Some(ActualCycleConservation {
        estate_funding_cycles: 0,
        exact_estate_creation_fee_cycles: 0,
        exact_unavoidable_fee_cycles: fees,
        final_controlled_cycles: controlled_cycles,
        observed_net_cycle_debit_cycles: burn,
        observed_starting_cycles: source.journal.initial_controlled_cycles,
        observed_net_cycle_credit_cycles: controlled_cycles.saturating_sub(accounted),
        operator_debit_cycles: debit,
        received_new_funding_cycles: funding,
    })
}

fn domains_match(source: &TerminalSourceView, observation: &FleetObservation) -> bool {
    let domains = &source.conservation.estate_funding_domains;
    if domains.len() != observation.estate_funding_domains.len()
        || domains.len() != source.journal.initial_estate_funding_cycles_by_root.len()
    {
        return false;
    }
    let mut roots = BTreeSet::new();
    domains.iter().all(|domain| {
        let Some(observed) = observation.estate_funding_domains.get(&domain.root) else {
            return false;
        };
        let Some(pool) = &observed.pool else {
            return false;
        };
        let identities: BTreeSet<_> = pool.assets.iter().map(|asset| &asset.principal).collect();
        let initial: BTreeSet<_> = domain.initial_pool_assets.iter().collect();
        let no_creation = domain.required_creation_count == 0
            && domain.maximum_creation_debit_cycles == 0
            && domain.maximum_creation_fee_cycles == 0
            && domain.maximum_funding_cycles == 0
            && domain.pending_creation_count == 0
            && domain.pending_creation.is_none()
            && pool.pending_creation.is_none();
        let authority_matches = domain.root_principal.is_some()
            && domain.cycles_ledger == observed.cycles_ledger
            && domain.root_principal == observed.root_principal;
        let balance_matches = domain.available_cycles.is_some()
            && domain.available_cycles == observed.balance_cycles
            && source
                .journal
                .initial_estate_funding_cycles_by_root
                .get(&domain.root)
                .copied()
                == domain.available_cycles;
        let pool_matches = identities.len() == pool.assets.len()
            && initial.len() == domain.initial_pool_assets.len()
            && identities == initial;
        no_creation
            && roots.insert(&domain.root)
            && authority_matches
            && balance_matches
            && pool_matches
    })
}

/// Compare the complete observed physical estate without reconstructing retained state.
pub(in crate::fleet_ensure) fn inventory_matches(
    state: &FleetEnsureStateRecord,
    entries: &[crate::registry::RegistryEntry],
) -> bool {
    let names: BTreeMap<_, _> = state
        .principals
        .iter()
        .map(|(name, principal)| (principal, name))
        .collect();
    let observed: BTreeSet<_> = entries.iter().map(|entry| &entry.pid).collect();
    let complete = names.len() == state.principals.len()
        && names.len() == state.topology.len()
        && names.len() == entries.len()
        && observed.len() == entries.len();
    if !complete {
        return false;
    }
    entries.iter().all(|entry| {
        let Some(topology) = names
            .get(&entry.pid)
            .and_then(|name| state.topology.get(*name))
        else {
            return false;
        };
        let parent = match &topology.parent {
            Some(name) => match state.principals.get(name) {
                Some(principal) => Some(principal.as_str()),
                None => return false,
            },
            None => None,
        };
        let expected = (
            &topology.module_hash,
            &topology.protocol_binding,
            &topology.role,
            parent,
        );
        let observed = (
            &entry.module_hash,
            &entry.protocol_binding,
            &entry.role,
            entry.parent_pid.as_deref(),
        );
        expected == observed
    })
}
