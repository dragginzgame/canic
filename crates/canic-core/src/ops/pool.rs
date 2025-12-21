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
//! - Ready entries have no code installed (reset_into_pool uninstalls before Ready)

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

///
/// PoolOpsError
///

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

///
/// PoolAdminCommand
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminCommand {
    CreateEmpty,
    Recycle { pid: Principal },
    ImportImmediate { pid: Principal },
    ImportQueued { pids: Vec<Principal> },
    RequeueFailed { pids: Option<Vec<Principal>> },
}

///
/// PoolAdminResponse
///

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
    FailedRequeued {
        requeued: u64,
        skipped: u64,
        total: u64,
    },
}

//
// INTERNAL HELPERS
//

fn pool_controllers() -> Vec<Principal> {
    let mut controllers = Config::get().controllers.clone();

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

fn register_or_update_preserving_metadata(
    pid: Principal,
    cycles: Cycles,
    status: CanisterPoolStatus,
    role: Option<CanisterRole>,
    parent: Option<Principal>,
    module_hash: Option<Vec<u8>>,
) {
    if let Some(mut entry) = CanisterPoolStorageOps::get(pid) {
        entry.cycles = cycles;
        entry.status = status;
        entry.role = role.or(entry.role);
        entry.parent = parent.or(entry.parent);
        entry.module_hash = module_hash.or(entry.module_hash);
        let _ = CanisterPoolStorageOps::update(pid, entry);
    } else {
        CanisterPoolStorageOps::register(pid, cycles, status, role, parent, module_hash);
    }
}

///
/// PoolOps
///

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

        let subnet_cfg = ConfigOps::current_subnet();
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
            PoolAdminCommand::RequeueFailed { pids } => {
                let (requeued, skipped, total) = pool_requeue_failed(pids)?;
                Ok(PoolAdminResponse::FailedRequeued {
                    requeued,
                    skipped,
                    total,
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

    register_or_update_preserving_metadata(
        pid,
        Cycles::default(),
        CanisterPoolStatus::PendingReset,
        None,
        None,
        None,
    );
    let _ = SubnetCanisterRegistryOps::remove(&pid);
    match reset_into_pool(pid).await {
        Ok(cycles) => {
            register_or_update_preserving_metadata(
                pid,
                cycles,
                CanisterPoolStatus::Ready,
                None,
                None,
                None,
            );
        }
        Err(err) => {
            log!(
                Topic::CanisterPool,
                Warn,
                "pool import failed for {pid}: {err}"
            );
            register_or_update_preserving_metadata(
                pid,
                Cycles::default(),
                CanisterPoolStatus::Failed {
                    reason: err.to_string(),
                },
                None,
                None,
                None,
            );
        }
    }
    Ok(())
}

fn pool_import_queued_canisters(pids: Vec<Principal>) -> Result<(u64, u64, u64, u64), Error> {
    pool_import_queued_canisters_inner(pids, true)
}

fn pool_import_queued_canisters_inner(
    pids: Vec<Principal>,
    enforce_root: bool,
) -> Result<(u64, u64, u64, u64), Error> {
    if enforce_root {
        OpsError::require_root()?;
    }

    let mut added = 0;
    let mut requeued = 0;
    let mut skipped = 0;

    for pid in &pids {
        if SubnetCanisterRegistryOps::get(*pid).is_some() {
            skipped += 1;
            continue;
        }

        if let Some(entry) = CanisterPoolStorageOps::get(*pid) {
            if entry.status.is_failed() {
                register_or_update_preserving_metadata(
                    *pid,
                    Cycles::default(),
                    CanisterPoolStatus::PendingReset,
                    None,
                    None,
                    None,
                );
                requeued += 1;
            } else {
                skipped += 1;
            }
            continue;
        }

        register_or_update_preserving_metadata(
            *pid,
            Cycles::default(),
            CanisterPoolStatus::PendingReset,
            None,
            None,
            None,
        );
        added += 1;
    }

    maybe_schedule_reset_worker();

    Ok((added, requeued, skipped, pids.len() as u64))
}

#[cfg(not(test))]
fn maybe_schedule_reset_worker() {
    PoolOps::schedule_reset_worker();
}

#[cfg(test)]
fn maybe_schedule_reset_worker() {
    RESET_SCHEDULED.with_borrow_mut(|flag| *flag = true);
}

#[cfg(test)]
thread_local! {
    static RESET_SCHEDULED: RefCell<bool> = const { RefCell::new(false) };
}

#[cfg(test)]
fn take_reset_scheduled() -> bool {
    RESET_SCHEDULED.with_borrow_mut(|flag| {
        let value = *flag;
        *flag = false;
        value
    })
}

fn pool_requeue_failed(pids: Option<Vec<Principal>>) -> Result<(u64, u64, u64), Error> {
    pool_requeue_failed_inner(pids, true)
}

fn pool_requeue_failed_inner(
    pids: Option<Vec<Principal>>,
    enforce_root: bool,
) -> Result<(u64, u64, u64), Error> {
    if enforce_root {
        OpsError::require_root()?;
    }

    let mut requeued = 0;
    let mut skipped = 0;
    let total;

    if let Some(pids) = pids {
        total = pids.len() as u64;
        for pid in pids {
            if let Some(entry) = CanisterPoolStorageOps::get(pid) {
                if entry.status.is_failed() {
                    register_or_update_preserving_metadata(
                        pid,
                        Cycles::default(),
                        CanisterPoolStatus::PendingReset,
                        None,
                        None,
                        None,
                    );
                    requeued += 1;
                } else {
                    skipped += 1;
                }
            } else {
                skipped += 1;
            }
        }
    } else {
        let entries = CanisterPoolStorageOps::export();
        total = entries.len() as u64;
        for (pid, entry) in entries {
            if entry.status.is_failed() {
                register_or_update_preserving_metadata(
                    pid,
                    Cycles::default(),
                    CanisterPoolStatus::PendingReset,
                    None,
                    None,
                    None,
                );
                requeued += 1;
            } else {
                skipped += 1;
            }
        }
    }

    if requeued > 0 {
        maybe_schedule_reset_worker();
    }

    Ok((requeued, skipped, total))
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

//
// TESTS
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::CanisterRole,
        model::memory::{CanisterEntry, pool::CanisterPool, topology::SubnetCanisterRegistry},
    };
    use candid::Principal;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn reset_state() {
        CanisterPool::clear();
        SubnetCanisterRegistry::clear_for_tests();
        let _ = take_reset_scheduled();
    }

    #[test]
    fn import_queued_registers_pending_entries() {
        reset_state();

        let p1 = p(1);
        let p2 = p(2);

        let (added, requeued, skipped, total) =
            pool_import_queued_canisters_inner(vec![p1, p2], false).unwrap();
        assert_eq!(added, 2);
        assert_eq!(requeued, 0);
        assert_eq!(skipped, 0);
        assert_eq!(total, 2);

        let e1 = CanisterPoolStorageOps::get(p1).unwrap();
        let e2 = CanisterPoolStorageOps::get(p2).unwrap();
        assert!(e1.status.is_pending_reset());
        assert!(e2.status.is_pending_reset());
        assert_eq!(e1.cycles, Cycles::default());
        assert_eq!(e2.cycles, Cycles::default());
    }

    #[test]
    fn import_queued_requeues_failed_entries() {
        reset_state();

        let p1 = p(3);
        CanisterPoolStorageOps::register(
            p1,
            Cycles::new(10),
            CanisterPoolStatus::Failed {
                reason: "nope".to_string(),
            },
            None,
            None,
            None,
        );

        let (added, requeued, skipped, total) =
            pool_import_queued_canisters_inner(vec![p1], false).unwrap();
        assert_eq!(added, 0);
        assert_eq!(requeued, 1);
        assert_eq!(skipped, 0);
        assert_eq!(total, 1);
        assert!(take_reset_scheduled());

        let entry = CanisterPoolStorageOps::get(p1).unwrap();
        assert!(entry.status.is_pending_reset());
        assert_eq!(entry.cycles, Cycles::default());
    }

    #[test]
    fn import_queued_skips_ready_entries() {
        reset_state();

        let p1 = p(4);
        CanisterPoolStorageOps::register(
            p1,
            Cycles::new(42),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
        );

        let (added, requeued, skipped, total) =
            pool_import_queued_canisters_inner(vec![p1], false).unwrap();
        assert_eq!(added, 0);
        assert_eq!(requeued, 0);
        assert_eq!(skipped, 1);
        assert_eq!(total, 1);

        let entry = CanisterPoolStorageOps::get(p1).unwrap();
        assert!(entry.status.is_ready());
        assert_eq!(entry.cycles, Cycles::new(42));
    }

    #[test]
    fn import_queued_skips_registry_canisters() {
        reset_state();

        let pid = p(5);
        SubnetCanisterRegistry::insert_for_tests(CanisterEntry {
            pid,
            role: CanisterRole::new("alpha"),
            parent_pid: None,
            module_hash: None,
            created_at: 0,
        });

        let (added, requeued, skipped, total) =
            pool_import_queued_canisters_inner(vec![pid], false).unwrap();
        assert_eq!(added, 0);
        assert_eq!(requeued, 0);
        assert_eq!(skipped, 1);
        assert_eq!(total, 1);
        assert!(CanisterPoolStorageOps::get(pid).is_none());
    }

    #[test]
    fn register_or_update_preserves_metadata() {
        reset_state();

        let pid = p(6);
        let role = CanisterRole::new("alpha");
        let parent = p(9);
        let hash = vec![1, 2, 3];

        CanisterPoolStorageOps::register(
            pid,
            Cycles::new(7),
            CanisterPoolStatus::Failed {
                reason: "oops".to_string(),
            },
            Some(role.clone()),
            Some(parent),
            Some(hash.clone()),
        );

        register_or_update_preserving_metadata(
            pid,
            Cycles::default(),
            CanisterPoolStatus::PendingReset,
            None,
            None,
            None,
        );

        let entry = CanisterPoolStorageOps::get(pid).unwrap();
        assert!(entry.status.is_pending_reset());
        assert_eq!(entry.cycles, Cycles::default());
        assert_eq!(entry.role, Some(role));
        assert_eq!(entry.parent, Some(parent));
        assert_eq!(entry.module_hash, Some(hash));
    }

    #[test]
    fn requeue_failed_scans_pool_and_schedules() {
        reset_state();

        let failed_pid = p(7);
        let ready_pid = p(8);

        CanisterPoolStorageOps::register(
            failed_pid,
            Cycles::new(11),
            CanisterPoolStatus::Failed {
                reason: "bad".to_string(),
            },
            None,
            None,
            None,
        );
        CanisterPoolStorageOps::register(
            ready_pid,
            Cycles::new(22),
            CanisterPoolStatus::Ready,
            None,
            None,
            None,
        );

        let (requeued, skipped, total) = pool_requeue_failed_inner(None, false).unwrap();
        assert_eq!(requeued, 1);
        assert_eq!(skipped, 1);
        assert_eq!(total, 2);
        assert!(take_reset_scheduled());

        let failed_entry = CanisterPoolStorageOps::get(failed_pid).unwrap();
        let ready_entry = CanisterPoolStorageOps::get(ready_pid).unwrap();
        assert!(failed_entry.status.is_pending_reset());
        assert_eq!(failed_entry.cycles, Cycles::default());
        assert!(ready_entry.status.is_ready());
        assert_eq!(ready_entry.cycles, Cycles::new(22));
    }
}
