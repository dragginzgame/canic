//! Reserve pool lifecycle helpers.
//!
//! The root canister maintains a pool of empty or decommissioned canisters
//! that can be quickly reassigned when scaling.
//!
//! INVARIANTS:
//! - Reserve canisters are NOT part of topology
//! - Reserve canisters have NO parent
//! - Root is the sole controller
//! - Importing a canister is destructive (code + controllers wiped)
//! - Registry metadata is informational only while in reserve

pub use crate::ops::storage::reserve::{CanisterReserveEntry, CanisterReserveView};

use crate::{
    Error, ThisError,
    cdk::{
        api::canister_self,
        futures::spawn,
        mgmt::{CanisterSettings, UpdateSettingsArgs},
        types::Principal,
    },
    config::{Config, schema::SubnetConfig},
    log::Topic,
    ops::{
        OPS_RESERVE_CHECK_INTERVAL, OPS_RESERVE_INIT_DELAY, OpsError,
        config::ConfigOps,
        ic::{
            get_cycles,
            mgmt::{create_canister, uninstall_code},
            timer::{TimerId, TimerOps},
            update_settings,
        },
        prelude::*,
        storage::{reserve::CanisterReserveStorageOps, topology::SubnetCanisterRegistryOps},
    },
    types::{Cycles, TC},
};
use candid::CandidType;
use serde::Deserialize;
use std::cell::RefCell;

///
/// ReserveOpsError
///

#[derive(Debug, ThisError)]
pub enum ReserveOpsError {
    #[error("reserve entry missing for {pid}")]
    ReserveEntryMissing { pid: Principal },

    #[error("missing module hash for reserve entry {pid}")]
    MissingModuleHash { pid: Principal },

    #[error("missing type for reserve entry {pid}")]
    MissingType { pid: Principal },
}

impl From<ReserveOpsError> for Error {
    fn from(err: ReserveOpsError) -> Self {
        OpsError::from(err).into()
    }
}

//
// ADMIN API
//

///
/// ReserveAdminCommand
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum ReserveAdminCommand {
    CreateEmpty,
    Recycle { pid: Principal },
    Import { pid: Principal },
}

///
/// ReserveAdminResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum ReserveAdminResponse {
    Created { pid: Principal },
    Recycled,
    Imported,
}

//
// TIMER
//

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

/// Default cycles allocated to freshly created reserve canisters.
const RESERVE_CANISTER_CYCLES: u128 = 5 * TC;

//
// INTERNAL HELPERS
//

/// Controller set for all reserve canisters (root-only).
fn reserve_controllers() -> Vec<Principal> {
    let mut controllers = Config::get().controllers.clone();
    let root = canister_self();

    if !controllers.contains(&root) {
        controllers.push(root);
    }

    controllers
}

/// Reset a canister into a clean reserve state.
async fn reset_into_reserve(pid: Principal) -> Result<Cycles, Error> {
    uninstall_code(pid).await?;

    update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(reserve_controllers()),
            ..Default::default()
        },
    })
    .await?;

    get_cycles(pid).await
}

///
/// ReserveOps
///

pub struct ReserveOps;

impl ReserveOps {
    /// Starts periodic reserve maintenance. Safe to call multiple times.
    pub fn start() {
        TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let Some(_cfg) = Self::enabled_subnet_config() else {
                return;
            };

            let id = TimerOps::set(OPS_RESERVE_INIT_DELAY, "reserve:init", async {
                let _ = Self::check();

                let interval_id = TimerOps::set_interval(
                    OPS_RESERVE_CHECK_INTERVAL,
                    "reserve:interval",
                    || async {
                        let _ = Self::check();
                    },
                );

                TIMER.with_borrow_mut(|slot| *slot = Some(interval_id));
            });

            *slot = Some(id);
        });
    }

    /// Stops reserve maintenance callbacks.
    pub fn stop() {
        TIMER.with_borrow_mut(|slot| {
            if let Some(id) = slot.take() {
                TimerOps::clear(id);
            }
        });
    }

    /// Ensures the reserve contains the minimum required entries.
    #[must_use]
    pub fn check() -> u64 {
        let subnet_cfg = match ConfigOps::current_subnet() {
            Ok(cfg) => cfg,
            Err(e) => {
                log!(
                    Topic::CanisterReserve,
                    Warn,
                    "cannot read subnet config: {e:?}"
                );
                return 0;
            }
        };

        let min_size: u64 = subnet_cfg.reserve.minimum_size.into();
        let reserve_size = CanisterReserveStorageOps::len();

        if reserve_size < min_size {
            let missing = (min_size - reserve_size).min(10);

            log!(
                Topic::CanisterReserve,
                Ok,
                "reserve low: {reserve_size}/{min_size}, creating {missing}"
            );

            spawn(async move {
                for i in 0..missing {
                    match reserve_create_canister().await {
                        Ok(_) => log!(
                            Topic::CanisterReserve,
                            Ok,
                            "created reserve canister {}/{}",
                            i + 1,
                            missing
                        ),
                        Err(e) => log!(
                            Topic::CanisterReserve,
                            Warn,
                            "failed reserve creation: {e:?}"
                        ),
                    }
                }
            });

            return missing;
        }

        0
    }

    /// Pops the first entry in the reserve.
    #[must_use]
    pub fn pop_first() -> Option<(Principal, CanisterReserveEntry)> {
        CanisterReserveStorageOps::pop_first()
    }

    /// Returns true if the reserve pool contains the given canister.
    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        CanisterReserveStorageOps::contains(pid)
    }

    /// Full export of reserve contents.
    #[must_use]
    pub fn export() -> CanisterReserveView {
        CanisterReserveStorageOps::export()
    }

    pub async fn admin(cmd: ReserveAdminCommand) -> Result<ReserveAdminResponse, Error> {
        match cmd {
            ReserveAdminCommand::CreateEmpty => {
                let pid = reserve_create_canister().await?;
                Ok(ReserveAdminResponse::Created { pid })
            }
            ReserveAdminCommand::Recycle { pid } => {
                reserve_recycle_canister(pid).await?;
                Ok(ReserveAdminResponse::Recycled)
            }
            ReserveAdminCommand::Import { pid } => {
                reserve_import_canister(pid).await?;
                Ok(ReserveAdminResponse::Imported)
            }
        }
    }

    fn enabled_subnet_config() -> Option<SubnetConfig> {
        match ConfigOps::current_subnet() {
            Ok(cfg) if cfg.reserve.minimum_size > 0 => Some(cfg),
            _ => None,
        }
    }
}

//
// CREATE
//

/// Creates a new empty reserve canister.
pub async fn reserve_create_canister() -> Result<Principal, Error> {
    OpsError::require_root()?;

    let cycles = Cycles::new(RESERVE_CANISTER_CYCLES);
    let pid = create_canister(reserve_controllers(), cycles.clone()).await?;

    CanisterReserveStorageOps::register(pid, cycles, None, None, None);

    Ok(pid)
}

//
// IMPORT / RECYCLE
//

/// Import an arbitrary canister into the reserve (destructive).
pub async fn reserve_import_canister(pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    let _ = SubnetCanisterRegistryOps::remove(&pid);

    let cycles = reset_into_reserve(pid).await?;
    CanisterReserveStorageOps::register(pid, cycles, None, None, None);

    Ok(())
}

/// Recycle a managed topology canister into the reserve.
pub async fn reserve_recycle_canister(pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    let entry =
        SubnetCanisterRegistryOps::get(pid).ok_or(ReserveOpsError::ReserveEntryMissing { pid })?;

    let role = Some(entry.role.clone());
    let hash = entry.module_hash.clone();

    let _ = SubnetCanisterRegistryOps::remove(&pid);

    let cycles = reset_into_reserve(pid).await?;
    CanisterReserveStorageOps::register(pid, cycles, role, None, hash);

    Ok(())
}

//
// EXPORT
//

/// Removes a canister from the reserve and returns its stored metadata.
///
/// NOTE:
/// - This does NOT attach the canister to topology.
/// - This does NOT install code.
/// - The caller is responsible for any reactivation workflow.
pub async fn reserve_export_canister(pid: Principal) -> Result<(CanisterRole, Vec<u8>), Error> {
    OpsError::require_root()?;

    let entry = CanisterReserveStorageOps::take(&pid)
        .ok_or(ReserveOpsError::ReserveEntryMissing { pid })?;

    let role = entry.role.ok_or(ReserveOpsError::MissingType { pid })?;
    let hash = entry
        .module_hash
        .ok_or(ReserveOpsError::MissingModuleHash { pid })?;

    Ok((role, hash))
}

//
// ORCHESTRATION HOOK
//

/// Recycles a canister via the orchestrator. Triggers topology and directory cascades.
pub async fn recycle_via_orchestrator(pid: Principal) -> Result<(), Error> {
    use crate::ops::orchestration::orchestrator::{CanisterLifecycleOrchestrator, LifecycleEvent};

    CanisterLifecycleOrchestrator::apply(LifecycleEvent::RecycleToReserve { pid })
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
        config::Config, config::schema::ConfigModel, ids::SubnetRole, ops::storage::env::EnvOps,
    };

    #[test]
    fn start_skips_when_minimum_size_zero() {
        ReserveOps::stop();
        Config::reset_for_tests();
        let cfg = ConfigModel::test_default();
        Config::init_from_toml(&toml::to_string(&cfg).unwrap()).unwrap();
        EnvOps::set_subnet_role(SubnetRole::PRIME);

        assert!(ReserveOps::enabled_subnet_config().is_none());
    }

    #[test]
    fn start_runs_when_minimum_size_nonzero() {
        ReserveOps::stop();
        let mut cfg = ConfigModel::test_default();
        let subnet = cfg.subnets.entry(SubnetRole::PRIME).or_default();
        subnet.reserve.minimum_size = 1;

        Config::reset_for_tests();
        Config::init_from_toml(&toml::to_string(&cfg).unwrap()).unwrap();
        EnvOps::set_subnet_role(SubnetRole::PRIME);

        assert!(ReserveOps::enabled_subnet_config().is_some());
    }
}
