//! Randomness seeding scheduler.

use crate::{
    Error,
    config::schema::{RandomnessConfig, RandomnessSource},
    domain::policy,
    ops::{
        config::ConfigOps,
        ic::{IcOps, mgmt::MgmtOps},
        runtime::timer::TimerId,
    },
    workflow::{prelude::*, runtime::timer::TimerWorkflow},
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
///

pub struct RandomWorkflow;

impl RandomWorkflow {
    /// Start the periodic randomness seeding timers.
    ///
    /// Preconditions:
    /// - Called during canister initialization / startup
    /// - Authority is enforced by the caller if required
    pub fn start() {
        let cfg = match Self::randomness_config() {
            Ok(cfg) => cfg,
            Err(err) => {
                crate::log!(Topic::Config, Warn, "randomness config unavailable: {err}");
                return;
            }
        };
        let Some(interval) = policy::randomness::schedule(&cfg) else {
            if cfg.enabled {
                crate::log!(
                    Topic::Config,
                    Warn,
                    "randomness reseed_interval_secs is 0; seeding disabled"
                );
            } else {
                crate::log!(Topic::Config, Info, "randomness seeding disabled by config");
            }
            return;
        };
        let source = cfg.source;

        let _ = TimerWorkflow::set_guarded_interval(
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
            RandomnessSource::Ic => match MgmtOps::raw_rand().await {
                Ok(seed) => rand_utils::seed_from(seed),
                Err(err) => {
                    crate::log!(Topic::Init, Warn, "raw_rand reseed failed: {err}");
                }
            },
            RandomnessSource::Time => Self::seed_from_time(),
        }
    }

    fn randomness_config() -> Result<RandomnessConfig, Error> {
        Ok(ConfigOps::current_canister()?.randomness)
    }

    fn seed_from_time() {
        let now = IcOps::now_nanos();
        let canister_id = IcOps::canister_self();

        let mut hasher = Sha256::new();
        hasher.update(now.to_be_bytes());
        hasher.update(canister_id.as_slice());
        let seed: [u8; 32] = hasher.finalize().into();

        rand_utils::seed_from(seed);
    }
}
