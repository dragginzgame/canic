//! Terminal cycle conservation for a completed-source reset, including reserved and Ledger domains.

use crate::fleet_ensure::{
    model::{ActualCycleConservation, completed_handoff::CompletedEstateResetRecord},
    view::completed_reset::CompletedResetBalancesView,
};
use std::collections::BTreeSet;

/// Both the current operation and the original source keep their own debit bounds.
pub(in crate::fleet_ensure) fn conserved(
    source: &CompletedEstateResetRecord,
    balances: &CompletedResetBalancesView,
    actual: &ActualCycleConservation,
    maximum_burn: u128,
    operator_source: u128,
) -> bool {
    check(source, balances, actual, maximum_burn, operator_source) == Some(true)
}

fn check(
    source: &CompletedEstateResetRecord,
    balances: &CompletedResetBalancesView,
    actual: &ActualCycleConservation,
    maximum_burn: u128,
    operator_source: u128,
) -> Option<bool> {
    let principals = source
        .preparation
        .custody
        .canisters
        .values()
        .map(|c| c.binding.principal.to_text())
        .collect::<BTreeSet<_>>();
    if principals != balances.canisters.keys().cloned().collect()
        || principals != balances.ledger.keys().cloned().collect()
        || source
            .other_ledger_cycles
            .iter()
            .any(|(owner, cycles)| balances.ledger.get(owner) != Some(cycles))
        || operator_source < source.operator_cycles
        || operator_source.checked_sub(balances.operator_cycles)? != actual.operator_debit_cycles
    {
        return Some(false);
    }
    let native = balances
        .canisters
        .values()
        .try_fold(0_u128, |total, balance| {
            total.checked_add(balance.native_cycles)
        })?;
    let controlled = source
        .root_ledger_cycles
        .keys()
        .try_fold(native, |total, root| {
            let name = source.source_names.get(root)?;
            let principal = source
                .preparation
                .custody
                .canisters
                .get(name)?
                .binding
                .principal
                .to_text();
            total.checked_add(*balances.ledger.get(&principal)?)
        })?;
    if controlled != actual.final_controlled_cycles {
        return Some(false);
    }
    let reserved_before = source
        .prepared
        .inspections
        .values()
        .try_fold(0_u128, |total, record| {
            total.checked_add(record.balance?.reserved_cycles)
        })?;
    let reserved_after = balances
        .canisters
        .values()
        .try_fold(0_u128, |total, record| {
            total.checked_add(record.reserved_cycles)
        })?;
    let available = actual
        .observed_starting_cycles
        .checked_add(reserved_before)?
        .checked_add(actual.received_new_funding_cycles)?
        .checked_sub(actual.exact_estate_creation_fee_cycles)?;
    let final_cycles = actual.final_controlled_cycles.checked_add(reserved_after)?;
    if available.saturating_sub(final_cycles) > maximum_burn {
        return Some(false);
    }
    let original = &source.preparation.source_accounting;
    let root_accounts = source
        .root_ledger_cycles
        .values()
        .try_fold(0_u128, |total, cycles| total.checked_add(*cycles))?;
    let original_available = original
        .initial_native_cycles
        .checked_add(original.recorded_funding_cycles)?
        .checked_add(root_accounts)?
        .checked_add(actual.received_new_funding_cycles)?
        .checked_sub(actual.exact_estate_creation_fee_cycles)?;
    let original_bound = original
        .maximum_source_burn_cycles
        .checked_add(source.preparation.maximum_execution_burn_cycles)?
        .checked_add(maximum_burn)?;
    Some(original_available.saturating_sub(actual.final_controlled_cycles) <= original_bound)
}
