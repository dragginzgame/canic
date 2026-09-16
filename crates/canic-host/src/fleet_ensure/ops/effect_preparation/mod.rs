//! Module: fleet_ensure::ops::effect_preparation
//!
//! Responsibility: capture fresh effect evidence and construct its unpersisted intent.
//! Does not own: journal publication, effect issuance, retry or completion decisions.
//! Boundary: native funding shares one live read only before its first intent is written.

use crate::fleet_ensure::{
    model::{EffectRecord, EffectState, EnsureAction, FleetEnsureStateRecord},
    ops::{EffectObservation, EnsurePlatform, action_sha256},
};

/// Fresh intent and optional first observation, never reconstructed from a retained journal.
pub(in crate::fleet_ensure) struct PreparedEffect {
    pub record: EffectRecord,
    pub observation: Option<EffectObservation>,
}

pub(in crate::fleet_ensure) fn prepare_effect<P: EnsurePlatform>(
    platform: &mut P,
    operation_id: &str,
    action: &EnsureAction,
    state: &FleetEnsureStateRecord,
) -> Result<PreparedEffect, P::Error> {
    let native_funding = matches!(action, EnsureAction::Fund { .. });
    let pre_cycles = if native_funding {
        None
    } else {
        platform.action_cycles(action, state)?
    };
    let destination_pre_cycles = platform.action_destination_cycles(action, state)?;
    let pre_canister_version = platform.action_canister_version(action, state)?;
    let mut record = EffectRecord {
        publication_attempts: 0,
        maintenance_attempts: 0,
        action_sha256: action_sha256(action),
        created_principal: None,
        destination_post_cycles: destination_pre_cycles,
        destination_pre_cycles,
        post_cycles: None,
        pre_cycles,
        pre_canister_version,
        progress_identity: None,
        receipt: None,
        state: EffectState::Intent,
    };
    let observation = if native_funding {
        // The funding observer performs the same protected balance/authority read
        // as action_cycles. With no receipt it cannot establish paid completion.
        let observed = platform.observe_effect(operation_id, action, &record, state)?;
        record.pre_cycles = observed.post_cycles;
        Some(observed)
    } else {
        None
    };
    Ok(PreparedEffect {
        record,
        observation,
    })
}
