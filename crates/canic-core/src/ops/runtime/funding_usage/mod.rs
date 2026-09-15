//! Module: ops::runtime::funding_usage
//!
//! Responsibility: project child grant ledgers and existing replay/cost reservations.
//! Boundary: controller observations grant no funding authority and create no state.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    cdk::types::{Cycles, Principal},
    dto::observability::ChildFundingUsage,
    model::{
        cycles_funding::CHILD_FUNDING_COMMAND_KIND,
        replay::{CommandKind, ExternalEffectDescriptor, ReplayActor},
    },
    ops::{
        cost_guard::CostGuardOps, ic::IcOps, runtime::cycles_funding::CyclesFundingLedgerOps,
        storage::replay::ReplayReceiptOps,
    },
    storage::stable::replay::ReplayReceiptRecord,
};

/// Report charged ledger usage separately from unresolved transfer reservations.
pub fn child(child: Principal) -> Result<ChildFundingUsage, InternalError> {
    if child == Principal::anonymous() || child == Principal::management_canister() {
        return Err(InternalError::invalid_input());
    }
    snapshot(IcOps::canister_self(), child, IcOps::now_nanos())
}

fn snapshot(
    parent: Principal,
    child: Principal,
    now_ns: u64,
) -> Result<ChildFundingUsage, InternalError> {
    let command =
        CommandKind::new(CHILD_FUNDING_COMMAND_KIND).map_err(|_| InternalError::invariant())?;
    let pending = ReplayReceiptOps::pending_for_actor_command(
        ReplayActor::direct_caller(child),
        &command,
        now_ns,
    );
    let reserved_cycles = pending
        .iter()
        .try_fold(0_u128, |total, receipt| {
            total.checked_add(reservation(receipt, parent, child, now_ns / 1_000_000_000)?)
        })
        .map(Cycles::new);
    let ledger = CyclesFundingLedgerOps::snapshot(child);
    Ok(ChildFundingUsage {
        parent,
        child,
        observed_at_ns: now_ns,
        accounted_cycles: Cycles::new(ledger.granted_total),
        last_accounted_at_secs: ledger.last_granted_at,
        pending_operations: u32::try_from(pending.len()).map_err(|_| InternalError::invariant())?,
        reserved_cycles,
    })
}

fn reservation(
    receipt: &ReplayReceiptRecord,
    parent: Principal,
    child: Principal,
    now_secs: u64,
) -> Option<u128> {
    if receipt.schema_version != crate::model::replay::REPLAY_RECEIPT_SCHEMA_VERSION {
        return None;
    }
    let ExternalEffectDescriptor::ManagementCall { canister, method } = receipt.effect.as_ref()?
    else {
        return None;
    };
    if *canister != child || method != "deposit_cycles" {
        return None;
    }
    let settlement = receipt.cost_guard_settlement.as_ref()?;
    CostGuardOps::observe_transfer_reservation(settlement, parent, now_secs)
}
