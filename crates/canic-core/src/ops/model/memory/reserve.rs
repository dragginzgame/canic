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
    config::{Config, schema::SubnetConfig},
    interface::ic::get_cycles,
    log::Topic,
    model::memory::reserve::{CanisterReserve, CanisterReserveEntry},
    ops::{
        canister::{create_canister, sync_directories_from_registry, uninstall_canister},
        config::ConfigOps,
        model::memory::topology::SubnetCanisterRegistryOps,
        model::{OPS_RESERVE_CHECK_INTERVAL, OPS_RESERVE_INIT_DELAY},
        prelude::*,
        sync::topology::root_cascade_topology_for_pid,
    },
    types::{Cycles, Principal, TC},
};
use std::cell::RefCell;

//
// TIMER
//

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

///
/// Constants
///

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

            let Some(_cfg) = Self::enabled_subnet_config() else {
                return;
            };

            let id = set_timer(OPS_RESERVE_INIT_DELAY, async {
                let _ = Self::check();

                let interval_id = set_timer_interval(OPS_RESERVE_CHECK_INTERVAL, || async {
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

    /// Return Some(subnet config) when reserve management is enabled for this subnet.
    fn enabled_subnet_config() -> Option<SubnetConfig> {
        match ConfigOps::current_subnet() {
            Ok(cfg) if cfg.reserve.minimum_size > 0 => Some(cfg),
            Ok(_) => {
                log!(
                    Topic::CanisterReserve,
                    Info,
                    "reserve timer not started: minimum_size is 0"
                );
                None
            }
            Err(e) => {
                log!(
                    Topic::CanisterState,
                    Warn,
                    "⚠️ reserve timer not started: config unavailable ({e})"
                );
                None
            }
        }
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

    // keep the registry entry around for logging or rollback if a later step fails
    let mut registry_entry = SubnetCanisterRegistryOps::get(canister_pid);
    let parent_pid = registry_entry.as_ref().and_then(|entry| entry.parent_pid);

    // uninstall but keep the canister alive so it can be repurposed
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

    // remove from registry after we know we control it again
    if let Some(entry) = SubnetCanisterRegistryOps::remove(&canister_pid) {
        log!(
            Topic::CanisterReserve,
            Ok,
            "🗑️  reserve_import_canister: removed {} ({}) from registry",
            canister_pid,
            entry.ty
        );
        registry_entry = Some(entry);
    } else if registry_entry.is_some() {
        log!(
            Topic::CanisterReserve,
            Warn,
            "⚠️ reserve_import_canister: {canister_pid} missing from registry during import"
        );
    }

    // cascade topology + directories so children observe the removal
    if let Some(parent_pid) = parent_pid {
        if parent_pid == canister_self() {
            log!(
                Topic::CanisterReserve,
                Info,
                "ℹ️ reserve_import_canister: parent is root for {canister_pid}; skipping topology cascade"
            );
        } else if SubnetCanisterRegistryOps::get(parent_pid).is_some() {
            root_cascade_topology_for_pid(parent_pid).await?;
        } else {
            log!(
                Topic::CanisterReserve,
                Warn,
                "⚠️ reserve_import_canister: parent {parent_pid} missing from registry; skipping targeted topology cascade"
            );
        }
    } else {
        log!(
            Topic::CanisterReserve,
            Info,
            "ℹ️ reserve_import_canister: no parent recorded for {canister_pid}; skipping targeted topology cascade"
        );
    }
    sync_directories_from_registry(registry_entry.as_ref().map(|e| &e.ty)).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config, config::schema::ConfigModel, ids::SubnetRole, ops::model::memory::EnvOps,
    };

    #[test]
    fn start_skips_when_minimum_size_zero() {
        CanisterReserveOps::stop();
        Config::reset_for_tests();
        let cfg = ConfigModel::test_default();
        Config::init_from_toml(&toml::to_string(&cfg).unwrap()).unwrap();
        EnvOps::set_subnet_type(SubnetRole::PRIME);

        assert!(CanisterReserveOps::enabled_subnet_config().is_none());
    }

    #[test]
    fn start_runs_when_minimum_size_nonzero() {
        CanisterReserveOps::stop();
        let mut cfg = ConfigModel::test_default();
        let subnet = cfg.subnets.entry(SubnetRole::PRIME).or_default();
        subnet.reserve.minimum_size = 1;

        Config::reset_for_tests();
        Config::init_from_toml(&toml::to_string(&cfg).unwrap()).unwrap();
        EnvOps::set_subnet_type(SubnetRole::PRIME);

        assert!(CanisterReserveOps::enabled_subnet_config().is_some());
    }
}
