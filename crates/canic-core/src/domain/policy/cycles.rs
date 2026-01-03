use crate::{cdk::types::Cycles, config::schema::CanisterConfig};

///
/// CycleTracker retention policy.
///

pub const CYCLE_TRACKER_RETENTION_SECS: u64 = 60 * 60 * 24 * 7; // ~7 days

#[must_use]
pub const fn retention_cutoff(now: u64) -> u64 {
    now.saturating_sub(CYCLE_TRACKER_RETENTION_SECS)
}

///
/// TopupPlan
///

#[derive(Clone, Debug)]
pub struct TopupPlan {
    pub amount: Cycles,
}

#[must_use]
pub fn should_topup(cycles: u128, cfg: &CanisterConfig) -> Option<TopupPlan> {
    let topup = cfg.topup.as_ref()?;
    if cycles >= topup.threshold.to_u128() {
        return None;
    }

    Some(TopupPlan {
        amount: topup.amount.clone(),
    })
}
