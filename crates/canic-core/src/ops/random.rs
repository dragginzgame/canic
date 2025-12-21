//! Randomness seeding helpers.

use crate::{
    config::schema::{RandomnessConfig, RandomnessSource},
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::{mgmt, timer::TimerOps},
    },
};
use canic_utils::rand as rand_utils;
use sha2::{Digest, Sha256};
use std::time::Duration;

///
/// RandomOps
/// Schedules PRNG seeding from the configured source (IC `raw_rand` or time).
///

pub struct RandomOps;

impl RandomOps {
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
        Self::schedule_seeding(Duration::ZERO, interval, cfg.source);
    }

    async fn seed_once(source: RandomnessSource) {
        match source {
            RandomnessSource::Ic => match mgmt::raw_rand().await {
                Ok(seed) => rand_utils::seed_from(seed),
                Err(err) => {
                    crate::log!(Topic::Init, Warn, "raw_rand reseed failed: {err}");
                }
            },
            RandomnessSource::Time => Self::seed_from_time(),
        }
    }

    fn schedule_seeding(delay: Duration, interval: Duration, source: RandomnessSource) {
        let _ = TimerOps::set(delay, "random:seed", async move {
            Self::seed_once(source).await;
            Self::schedule_seeding(interval, interval, source);
        });
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
