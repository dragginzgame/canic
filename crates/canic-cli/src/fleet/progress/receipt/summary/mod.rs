//! Module: fleet::progress::receipt::summary
//!
//! Responsibility: summarize existing observation boundaries for human output.
//! Boundary: inclusive child spans are excluded; timings never authorize effects.

use std::io::{self, Write};

use canic_host::fleet_ensure::dto::{FleetObservationStage, FleetObservationTiming};

/// Bounded aggregate of completed outer observations, one entry per stage kind.
#[derive(Default)]
pub(super) struct Summary {
    phases: Vec<Phase>,
}

/// Inclusive cost of completed observations for one outer stage.
struct Phase {
    stage: FleetObservationStage,
    elapsed_millis: u128,
    remote_call_attempts: u64,
    failures: u64,
}

impl Summary {
    pub(super) fn observe(&mut self, timing: &FleetObservationTiming) {
        let Some(succeeded) = timing.succeeded else {
            return;
        };
        if timing.parent_span_id.is_some() || timing.parent_stage.is_some() {
            return;
        }
        let index = self
            .phases
            .iter()
            .position(|phase| phase.stage == timing.stage)
            .unwrap_or_else(|| {
                self.phases.push(Phase {
                    stage: timing.stage,
                    elapsed_millis: 0,
                    remote_call_attempts: 0,
                    failures: 0,
                });
                self.phases.len() - 1
            });
        let phase = &mut self.phases[index];
        phase.elapsed_millis = phase.elapsed_millis.saturating_add(timing.elapsed_millis);
        phase.remote_call_attempts = phase
            .remote_call_attempts
            .saturating_add(timing.remote_call_attempts);
        phase.failures = phase.failures.saturating_add(u64::from(!succeeded));
    }

    pub(super) fn write(&self, output: &mut impl Write) -> io::Result<()> {
        for phase in &self.phases {
            writeln!(
                output,
                "  {:?}: {} ms, {} remote attempts, {} failed observations",
                phase.stage, phase.elapsed_millis, phase.remote_call_attempts, phase.failures,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_and_unfinished_observations_do_not_inflate_phase_cost() {
        let mut summary = Summary::default();
        let mut timing = FleetObservationTiming {
            span_id: 1,
            parent_span_id: None,
            stage: FleetObservationStage::Planning,
            parent_stage: None,
            elapsed_millis: 80,
            remote_call_attempts: 3,
            identity_lookup_attempts: 0,
            identity_lookup_millis: 0,
            cached_read_hits: 0,
            succeeded: None,
        };
        summary.observe(&timing);
        timing.succeeded = Some(true);
        summary.observe(&timing);
        timing.span_id = 2;
        timing.parent_span_id = Some(1);
        summary.observe(&timing);
        timing.parent_span_id = None;
        timing.parent_stage = Some(FleetObservationStage::FleetSnapshot);
        summary.observe(&timing);
        timing.parent_stage = None;
        timing.succeeded = Some(false);
        timing.elapsed_millis = 20;
        timing.remote_call_attempts = 1;
        summary.observe(&timing);
        let [phase] = summary.phases.as_slice() else {
            panic!("one completed outer stage kind");
        };
        assert_eq!(phase.stage, FleetObservationStage::Planning);
        assert_eq!(phase.elapsed_millis, 100);
        assert_eq!(phase.remote_call_attempts, 4);
        assert_eq!(phase.failures, 1);
    }
}
