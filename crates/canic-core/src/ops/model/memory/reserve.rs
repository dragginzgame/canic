//! Lifecycle helpers for the shared reserve pool.

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
        config::ConfigOps,
        mgmt::{create_canister, uninstall_canister},
        model::memory::topology::SubnetCanisterRegistryOps,
        model::{OPS_RESERVE_CHECK_INTERVAL, OPS_RESERVE_INIT_DELAY},
        prelude::*,
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

/// Default cycle balance for freshly created reserve canisters.
const RESERVE_CANISTER_CYCLES: u128 = 5 * TC;

pub struct CanisterReserveOps;

impl CanisterReserveOps {
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

    pub fn stop() {
        TIMER.with_borrow_mut(|slot| {
            if let Some(id) = slot.take() {
                clear_timer(id);
            }
        });
    }

    #[must_use]
    pub fn check() -> u64 {
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
                            "✨ reserve created ({}/{missing})",
                            i + 1
                        ),
                        Err(e) => log!(
                            Topic::CanisterReserve,
                            Warn,
                            "⚠️ failed to create reserve: {e:?}"
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

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        CanisterReserve::contains(pid)
    }

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

/// Create a new empty reserve canister.
pub async fn reserve_create_canister() -> Result<Principal, Error> {
    OpsError::require_root()?;

    let cycles = Cycles::new(RESERVE_CANISTER_CYCLES);
    let pid = create_canister(cycles.clone()).await?;

    CanisterReserve::register(pid, cycles, None, None, None);
    Ok(pid)
}

//
// SHARED HELPER — avoids duplication
//

async fn move_into_reserve(
    pid: Principal,
    removed_ty: Option<CanisterRole>,
    parent_pid: Option<Principal>,
    module_hash: Option<Vec<u8>>,
) -> Result<(Option<CanisterRole>, Option<Principal>), Error> {
    uninstall_canister(pid).await?;

    // Reset controllers
    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self());
    mgmt::update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(controllers),
            ..Default::default()
        },
    })
    .await?;

    let cycles = get_cycles(pid).await?;
    CanisterReserve::register(pid, cycles, removed_ty.clone(), parent_pid, module_hash);

    Ok((removed_ty, parent_pid))
}

//
// IMPORT
//

pub async fn reserve_import_canister(
    pid: Principal,
) -> Result<(Option<CanisterRole>, Option<Principal>), Error> {
    OpsError::require_root()?;

    let entry = SubnetCanisterRegistryOps::get(pid);
    let parent_pid = entry.as_ref().and_then(|e| e.parent_pid);
    let removed_ty = entry.as_ref().map(|e| e.ty.clone());
    let module_hash = entry.as_ref().and_then(|e| e.module_hash.clone());

    if SubnetCanisterRegistryOps::remove(&pid).is_some() {
        log!(
            Topic::CanisterReserve,
            Ok,
            "🗑️ reserve_import: removed {pid}"
        );
    }

    move_into_reserve(pid, removed_ty, parent_pid, module_hash).await
}

//
// RECYCLE
//

pub async fn reserve_recycle_canister(
    pid: Principal,
) -> Result<(Option<CanisterRole>, Option<Principal>), Error> {
    OpsError::require_root()?;

    let entry = SubnetCanisterRegistryOps::get(pid);
    let parent_pid = entry.as_ref().and_then(|e| e.parent_pid);
    let removed_ty = entry.as_ref().map(|e| e.ty.clone());
    let module_hash = entry.as_ref().and_then(|e| e.module_hash.clone());

    if SubnetCanisterRegistryOps::remove(&pid).is_some() {
        log!(
            Topic::CanisterReserve,
            Ok,
            "🗑️ reserve_recycle: removed {pid}"
        );
    }

    move_into_reserve(pid, removed_ty, parent_pid, module_hash).await
}

//
// EXPORT
//

pub async fn reserve_export_canister(pid: Principal) -> Result<(CanisterRole, Principal), Error> {
    OpsError::require_root()?;

    let entry = CanisterReserve::take(&pid)
        .ok_or_else(|| Error::custom(format!("reserve_export: missing {pid}")))?;

    let ty = entry.ty.ok_or_else(|| Error::custom("missing type"))?;
    let parent = entry
        .parent
        .ok_or_else(|| Error::custom("missing parent"))?;
    let module_hash = entry
        .module_hash
        .ok_or_else(|| Error::custom("missing module hash"))?;

    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self());
    mgmt::update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(controllers),
            ..Default::default()
        },
    })
    .await?;

    SubnetCanisterRegistryOps::register(pid, &ty, parent, module_hash);

    Ok((ty, parent))
}

//
// ORCHESTRATOR
//

pub async fn recycle_via_orchestrator(pid: Principal) -> Result<(), Error> {
    use crate::ops::orchestration::root_orchestrator::{
        CanisterLifecycleOrchestrator, LifecycleEvent,
    };

    CanisterLifecycleOrchestrator::apply(LifecycleEvent::RecycleToReserve { pid })
        .await
        .map(|_| ())
}
