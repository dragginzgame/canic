//! Module: observatory::ops::comparison
//!
//! Responsibility: bind saved snapshots and project independently qualified interval evidence.
//! Boundary: local files and source observations; no IC requests or transfer completeness claims.

mod source;
#[cfg(test)]
mod tests;

use crate::observatory::{
    ObservatoryError,
    model::cost::{CostInterval, CounterMovement, SignedCycleAmount},
    policy::cost,
    view::*,
};
use std::{collections::BTreeMap, io::Read, path::Path};

/// Read one private report within a fixed byte envelope before deserializing it.
pub fn read_snapshot(
    path: &Path,
    maximum_bytes: usize,
) -> Result<ObservatorySnapshotView, ObservatoryError> {
    if !(1024..=16 * 1024 * 1024).contains(&maximum_bytes) {
        return Err(ObservatoryError::Bound("snapshot bytes"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(maximum_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > maximum_bytes {
        return Err(ObservatoryError::Bound("snapshot bytes"));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

/// Compare exact retained bindings and source windows without contacting a Fleet.
pub fn compare(
    before: &ObservatorySnapshotView,
    after: &ObservatorySnapshotView,
) -> Result<ObservatoryComparisonView, ObservatoryError> {
    source::same_authority(before, after).map_err(ObservatoryError::Comparison)?;
    let before_roles = source::roles(before).map_err(ObservatoryError::Comparison)?;
    let after_roles = source::roles(after).map_err(ObservatoryError::Comparison)?;
    if before_roles.keys().ne(after_roles.keys()) {
        return Err(ObservatoryError::Comparison(
            CostComparisonFailure::BindingChanged,
        ));
    }
    for (id, old) in &before_roles {
        source::same_role(old, after_roles[id]).map_err(ObservatoryError::Comparison)?;
    }
    let roles = before_roles
        .iter()
        .map(|(id, old)| compare_role(old, after_roles[id], &before_roles, &after_roles))
        .collect();
    Ok(ObservatoryComparisonView {
        schema_version: 1,
        environment: after.environment.clone(),
        fleet: after.fleet.clone(),
        recorded_authority: before.authority.clone(),
        before_collected_at_unix_ms: before.collected_at_unix_ms,
        after_collected_at_unix_ms: after.collected_at_unix_ms,
        roles,
    })
}

fn compare_role(
    before: &ObservatoryRoleView,
    after: &ObservatoryRoleView,
    before_roles: &BTreeMap<&str, &ObservatoryRoleView>,
    after_roles: &BTreeMap<&str, &ObservatoryRoleView>,
) -> RoleCostComparisonView {
    let balances = source::balances(before, after);
    let outgoing = source::grants(before, after, None);
    let incoming = before
        .parent_canister_id
        .as_deref()
        .and_then(|id| Some((before_roles.get(id)?, after_roles.get(id)?)))
        .ok_or(CostComparisonFailure::MissingParent)
        .and_then(|(old, new)| source::grants(old, new, Some(&before.canister_id)));
    let adjusted = adjusted(balances, incoming, outgoing);
    RoleCostComparisonView {
        role: after.role.clone(),
        canister_id: after.canister_id.clone(),
        parent_canister_id: after.parent_canister_id.clone(),
        balance_change: result(balances.map(|values| {
            movement(
                values.interval,
                cost::difference(values.closing, values.opening),
            )
        })),
        incoming_grants: result(incoming.map(counter_movement)),
        outgoing_grants: result(outgoing.map(counter_movement)),
        known_grant_adjusted_decrease: result(adjusted),
        limitations: vec![
            CostEvidenceLimitation::TimerRegistrationResetUnobservable,
            CostEvidenceLimitation::TransferCoverageIncomplete,
            CostEvidenceLimitation::UnattributedExecutionMessageStorage,
        ],
    }
}

fn adjusted(
    balances: Result<source::BalanceWindow, CostComparisonFailure>,
    incoming: Result<CounterMovement, CostComparisonFailure>,
    outgoing: Result<CounterMovement, CostComparisonFailure>,
) -> Result<CycleMovementView, CostComparisonFailure> {
    let balances = balances?;
    let incoming = incoming?;
    let outgoing = outgoing?;
    if balances.interval != incoming.interval || balances.interval != outgoing.interval {
        return Err(CostComparisonFailure::TimeWindowsDiffer);
    }
    let amount = cost::grant_adjusted_decrease(
        balances.opening,
        balances.closing,
        incoming.amount,
        outgoing.amount,
    )
    .map_err(source::numeric_failure)?;
    Ok(movement(balances.interval, amount))
}

fn counter_movement(value: CounterMovement) -> CycleMovementView {
    movement(
        value.interval,
        SignedCycleAmount {
            negative: false,
            magnitude: value.amount,
        },
    )
}

fn movement(interval: CostInterval, amount: SignedCycleAmount) -> CycleMovementView {
    CycleMovementView {
        start_ns: interval.start_ns,
        end_ns: interval.end_ns,
        elapsed_ns: interval.end_ns - interval.start_ns,
        cycles: if amount.negative {
            format!("-{}", amount.magnitude)
        } else {
            amount.magnitude.to_string()
        },
    }
}

fn result<T>(value: Result<T, CostComparisonFailure>) -> CostComparisonResult<T> {
    match value {
        Ok(value) => CostComparisonResult::Available { value },
        Err(reason) => CostComparisonResult::Unavailable { reason },
    }
}
