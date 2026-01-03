//! Randomness seeding scheduler.

use crate::{
    Error,
    config::schema::{RandomnessConfig, RandomnessSource},
    domain::policy,
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::mgmt::raw_rand,
        runtime::timer::{TimerId, TimerOps},
    },
};
use canic_utils::rand as rand_utils;
use sha2::{Digest, Sha256};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static SEED_TIMER: RefCell<Option<TimerId>> =
        const { RefCell::new(None) };
}

/// Start the periodic randomness seeding timers.
///
/// Preconditions:
/// - Called during canister initialization / startup
/// - Authority is enforced by the caller if required
pub fn start() {
    let cfg = match randomness_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            crate::log!(Topic::Init, Warn, "randomness config unavailable: {err}");
            return;
        }
    };
    let Some(interval) = policy::randomness::schedule(&cfg) else {
        if cfg.enabled {
            crate::log!(
                Topic::Init,
                Warn,
                "randomness reseed_interval_secs is 0; seeding disabled"
            );
        } else {
            crate::log!(Topic::Init, Info, "randomness seeding disabled by config");
        }
        return;
    };
    let source = cfg.source;

    let _ = TimerOps::set_guarded_interval(
        &SEED_TIMER,
        Duration::ZERO,
        "random:seed:init",
        move || async move {
            seed_once(source).await;
        },
        interval,
        "random:seed:interval",
        move || async move {
            seed_once(source).await;
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
        RandomnessSource::Time => seed_from_time(),
    }
}

fn randomness_config() -> Result<RandomnessConfig, Error> {
    Ok(ConfigOps::current_canister()?.randomness)
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
