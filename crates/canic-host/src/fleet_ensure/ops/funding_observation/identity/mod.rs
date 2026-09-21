//! Module: fleet_ensure::ops::funding_observation::identity
//!
//! Responsibility: resolve infrastructure identities from retained operation evidence.
//! Boundary: configured principals remain exact; only applied creation receipts fill missing identities.

use crate::fleet_ensure::{
    model::{
        DesiredFleet, EffectState, EnsureAction, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetEnsureStateRecord, funding_observation::FundingObservationError,
    },
    ops::action_sha256,
};

pub(in crate::fleet_ensure) fn resolved(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<DesiredFleet, FundingObservationError> {
    let mut desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(FundingObservationError::AuthorityMismatch)?
        .desired()
        .clone();
    for configured in &mut desired.canisters {
        if configured.principal.is_some() {
            continue;
        }
        let Some(canister) = plan
            .canisters
            .iter()
            .find(|entry| entry.name == configured.name)
        else {
            continue;
        };
        configured.principal.clone_from(&canister.principal);
        for action in &canister.actions {
            if !matches!(action, EnsureAction::Create { name, .. } if name == &configured.name) {
                continue;
            }
            let hash = action_sha256(action);
            if let Some(effect) = journal
                .effects
                .iter()
                .find(|effect| effect.action_sha256 == hash && effect.state == EffectState::Applied)
            {
                configured.principal.clone_from(&effect.created_principal);
            }
        }
    }
    Ok(desired)
}

/// Non-executable source readers already bind these state bytes and validate infrastructure.
pub(in crate::fleet_ensure) fn resolved_from_state(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
) -> DesiredFleet {
    let mut desired = desired.clone();
    for configured in &mut desired.canisters {
        if configured.principal.is_none() {
            configured.principal = state
                .pending_principals
                .get(&configured.name)
                .or_else(|| state.principals.get(&configured.name))
                .cloned();
        }
    }
    desired
}
