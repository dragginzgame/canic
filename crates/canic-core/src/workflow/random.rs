//! Randomness seeding helpers.

use crate::{
    cdk::timers::TimerId,
    config::schema::{RandomnessConfig, RandomnessSource},
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::{mgmt::raw_rand, timer::TimerOps},
    },
};
use canic_utils::rand as rand_utils;
use sha2::{Digest, Sha256};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static SEED_TIMER: RefCell<Option<TimerId>> =
        const { RefCell::new(None) };
}

///
/// RandomWorkflow
/// Schedules PRNG seeding from the configured source (IC `raw_rand` or time).
///

pub struct RandomWorkflow;

impl RandomWorkflow {
    /// Start the periodic seeding timers.
    pub fn start() {
        let cfg = Self::randomness_config();
        if !cfg.enabled {
            crate::log!(Topic::Init, Info, "randomness seeding disabled by config");
            return;
        }

        let interval_secs = cfg.reseed_interval_secs;
        if interval_secs == 0 {
            crate::log!(
                Topic::Init,
                Warn,
                "randomness reseed_interval_secs is 0; seeding disabled"
            );
            return;
        }

        let interval = Duration::from_secs(interval_secs);
        let source = cfg.source;
        let _ = TimerOps::set_guarded_interval(
            &SEED_TIMER,
            Duration::ZERO,
            "random:seed:init",
            move || async move {
                Self::seed_once(source).await;
            },
            interval,
            "random:seed:interval",
            move || async move {
                Self::seed_once(source).await;
            },
        );
    }

    async fn seed_once(source: RandomnessSource) {
        match source {
            RandomnessSource::Ic => match raw_rand().await {
                Ok(seed) => rand_utils::seed_from(seed),
                Err(err) => {
                    crate::log!(Topic::Init, Warn, "raw_rand reseed failed: {err}");
                }
            },
            RandomnessSource::Time => Self::seed_from_time(),
        }
    }

    fn randomness_config() -> RandomnessConfig {
        ConfigOps::current_canister().randomness
    }

    fn seed_from_time() {
        let now = crate::cdk::api::time();
        let canister_id = crate::cdk::api::canister_self();

        let mut hasher = Sha256::new();
        hasher.update(now.to_be_bytes());
        hasher.update(canister_id.as_slice());
        let seed: [u8; 32] = hasher.finalize().into();

        rand_utils::seed_from(seed);
    }
}
