//! Module: fleet_ensure::ops::effect_evidence
//!
//! Responsibility: retain observed balances and completion evidence in effect records.
//! Does not own: observation, authority validation, journal publication, or retry decisions.
//! Boundary: absent balance observations preserve the last retained evidence.

use crate::fleet_ensure::{
    model::{EffectRecord, EffectState},
    ops::EffectObservation,
};

pub(in crate::fleet_ensure) fn retain_cycles(
    record: &mut EffectRecord,
    source: Option<u128>,
    destination: Option<u128>,
) {
    record.post_cycles = source.or(record.post_cycles);
    record.destination_post_cycles = destination.or(record.destination_post_cycles);
}

pub(in crate::fleet_ensure) fn retain_observation(
    record: &mut EffectRecord,
    observed: EffectObservation,
    source: Option<u128>,
    destination: Option<u128>,
) {
    retain_cycles(record, source, destination);
    record.progress_identity = Some(observed.progress_identity);
    if observed.applied {
        record.state = EffectState::Applied;
    }
}
