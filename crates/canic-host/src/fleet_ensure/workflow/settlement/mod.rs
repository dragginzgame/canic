//! Module: fleet_ensure::workflow::settlement
//!
//! Responsibility: derive bounded credits from exact activation-recovery Stop receipts.
//! Does not own: source journals, refund attribution or funding authority.
//! Boundary: other scopes and non-Stop movements cannot grant settlement credit.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{
        EffectRecord, EffectState, EnsureAction, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetEnsurePlanScope, InstallMode,
    },
    ops::action_sha256,
    workflow::{EnsureWorkflowError, ordered_actions},
};
use canic_core::cdk::types::Cycles;

pub(super) fn observed_credit<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<u128, EnsureWorkflowError<E>> {
    let source_bound = plan
        .reinstall
        .as_ref()
        .is_some_and(|intent| intent.activation_reset.is_some());
    if !eligible_scope(plan.scope, source_bound) {
        return Ok(0);
    }
    let bound = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired()
        .maximum_observation_burn_cycles
        .parse::<Cycles>()
        .map_err(|_| EnsureWorkflowError::PlanIntegrity)?
        .to_u128();
    let actions = ordered_actions(plan);
    if actions.len() != journal.effects.len() {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    actions
        .iter()
        .zip(&journal.effects)
        .try_fold(0_u128, |total, (action, effect)| {
            total
                .checked_add(receipt_credit(action, effect, bound)?)
                .ok_or(EnsureWorkflowError::JournalIntegrity)
        })
}

const fn eligible_scope(scope: FleetEnsurePlanScope, source_bound: bool) -> bool {
    source_bound
        && matches!(
            scope,
            FleetEnsurePlanScope::ReinstallPreparation
                | FleetEnsurePlanScope::RootReinstallPrerequisite
        )
}

fn receipt_credit<E: std::error::Error + 'static>(
    action: &EnsureAction,
    effect: &EffectRecord,
    maximum_credit: u128,
) -> Result<u128, EnsureWorkflowError<E>> {
    if effect.state != EffectState::Applied || effect.action_sha256 != action_sha256(action) {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    match action {
        EnsureAction::Start { .. }
        | EnsureAction::Fund { .. }
        | EnsureAction::Install {
            mode: InstallMode::Reinstall,
            ..
        } => Ok(0),
        EnsureAction::Stop { .. } => {
            let before = effect
                .pre_cycles
                .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            let after = effect
                .post_cycles
                .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            let credit = after.saturating_sub(before);
            if credit > maximum_credit {
                return Err(EnsureWorkflowError::Conservation(format!(
                    "activation Stop credit {credit} exceeded reviewed observation bound {maximum_credit}"
                )));
            }
            Ok(credit)
        }
        _ => Err(EnsureWorkflowError::PlanIntegrity),
    }
}
