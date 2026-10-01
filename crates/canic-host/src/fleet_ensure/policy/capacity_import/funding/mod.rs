//! Validate additional funding without replacing the original capacity debit equation.
//!
//! Record construction and verification of the retained observation owner belong to ops.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::capacity_import::{
        CapacityImportPlanRecord,
        funding::{CapacityImportFundingCreditRecord, CapacityImportFundingOrigin},
    },
    policy::capacity_import::{CapacityImportPolicyError, custody, require_headroom, source_total},
};
use candid::Principal;
use std::collections::BTreeSet;

/// Every credit names exactly one reviewed source or its destination Root.
pub(in crate::fleet_ensure) fn validate(
    plan: &CapacityImportPlanRecord,
) -> Result<(), CapacityImportPolicyError> {
    let mut seen = BTreeSet::new();
    for credit in &plan.funding_credits {
        let canister = credit.before.binding.canister_id;
        if !seen.insert(canister) {
            return Err(CapacityImportPolicyError::InvalidSources);
        }
        let (cycles, reserved, minimum, maximum_debit) = if canister == plan.authority.root {
            let budget = &plan.root_budget;
            (
                budget.observed_cycles,
                budget.observed_reserved_cycles,
                budget.minimum_retained_cycles,
                budget.maximum_debit_cycles,
            )
        } else {
            let source = plan
                .sources
                .iter()
                .find(|source| source.binding.canister_id == canister)
                .ok_or(CapacityImportPolicyError::InvalidSources)?;
            if source.binding != credit.observed.binding {
                return Err(CapacityImportPolicyError::SourceChanged { canister });
            }
            (
                source.observed_cycles,
                source.observed_reserved_cycles,
                source.minimum_ready_cycles,
                source.maximum_debit_cycles,
            )
        };
        if cycles != credited_cycles(credit)? || reserved != credit.before.reserved_cycles {
            return Err(CapacityImportPolicyError::ConservationUnproven { canister });
        }
        validate_observation(credit, plan.authority.root, maximum_debit, minimum)?;
    }
    Ok(())
}

/// Original native balance plus the exact reviewed credit; reserve remains non-liquid.
pub(in crate::fleet_ensure) fn credited_cycles(
    credit: &CapacityImportFundingCreditRecord,
) -> Result<u128, CapacityImportPolicyError> {
    if credit.credited_cycles == 0 {
        return Err(CapacityImportPolicyError::InvalidCycleBounds);
    }
    credit
        .before
        .cycles
        .checked_add(credit.credited_cycles)
        .ok_or(CapacityImportPolicyError::InvalidCycleBounds)
}

/// The credit cannot hide earlier consumption or admit a different physical source.
pub(in crate::fleet_ensure) fn validate_observation(
    credit: &CapacityImportFundingCreditRecord,
    root: Principal,
    maximum_debit: u128,
    minimum: u128,
) -> Result<(), CapacityImportPolicyError> {
    let before = &credit.before;
    let observed = &credit.observed;
    let canister = before.binding.canister_id;
    let running = !before.binding.stopped
        && !matches!(
            credit.origin,
            CapacityImportFundingOrigin::BootstrapSource { .. }
        )
        && (canister == root || before.binding.controllers.contains(&root));
    let version_matches = observed.binding.canister_version == before.binding.canister_version
        || (running && observed.binding.canister_version >= before.binding.canister_version);
    if custody(&before.binding) != custody(&observed.binding) || !version_matches {
        return Err(CapacityImportPolicyError::SourceChanged { canister });
    }
    let initial = source_total(credited_cycles(credit)?, before.reserved_cycles)?;
    let current = source_total(observed.cycles, observed.reserved_cycles)?;
    let debit = initial
        .checked_sub(current)
        .filter(|debit| *debit <= maximum_debit)
        .ok_or(CapacityImportPolicyError::ConservationUnproven { canister })?;
    require_headroom(canister, observed.cycles, minimum, maximum_debit - debit)
}
