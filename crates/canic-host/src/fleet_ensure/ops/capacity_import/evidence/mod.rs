//! Validate protected Root progress against immutable host review and cycle baselines.

#[cfg(test)]
pub(in crate::fleet_ensure) mod tests;

use crate::fleet_ensure::{
    model::capacity_import::CapacityImportPlanRecord,
    ops::capacity_import::{CapacityImportReviewError, root_reservation},
};
use canic_core::dto::pool_import::{PoolImportPhase, PoolImportSourceProgress, PoolImportStatus};

/// Validate authenticated Root evidence before retaining or acting on its contents.
/// This does not authenticate a reply or authorize local inventory publication.
pub fn validate_root_status(
    plan: &CapacityImportPlanRecord,
    status: &PoolImportStatus,
) -> Result<(), CapacityImportReviewError> {
    if status.reservation != root_reservation(plan)? || status.progress.len() != plan.sources.len()
    {
        return Err(CapacityImportReviewError::ReservationMismatch);
    }
    let budget = &plan.root_budget;
    if status.paid_calls > budget.maximum_paid_calls
        || status.reserved_debit_cycles > budget.maximum_debit_cycles
        || status.last_root_cycles < budget.minimum_retained_cycles
    {
        return Err(CapacityImportReviewError::RootEvidenceMismatch);
    }
    let mut all_ready = true;
    for (source, progress) in plan.sources.iter().zip(&status.progress) {
        let PoolImportSourceProgress::Ready(receipt) = progress else {
            all_ready = false;
            continue;
        };
        let minimum_before = source
            .binding
            .canister_version
            .checked_add(
                crate::fleet_ensure::policy::capacity_import::controller_version_delta(
                    plan, source,
                ),
            )
            .ok_or(CapacityImportReviewError::RootEvidenceMismatch)?;
        let before_matches = if source.binding.stopped {
            receipt.before_uninstall_canister_version == minimum_before
        } else {
            receipt.before_uninstall_canister_version > minimum_before
        };
        if receipt.canister_id != source.binding.canister_id
            || !before_matches
            || receipt.before_uninstall_canister_version.checked_add(1)
                != Some(receipt.canister_version)
        {
            return Err(CapacityImportReviewError::RootEvidenceMismatch);
        }
        require_conservation(
            Balance {
                native: source.observed_cycles,
                reserved: source.observed_reserved_cycles,
            },
            Balance {
                native: receipt.retained_cycles,
                reserved: receipt.retained_reserved_cycles,
            },
            receipt.observed_debit_cycles,
            source.minimum_ready_cycles,
            source.maximum_debit_cycles,
        )?;
    }
    match status.phase {
        PoolImportPhase::Reserved if !all_ready && status.root_receipt.is_none() => {}
        PoolImportPhase::Ready if all_ready => {}
        PoolImportPhase::Released { publication_sha256 }
            if all_ready && status.root_receipt.is_some() && publication_sha256 != [0; 32] => {}
        _ => return Err(CapacityImportReviewError::RootEvidenceMismatch),
    }
    if let Some(receipt) = &status.root_receipt {
        if status.last_root_cycles != receipt.retained_cycles {
            return Err(CapacityImportReviewError::RootEvidenceMismatch);
        }
        require_conservation(
            Balance {
                native: budget.observed_cycles,
                reserved: budget.observed_reserved_cycles,
            },
            Balance {
                native: receipt.retained_cycles,
                reserved: receipt.retained_reserved_cycles,
            },
            receipt.observed_debit_cycles,
            budget.minimum_retained_cycles,
            budget.maximum_debit_cycles,
        )?;
    }
    Ok(())
}

struct Balance {
    native: u128,
    reserved: u128,
}

fn require_conservation(
    original: Balance,
    retained: Balance,
    debit: u128,
    native_floor: u128,
    maximum_debit: u128,
) -> Result<(), CapacityImportReviewError> {
    let original_total = original
        .native
        .checked_add(original.reserved)
        .ok_or(CapacityImportReviewError::RootEvidenceMismatch)?;
    let retained_total = retained
        .native
        .checked_add(retained.reserved)
        .ok_or(CapacityImportReviewError::RootEvidenceMismatch)?;
    // Receipts bind net debit; a positive difference is observable native surplus,
    // never proof of an operator payment or permission to replenish a call budget.
    if debit != original_total.saturating_sub(retained_total)
        || retained.native < native_floor
        || debit > maximum_debit
    {
        return Err(CapacityImportReviewError::RootEvidenceMismatch);
    }
    Ok(())
}
