//! Pool lifecycle helpers.
//!
//! The root canister maintains a pool of empty or decommissioned canisters
//! that can be quickly reassigned when scaling.
//!
//! INVARIANTS:
//! - Pool canisters are NOT part of topology
//! - Pool canisters have NO parent
//! - Root is the sole controller
//! - Importing a canister is destructive (code + controllers wiped)
//! - Registry metadata is informational only while in pool

pub use crate::ops::storage::pool::{CanisterPoolEntry, CanisterPoolStatus, CanisterPoolView};

use crate::{
    Error, ThisError,
    cdk::{
        api::canister_self,
        futures::spawn,
        mgmt::{CanisterSettings, UpdateSettingsArgs},
        types::Principal,
    },
    config::Config,
    log::Topic,
    ops::{
        OPS_POOL_CHECK_INTERVAL, OPS_POOL_INIT_DELAY, OpsError,
        config::ConfigOps,
        ic::{
            get_cycles,
            mgmt::{create_canister, uninstall_code},
            timer::{TimerId, TimerOps},
            update_settings,
        },
        prelude::*,
        storage::{pool::CanisterPoolStorageOps, topology::SubnetCanisterRegistryOps},
    },
    types::{Cycles, TC},
};

use candid::CandidType;
use serde::Deserialize;
use std::{cell::RefCell, time::Duration};

//
// ERRORS
//

#[derive(Debug, ThisError)]
pub enum PoolOpsError {
    #[error("pool entry missing for {pid}")]
    PoolEntryMissing { pid: Principal },

    #[error("missing module hash for pool entry {pid}")]
    MissingModuleHash { pid: Principal },

    #[error("missing type for pool entry {pid}")]
    MissingType { pid: Principal },

    #[error("pool entry {pid} is not ready")]
    PoolEntryNotReady { pid: Principal },
}

impl From<PoolOpsError> for Error {
    fn from(err: PoolOpsError) -> Self {
        OpsError::from(err).into()
    }
}

//
// ADMIN API
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminCommand {
    CreateEmpty,
    Recycle { pid: Principal },
    ImportImmediate { pid: Principal },
    ImportQueued { pids: Vec<Principal> },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminResponse {
    Created {
        pid: Principal,
    },
    Recycled,
    Imported,
    QueuedImported {
        added: u64,
        requeued: u64,
        skipped: u64,
        total: u64,
    },
}

//
// TIMER STATE
//

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
    static RESET_IN_PROGRESS: RefCell<bool> = const { RefCell::new(false) };
    static RESET_RESCHEDULE: RefCell<bool> = const { RefCell::new(false) };
    static RESET_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

/// Default cycles allocated to freshly created pool canisters.
const POOL_CANISTER_CYCLES: u128 = 5 * TC;

/// Default batch size for resetting pending pool entries.
const POOL_RESET_BATCH_SIZE: usize = 10;

//
// INTERNAL HELPERS
//

fn pool_controllers() -> Vec<Principal> {
    let mut controllers = Config::try_get()
        .map(|cfg| cfg.controllers.clone())
        .unwrap_or_default();

    let root = canister_self();
    if !controllers.contains(&root) {
        controllers.push(root);
    }

    controllers
}

async fn reset_into_pool(pid: Principal) -> Result<Cycles, Error> {
    uninstall_code(pid).await?;

    update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(pool_controllers()),
            ..Default::default()
        },
    })
    .await?;

    get_cycles(pid).await
}

//
// POOL OPS
//

pub struct PoolOps;

impl PoolOps {
    // ---------------------------------------------------------------------
    // Lifecycle
    // ---------------------------------------------------------------------

    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let id = TimerOps::set(OPS_POOL_INIT_DELAY, "pool:init", async {
                let _ = Self::check();

                let interval =
                    TimerOps::set_interval(OPS_POOL_CHECK_INTERVAL, "pool:interval", || async {
                        let _ = Self::check();
                    });

                TIMER.with_borrow_mut(|slot| *slot = Some(interval));
            });

            *slot = Some(id);
        });
    }

    pub fn stop() {
        TIMER.with_borrow_mut(|slot| {
            if let Some(id) = slot.take() {
                TimerOps::clear(id);
            }
        });
    }

    // ---------------------------------------------------------------------
    // Public API
    // ---------------------------------------------------------------------

    #[must_use]
    pub fn check() -> u64 {
        Self::schedule_reset_worker();

        let subnet_cfg = match ConfigOps::current_subnet() {
            Ok(cfg) => cfg,
            Err(e) => {
                log!(
                    Topic::CanisterPool,
                    Warn,
                    "cannot read subnet config: {e:?}"
                );
                return 0;
            }
        };

        let min_size: u64 = subnet_cfg.pool.minimum_size.into();
        let ready_size = Self::ready_len();

        if ready_size >= min_size {
            return 0;
        }

        let missing = (min_size - ready_size).min(10);
        log!(
            Topic::CanisterPool,
            Ok,
            "pool low: {ready_size}/{min_size}, creating {missing}"
        );

        spawn(async move {
            for i in 0..missing {
                match pool_create_canister().await {
                    Ok(_) => log!(
                        Topic::CanisterPool,
                        Ok,
                        "created pool canister {}/{}",
                        i + 1,
                        missing
                    ),
                    Err(e) => log!(Topic::CanisterPool, Warn, "pool creation failed: {e:?}"),
                }
            }
        });

        missing
    }

    #[must_use]
    pub fn pop_ready() -> Option<(Principal, CanisterPoolEntry)> {
        CanisterPoolStorageOps::pop_ready()
    }

    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        CanisterPoolStorageOps::contains(pid)
    }

    #[must_use]
    pub fn export() -> CanisterPoolView {
        CanisterPoolStorageOps::export()
    }

    pub async fn admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, Error> {
        match cmd {
            PoolAdminCommand::CreateEmpty => {
                let pid = pool_create_canister().await?;
                Ok(PoolAdminResponse::Created { pid })
            }
            PoolAdminCommand::Recycle { pid } => {
                pool_recycle_canister(pid).await?;
                Ok(PoolAdminResponse::Recycled)
            }
            PoolAdminCommand::ImportImmediate { pid } => {
                pool_import_canister(pid).await?;
                Ok(PoolAdminResponse::Imported)
            }
            PoolAdminCommand::ImportQueued { pids } => {
                let (a, r, s, t) = pool_import_queued_canisters(pids)?;
                Ok(PoolAdminResponse::QueuedImported {
                    added: a,
                    requeued: r,
                    skipped: s,
                    total: t,
                })
            }
        }
    }

    // ---------------------------------------------------------------------
    // Scheduler + worker
    // ---------------------------------------------------------------------

    fn ready_len() -> u64 {
        CanisterPoolStorageOps::export()
            .into_iter()
            .filter(|(_, e)| e.status.is_ready())
            .count() as u64
    }

    fn has_pending_reset() -> bool {
        CanisterPoolStorageOps::export()
            .into_iter()
            .any(|(_, e)| e.status.is_pending_reset())
    }

    fn maybe_reschedule() {
        let reschedule = RESET_RESCHEDULE.with_borrow_mut(|f| {
            let v = *f;
            *f = false;
            v
        });

        if reschedule || Self::has_pending_reset() {
            Self::schedule_reset_worker();
        }
    }

    fn schedule_reset_worker() {
        RESET_TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let id = TimerOps::set(Duration::ZERO, "pool:pending", async {
                RESET_TIMER.with_borrow_mut(|slot| *slot = None);
                let _ = Self::run_reset_worker(POOL_RESET_BATCH_SIZE).await;
            });

            *slot = Some(id);
        });
    }

    async fn run_reset_worker(limit: usize) -> Result<(), Error> {
        if limit == 0 {
            return Ok(());
        }

        let should_run = RESET_IN_PROGRESS.with_borrow_mut(|flag| {
            if *flag {
                RESET_RESCHEDULE.with_borrow_mut(|r| *r = true);
                false
            } else {
                *flag = true;
                true
            }
        });

        if !should_run {
            return Ok(());
        }

        let result = Self::run_reset_batch(limit).await;

        RESET_IN_PROGRESS.with_borrow_mut(|f| *f = false);
        Self::maybe_reschedule();

        result
    }

    async fn run_reset_batch(limit: usize) -> Result<(), Error> {
        let mut pending: Vec<_> = CanisterPoolStorageOps::export()
            .into_iter()
            .filter(|(_, e)| e.status.is_pending_reset())
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        pending.sort_by_key(|(_, e)| e.created_at);

        for (pid, mut entry) in pending.into_iter().take(limit) {
            match reset_into_pool(pid).await {
                Ok(cycles) => {
                    entry.cycles = cycles;
                    entry.status = CanisterPoolStatus::Ready;
                }
                Err(err) => {
                    entry.status = CanisterPoolStatus::Failed {
                        reason: err.to_string(),
                    };
                    log!(
                        Topic::CanisterPool,
                        Warn,
                        "pool reset failed for {pid}: {err}"
                    );
                }
            }

            if !CanisterPoolStorageOps::update(pid, entry) {
                log!(
                    Topic::CanisterPool,
                    Warn,
                    "pool reset update missing for {pid}"
                );
            }
        }

        Ok(())
    }
}

//
// CREATE / IMPORT / RECYCLE / EXPORT
//

pub async fn pool_create_canister() -> Result<Principal, Error> {
    OpsError::require_root()?;

    let cycles = Cycles::new(POOL_CANISTER_CYCLES);
    let pid = create_canister(pool_controllers(), cycles.clone()).await?;

    CanisterPoolStorageOps::register(pid, cycles, CanisterPoolStatus::Ready, None, None, None);
    Ok(pid)
}

pub async fn pool_import_canister(pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    let _ = SubnetCanisterRegistryOps::remove(&pid);
    let cycles = reset_into_pool(pid).await?;

    CanisterPoolStorageOps::register(pid, cycles, CanisterPoolStatus::Ready, None, None, None);
    Ok(())
}

fn pool_import_queued_canisters(pids: Vec<Principal>) -> Result<(u64, u64, u64, u64), Error> {
    OpsError::require_root()?;

    let mut added = 0;
    let mut requeued = 0;
    let mut skipped = 0;

    for pid in &pids {
        if SubnetCanisterRegistryOps::get(*pid).is_some() {
            skipped += 1;
            continue;
        }

        if let Some(mut entry) = CanisterPoolStorageOps::get(*pid) {
            if entry.status.is_failed() {
                entry.status = CanisterPoolStatus::PendingReset;
                entry.cycles = Cycles::default();
                if CanisterPoolStorageOps::update(*pid, entry) {
                    requeued += 1;
                } else {
                    skipped += 1;
                }
            } else {
                skipped += 1;
            }
            continue;
        }

        CanisterPoolStorageOps::register(
            *pid,
            Cycles::default(),
            CanisterPoolStatus::PendingReset,
            None,
            None,
            None,
        );
        added += 1;
    }

    PoolOps::schedule_reset_worker();

    Ok((added, requeued, skipped, pids.len() as u64))
}

pub async fn pool_recycle_canister(pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    let entry =
        SubnetCanisterRegistryOps::get(pid).ok_or(PoolOpsError::PoolEntryMissing { pid })?;

    let role = Some(entry.role.clone());
    let hash = entry.module_hash.clone();

    let _ = SubnetCanisterRegistryOps::remove(&pid);

    let cycles = reset_into_pool(pid).await?;
    CanisterPoolStorageOps::register(pid, cycles, CanisterPoolStatus::Ready, role, None, hash);

    Ok(())
}

pub async fn pool_export_canister(pid: Principal) -> Result<(CanisterRole, Vec<u8>), Error> {
    OpsError::require_root()?;

    let entry = CanisterPoolStorageOps::take(&pid).ok_or(PoolOpsError::PoolEntryMissing { pid })?;

    if !entry.status.is_ready() {
        return Err(PoolOpsError::PoolEntryNotReady { pid }.into());
    }

    let role = entry.role.ok_or(PoolOpsError::MissingType { pid })?;
    let hash = entry
        .module_hash
        .ok_or(PoolOpsError::MissingModuleHash { pid })?;

    Ok((role, hash))
}

//
// ORCHESTRATION HOOK
//

pub async fn recycle_via_orchestrator(pid: Principal) -> Result<(), Error> {
    use crate::ops::orchestration::orchestrator::{CanisterLifecycleOrchestrator, LifecycleEvent};

    CanisterLifecycleOrchestrator::apply(LifecycleEvent::RecycleToPool { pid })
        .await
        .map(|_| ())
}
