//! Observe all retained physical balances under the preparation's bounded intent.
//!
//! Samples support reset planning; running applications still require a terminal conservation pass.

use crate::{
    fleet_ensure::{
        model::{EnsureAction, completed_handoff::preparation::*},
        ops::{
            self, EnsurePaths,
            completed_preparation::{self as preparation, CompletedPreparationError},
        },
    },
    icp::IcpCli,
};
use canic_core::cdk::utils::hash::{hex_bytes, sha256_hex};

pub(super) fn verify_review(
    review: &CompletedPreparationReviewRecord,
) -> Result<(), CompletedPreparationError> {
    if !review
        .inspection_roots
        .keys()
        .eq(review.custody.canisters.keys())
    {
        return Err(CompletedPreparationError::Conflict);
    }
    for (name, root) in &review.inspection_roots {
        let binding = &review.custody.canisters[name].binding;
        if let Some(root) = root {
            let root_binding = review
                .custody
                .canisters
                .get(root)
                .ok_or(CompletedPreparationError::Conflict)?;
            if !review.actions.iter().any(|a| {
                matches!(a, EnsureAction::SealAuthority {
                name, authority_kind: crate::fleet_ensure::model::DesiredCanisterKind::Root, ..
            } if name == root)
            }) || name == root
                || binding.subnet != root_binding.binding.subnet
                || !binding
                    .controllers
                    .contains(&root_binding.binding.principal)
            {
                return Err(CompletedPreparationError::Conflict);
            }
        } else if !binding.controllers.contains(&review.custody.operator) {
            return Err(CompletedPreparationError::Conflict);
        }
    }
    Ok(())
}

pub(super) fn verify_journal(
    review: &CompletedPreparationReviewRecord,
    journal: &CompletedPreparationJournalRecord,
) -> Result<(), CompletedPreparationError> {
    if !review
        .inspection_roots
        .keys()
        .eq(journal.inspections.keys())
    {
        return Err(CompletedPreparationError::Conflict);
    }
    let sealed = journal
        .effects
        .iter()
        .all(|e| e.applied && e.after.is_some());
    for inspection in journal.inspections.values() {
        if inspection.attempts > preparation::ATTEMPTS
            || (inspection.attempts != 0 && !sealed)
            || (inspection.balance.is_some() && inspection.attempts == 0)
        {
            return Err(CompletedPreparationError::Conflict);
        }
        if let Some(balance) = inspection.balance {
            balance
                .native_cycles
                .checked_add(balance.reserved_cycles)
                .ok_or(CompletedPreparationError::Conservation)?;
        }
    }
    if journal.prepared {
        conservation(review, journal)?;
    }
    Ok(())
}

/// Inspect descendants first, then their paying authorities, so sampled Root balances
/// include the inspection debit. Reopening never replaces a successful observation.
pub(in crate::fleet_ensure) fn pending(
    review: &CompletedPreparationReviewRecord,
    journal: &CompletedPreparationJournalRecord,
) -> Vec<String> {
    let mut names = review
        .inspection_roots
        .keys()
        .filter(|name| journal.inspections[*name].balance.is_none())
        .cloned()
        .collect::<Vec<_>>();
    names.sort_by_key(|name| (review.inspection_roots[name].is_none(), name.clone()));
    names
}

pub(in crate::fleet_ensure) fn consume(
    paths: &EnsurePaths,
    journal: &mut CompletedPreparationJournalRecord,
    name: &str,
) -> Result<(), CompletedPreparationError> {
    if journal.prepared
        || journal
            .effects
            .iter()
            .any(|e| !e.applied || e.after.is_none())
    {
        return Err(CompletedPreparationError::Conflict);
    }
    let entry = journal
        .inspections
        .get_mut(name)
        .ok_or(CompletedPreparationError::Conflict)?;
    if entry.balance.is_some() || entry.attempts >= preparation::ATTEMPTS {
        return Err(CompletedPreparationError::Budget);
    }
    entry.attempts += 1;
    preparation::save(paths, journal)
}

pub(in crate::fleet_ensure) fn retain(
    paths: &EnsurePaths,
    journal: &mut CompletedPreparationJournalRecord,
    name: &str,
    balance: CompletedPreparationBalanceRecord,
) -> Result<(), CompletedPreparationError> {
    let entry = journal
        .inspections
        .get_mut(name)
        .ok_or(CompletedPreparationError::Conflict)?;
    if entry.balance.is_some() || entry.attempts == 0 || journal.prepared {
        return Err(CompletedPreparationError::Conflict);
    }
    balance
        .native_cycles
        .checked_add(balance.reserved_cycles)
        .ok_or(CompletedPreparationError::Conservation)?;
    entry.balance = Some(balance);
    preparation::save(paths, journal)
}

pub(in crate::fleet_ensure) fn observe(
    paths: &EnsurePaths,
    icp: &IcpCli,
    review: &CompletedPreparationReviewRecord,
    name: &str,
) -> Result<CompletedPreparationBalanceRecord, CompletedPreparationError> {
    let binding = &review
        .custody
        .canisters
        .get(name)
        .ok_or(CompletedPreparationError::Conflict)?
        .binding;
    let root = review
        .inspection_roots
        .get(name)
        .ok_or(CompletedPreparationError::Conflict)?;
    let status = if let Some(root) = root {
        let action = review
            .actions
            .iter()
            .find(|action| action.name() == root)
            .ok_or(CompletedPreparationError::Conflict)?;
        let EnsureAction::SealAuthority {
            candid,
            candid_sha256,
            ..
        } = action
        else {
            return Err(CompletedPreparationError::Conflict);
        };
        let path = paths.workspace.join(candid);
        let bytes = crate::durable_io::read_regular_bytes(&path, 1024 * 1024)
            .map_err(|_| CompletedPreparationError::Conflict)?;
        if sha256_hex(&bytes) != *candid_sha256 || !preparation::sealed(icp, paths, review, action)?
        {
            return Err(CompletedPreparationError::Conflict);
        }
        let status = ops::current_inventory::inspect_root_controlled_canister(
            icp,
            &path,
            review.custody.canisters[root].binding.principal,
            binding.principal,
        )
        .map_err(Box::new)?;
        preparation::Status {
            status: status.status,
            cycles: status.cycles,
            reserved_cycles: status.reserved_cycles,
            module_hash: status.module_hash,
            settings: preparation::Settings {
                controllers: status.settings.controllers,
            },
        }
    } else {
        icp.management_canister_status_candid(
            binding.principal,
            &canic_core::dto::canister::CanisterInspectionRequest {
                canister_id: binding.principal,
            },
        )
        .map_err(Box::new)?
    };
    let mut controllers = status.settings.controllers;
    controllers.sort_unstable();
    if controllers != binding.controllers
        || status.module_hash.map(hex_bytes) != binding.module_sha256
    {
        return Err(CompletedPreparationError::Conflict);
    }
    Ok(CompletedPreparationBalanceRecord {
        status: preparation::runtime_status(status.status),
        native_cycles: u128::try_from(status.cycles.0)
            .map_err(|_| CompletedPreparationError::Conservation)?,
        reserved_cycles: u128::try_from(status.reserved_cycles.0)
            .map_err(|_| CompletedPreparationError::Conservation)?,
    })
}

/// Preserve the original native baseline. Reserved and other Ledger balances cannot
/// excuse missing original native funds; they remain separately controlled balances.
pub(in crate::fleet_ensure) fn conservation(
    review: &CompletedPreparationReviewRecord,
    journal: &CompletedPreparationJournalRecord,
) -> Result<u128, CompletedPreparationError> {
    let native = journal
        .inspections
        .values()
        .try_fold(0_u128, |total, inspection| {
            total
                .checked_add(
                    inspection
                        .balance
                        .ok_or(CompletedPreparationError::Conflict)?
                        .native_cycles,
                )
                .ok_or(CompletedPreparationError::Conservation)
        })?;
    let accounting = &review.source_accounting;
    let available = accounting
        .initial_native_cycles
        .checked_add(accounting.recorded_funding_cycles)
        .ok_or(CompletedPreparationError::Conservation)?;
    let debit = available
        .checked_sub(native)
        .ok_or(CompletedPreparationError::Conservation)?;
    let ceiling = accounting
        .maximum_source_burn_cycles
        .checked_add(review.maximum_execution_burn_cycles)
        .ok_or(CompletedPreparationError::Conservation)?;
    if debit > ceiling {
        return Err(CompletedPreparationError::Conservation);
    }
    for (action, effect) in review.actions.iter().zip(&journal.effects) {
        let before = effect.before.ok_or(CompletedPreparationError::Conflict)?;
        let after = journal
            .inspections
            .get(action.name())
            .and_then(|inspection| inspection.balance)
            .ok_or(CompletedPreparationError::Conflict)?;
        let observed_debit = before
            .native_cycles
            .checked_add(before.reserved_cycles)
            .and_then(|before| {
                before.checked_sub(after.native_cycles.checked_add(after.reserved_cycles)?)
            })
            .ok_or(CompletedPreparationError::Conservation)?;
        if observed_debit > authority_budget(review, action.name())? {
            return Err(CompletedPreparationError::Conservation);
        }
    }
    Ok(debit)
}

/// The source Root pays for its descendants' status calls, regardless of their balances.
pub(in crate::fleet_ensure) fn require_headroom(
    review: &CompletedPreparationReviewRecord,
    name: &str,
    balance: CompletedPreparationBalanceRecord,
) -> Result<(), CompletedPreparationError> {
    if balance.native_cycles < authority_budget(review, name)? {
        return Err(CompletedPreparationError::Conservation);
    }
    Ok(())
}

fn authority_budget(
    review: &CompletedPreparationReviewRecord,
    name: &str,
) -> Result<u128, CompletedPreparationError> {
    let count = review
        .inspection_roots
        .iter()
        .filter(|(target, root)| root.as_deref().unwrap_or(target.as_str()) == name)
        .count() as u128;
    preparation::INSPECTION_BURN
        .checked_mul(count)
        .and_then(|burn| burn.checked_add(preparation::ACTION_BURN))
        .ok_or(CompletedPreparationError::Conservation)
}
