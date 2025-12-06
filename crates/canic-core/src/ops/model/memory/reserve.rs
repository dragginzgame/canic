//! Lifecycle helpers for the shared reserve pool.
//!
//! The root canister maintains an inventory of empty canisters that can be
//! handed out quickly when scaling. These helpers create new reserve
//! canisters, top them up with cycles, and reclaim existing canisters into the
//! pool.

pub use crate::model::memory::reserve::CanisterReserveView;

use crate::{
    Error,
    cdk::{
        api::canister_self,
        futures::spawn,
        mgmt::{self, CanisterSettings, UpdateSettingsArgs},
        timers::{TimerId, clear_timer, set_timer, set_timer_interval},
    },
    config::Config,
    interface::ic::get_cycles,
    log::Topic,
    model::memory::reserve::{CanisterReserve, CanisterReserveEntry},
    ops::{
        canister::{create_canister, uninstall_canister},
        config::ConfigOps,
        prelude::*,
    },
    types::{Cycles, Principal, TC},
};
use std::{cell::RefCell, time::Duration};

//
// TIMER
//

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

///
/// Constants
///

/// Wait 30 seconds till we start so the auto-create finishes
const RESERVE_INIT_DELAY: Duration = Duration::new(30, 0);

/// Check every 30 minutes if we need to create more canisters
const RESERVE_CHECK_TIMER: Duration = Duration::from_secs(30 * 60);

/// Default cycle balance for freshly created reserve canisters (5 T cycles).
const RESERVE_CANISTER_CYCLES: u128 = 5 * TC;

///
/// CanisterReserveOps
///

pub struct CanisterReserveOps;

impl CanisterReserveOps {
    /// Start recurring tracking every 30 minutes
    /// Safe to call multiple times: only one loop will run.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let id = set_timer(RESERVE_INIT_DELAY, async {
                let _ = Self::check();

                let interval_id = set_timer_interval(RESERVE_CHECK_TIMER, || async {
                    let _ = Self::check();
                });

                TIMER.with_borrow_mut(|slot| *slot = Some(interval_id));
            });

            *slot = Some(id);
        });
    }

    /// Stop recurring tracking.
    pub fn stop() {
        TIMER.with_borrow_mut(|slot| {
            if let Some(id) = slot.take() {
                clear_timer(id);
            }
        });
    }

    #[must_use]
    pub fn check() -> u64 {
        // try and get the subnet config
        let subnet_cfg = match ConfigOps::current_subnet() {
            Ok(cfg) => cfg,
            Err(e) => {
                log!(
                    Topic::CanisterState,
                    Warn,
                    "⚠️ cannot get current subnet config: {e:?}"
                );
                return 0;
            }
        };

        // success
        let min_size = u64::from(subnet_cfg.reserve.minimum_size);
        let reserve_size = CanisterReserve::len();

        if reserve_size < min_size {
            let missing = (min_size - reserve_size).min(10);
            log!(
                Topic::Cycles,
                Ok,
                "💧 reserve low: {reserve_size}/{min_size}, creating {missing}"
            );

            spawn(async move {
                for i in 0..missing {
                    match reserve_create_canister().await {
                        Ok(_) => log!(
                            Topic::CanisterReserve,
                            Ok,
                            "✨ reserve canister created ({}/{missing})",
                            i + 1
                        ),
                        Err(e) => log!(
                            Topic::CanisterReserve,
                            Warn,
                            "⚠️ failed to create reserve canister: {e:?}"
                        ),
                    }
                }
            });

            return missing;
        }

        0
    }

    #[must_use]
    pub fn export() -> CanisterReserveView {
        CanisterReserve::export()
    }

    #[must_use]
    pub fn pop_first() -> Option<(Principal, CanisterReserveEntry)> {
        CanisterReserve::pop_first()
    }
}

/// Create an empty reserve canister controlled by root.
pub async fn reserve_create_canister() -> Result<Principal, Error> {
    OpsError::require_root()?;

    let cycles = Cycles::new(RESERVE_CANISTER_CYCLES);
    let canister_pid = create_canister(cycles.clone()).await?;

    CanisterReserve::register(canister_pid, cycles);

    Ok(canister_pid)
}

/// Move an existing canister into the reserve pool after uninstalling it.
pub async fn reserve_import_canister(canister_pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    // uninstall
    uninstall_canister(canister_pid).await?;

    // reset controllers to the configured set (+ root) before reuse
    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self());
    let settings = CanisterSettings {
        controllers: Some(controllers),
        ..Default::default()
    };
    mgmt::update_settings(&UpdateSettingsArgs {
        canister_id: canister_pid,
        settings,
    })
    .await?;

    // register to Reserve
    let cycles = get_cycles(canister_pid).await?;

    log!(
        Topic::CanisterReserve,
        Ok,
        "🪶  reserve_import_canister: {canister_pid} ({cycles})",
    );

    CanisterReserve::register(canister_pid, cycles);

    Ok(())
}
