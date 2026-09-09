//! Module: fleet_ensure::policy::recovery
//!
//! Responsibility: bound dependent recovery review and affordable protocol prefixes.
//! Does not own: funding authority, persistence, or platform observations.
//! Boundary: estimates never authorize effects outside the sealed plan.

use super::{
    CycleBounds, EnsurePolicyError, checked_add, expected_plan_sha256, successor_phase_burn,
};
use crate::fleet_ensure::model::{
    DesiredFleet, EstatePoolAssetLifecycle, EstatePoolAssetObservation,
    EstatePoolInventoryObservation, FleetEnsurePlan, FleetObservation, FleetRecoveryReview,
    PoolRecoveryFunding, RecoveryDiscovery,
};

pub(super) fn review(
    observation: &FleetObservation,
    bounds: CycleBounds,
    maximum_successor_actions: u32,
    base: u128,
    available: u128,
) -> Result<FleetRecoveryReview, EnsurePolicyError> {
    let per_step = bounds
        .observation_burn
        .checked_mul(3)
        .and_then(|value| value.checked_add(bounds.update_burn))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "successor burn bound",
        })?;
    let ceiling = per_step
        .checked_mul(u128::from(maximum_successor_actions))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "successor burn bound",
        })?;
    let mut known_pool_funding = Vec::new();
    for domain in observation.estate_funding_domains.values() {
        if let (Some(root), Some(pool)) = (&domain.root_principal, &domain.pool) {
            for asset in &pool.assets {
                if matches!(
                    asset.lifecycle,
                    EstatePoolAssetLifecycle::PendingReset | EstatePoolAssetLifecycle::Failed
                ) && let Some(funding) = pool_funding(pool, asset, root, bounds)?
                {
                    known_pool_funding.push(funding);
                }
            }
        }
    }
    known_pool_funding.sort_by(|a, b| (&a.root, &a.principal).cmp(&(&b.root, &b.principal)));
    Ok(FleetRecoveryReview {
        base_execution_burn_cycles: base,
        continuation_reserve_cycles: ceiling.min(available.saturating_sub(base)),
        whole_continuation_ceiling_cycles: ceiling,
        known_pool_funding,
        discovery: RecoveryDiscovery::PendingCurrentProtocol,
    })
}

pub(super) fn pool_funding(
    pool: &EstatePoolInventoryObservation,
    asset: &EstatePoolAssetObservation,
    root: &str,
    bounds: CycleBounds,
) -> Result<Option<PoolRecoveryFunding>, EnsurePolicyError> {
    let minimum = checked_add(
        pool.readiness_floor_cycles,
        pool.creation_execution_margin_cycles,
        "pool recovery target",
    )?;
    if asset.cycles >= minimum {
        return Ok(None);
    }
    let margin = checked_add(
        bounds.observation_burn,
        bounds.update_burn,
        "pool reconciliation burn",
    )?;
    let deficit = minimum - asset.cycles;
    let amount = checked_add(deficit, margin, "pool reconciliation funding")?;
    Ok(Some(PoolRecoveryFunding {
        principal: asset.principal.clone(),
        root: root.into(),
        amount_cycles: amount,
        ledger_fee_cycles: bounds.ledger_fee,
        funding_deficit_cycles: deficit,
        funding_margin_cycles: margin,
        expected_post_cycles: checked_add(
            asset.cycles,
            amount,
            "pool reconciliation post balance",
        )?,
    }))
}

/// Return the longest affordable immutable prefix without reducing any per-effect bound.
pub(in crate::fleet_ensure) fn affordable_successor(
    desired: &DesiredFleet,
    mut phase: FleetEnsurePlan,
    remaining: u128,
) -> Result<Option<FleetEnsurePlan>, EnsurePolicyError> {
    let original_count = phase.protocol_actions.len();
    while !phase.protocol_actions.is_empty() {
        let burn = successor_phase_burn(desired, &phase)?;
        if burn <= remaining {
            if phase.protocol_actions.len() != original_count {
                phase.conservation.maximum_execution_burn_cycles = burn;
                phase.conservation.expected_post_operation_cycles = phase
                    .conservation
                    .observed_controlled_cycles
                    .checked_sub(burn)
                    .ok_or(EnsurePolicyError::ArithmeticOverflow {
                        field: "affordable successor balance",
                    })?;
            }
            phase.plan_sha256 = expected_plan_sha256(&phase);
            return Ok(Some(phase));
        }
        phase.protocol_actions.pop();
    }
    Ok(None)
}
