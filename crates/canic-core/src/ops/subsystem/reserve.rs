//! Reserve pool lifecycle helpers.
//!
//! The root canister maintains a pool of empty or decommissioned canisters
//! that can be quickly reassigned when scaling. Reserve entries store the
//! canister’s cycles balance and any metadata required to reconstruct its
//! registry state when reactivated.

pub use crate::model::memory::reserve::CanisterReserveView;

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
    model::memory::reserve::{CanisterReserve, CanisterReserveEntry},
    ops::{
        OPS_RESERVE_CHECK_INTERVAL, OPS_RESERVE_INIT_DELAY,
        config::ConfigOps,
        ic::{
            get_cycles,
            mgmt::{create_canister, uninstall_code},
            timer::{TimerId, TimerOps},
            update_settings,
        },
        prelude::*,
        storage::topology::SubnetCanisterRegistryOps,
        subsystem::SubsystemOpsError,
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
    #[error("missing module hash for reserve entry {pid}")]
    MissingModuleHash { pid: Principal },

    #[error("missing parent for reserve entry {pid}")]
    MissingParent { pid: Principal },

    #[error("missing type for reserve entry {pid}")]
    MissingType { pid: Principal },

    #[error("reserve entry missing for {pid}")]
    ReserveEntryMissing { pid: Principal },
}

impl From<ReserveOpsError> for Error {
    fn from(err: ReserveOpsError) -> Self {
        SubsystemOpsError::from(err).into()
    }
}

///
/// CanisterReserveAdminCommand
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CanisterReserveAdminCommand {
    CreateEmpty,
    Recycle { pid: Principal },
    Import { pid: Principal },
}

///
/// CanisterReserveAdminResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CanisterReserveAdminResponse {
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

/// Operations for managing the reserve pool.
pub struct CanisterReserveOps;

impl CanisterReserveOps {
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
                    Topic::CanisterState,
                    Warn,
                    "cannot read subnet config: {e:?}"
                );
                return 0;
            }
        };

        let min_size: u64 = subnet_cfg.reserve.minimum_size.into();
        let reserve_size = CanisterReserve::len();

        if reserve_size < min_size {
            let missing = (min_size - reserve_size).min(10);

            log!(
                Topic::Cycles,
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

    /// Full export of reserve contents.
    #[must_use]
    pub fn export() -> CanisterReserveView {
        CanisterReserve::export()
    }

    pub async fn admin(
        cmd: CanisterReserveAdminCommand,
    ) -> Result<CanisterReserveAdminResponse, Error> {
        match cmd {
            CanisterReserveAdminCommand::CreateEmpty => {
                let pid = reserve_create_canister().await?;

                Ok(CanisterReserveAdminResponse::Created { pid })
            }
            CanisterReserveAdminCommand::Recycle { pid } => {
                recycle_via_orchestrator(pid).await?;

                Ok(CanisterReserveAdminResponse::Recycled)
            }
            CanisterReserveAdminCommand::Import { pid } => {
                let _ = reserve_import_canister(pid).await?;

                Ok(CanisterReserveAdminResponse::Imported)
            }
        }
    }

    /// Pops the first entry in the reserve.
    #[must_use]
    pub fn pop_first() -> Option<(Principal, CanisterReserveEntry)> {
        CanisterReserve::pop_first()
    }

    /// Returns true if the reserve pool contains the given canister.
    #[must_use]
    pub fn contains(pid: &Principal) -> bool {
        CanisterReserve::contains(pid)
    }

    /// Returns the subnet configuration if reserve management is enabled.
    fn enabled_subnet_config() -> Option<SubnetConfig> {
        match ConfigOps::current_subnet() {
            Ok(cfg) if cfg.reserve.minimum_size > 0 => Some(cfg),
            Ok(_) | Err(_) => None,
        }
    }
}

//
// CREATE
//

/// Creates a new empty canister and adds it to the reserve pool.
pub async fn reserve_create_canister() -> Result<Principal, Error> {
    OpsError::require_root()?;

    let cycles = Cycles::new(RESERVE_CANISTER_CYCLES);
    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self());

    let pid = create_canister(controllers, cycles.clone()).await?;

    CanisterReserve::register(pid, cycles, None, None, None);
    Ok(pid)
}

//
// SHARED INTERNAL HELPER
//

/// Moves a canister into the reserve after uninstalling and resetting controllers.
async fn move_into_reserve(
    pid: Principal,
    removed_role: Option<CanisterRole>,
    parent_pid: Option<Principal>,
    module_hash: Option<Vec<u8>>,
) -> Result<(Option<CanisterRole>, Option<Principal>), Error> {
    uninstall_code(pid).await?;

    // Reset controllers to root-configured set.
    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self());
    update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(controllers),
            ..Default::default()
        },
    })
    .await?;

    let cycles = get_cycles(pid).await?;

    CanisterReserve::register(pid, cycles, removed_role.clone(), parent_pid, module_hash);

    Ok((removed_role, parent_pid))
}

//
// IMPORT
//

/// Imports a canister into the reserve. Used when the canister did not
/// originate from this subnet’s lifecycle management.
pub async fn reserve_import_canister(
    pid: Principal,
) -> Result<(Option<CanisterRole>, Option<Principal>), Error> {
    OpsError::require_root()?;

    let entry = SubnetCanisterRegistryOps::get(pid);
    let parent = entry.as_ref().and_then(|e| e.parent_pid);
    let role = entry.as_ref().map(|e| e.role.clone());
    let hash = entry.as_ref().and_then(|e| e.module_hash.clone());

    let _ = SubnetCanisterRegistryOps::remove(&pid);

    move_into_reserve(pid, role, parent, hash).await
}

//
// RECYCLE
//

/// Recycles a topology canister into the reserve. Used when retiring
/// canisters that were originally managed by this subnet.
pub async fn reserve_recycle_canister(
    pid: Principal,
) -> Result<(Option<CanisterRole>, Option<Principal>), Error> {
    OpsError::require_root()?;

    let entry = SubnetCanisterRegistryOps::get(pid);
    let parent = entry.as_ref().and_then(|e| e.parent_pid);
    let role = entry.as_ref().map(|e| e.role.clone());
    let hash = entry.as_ref().and_then(|e| e.module_hash.clone());

    let _ = SubnetCanisterRegistryOps::remove(&pid);

    move_into_reserve(pid, role, parent, hash).await
}

//
// EXPORT
//

/// Reactivates a reserve canister and restores its registry entry.
pub async fn reserve_export_canister(pid: Principal) -> Result<(CanisterRole, Principal), Error> {
    OpsError::require_root()?;

    let entry = CanisterReserve::take(&pid).ok_or(ReserveOpsError::ReserveEntryMissing { pid })?;

    let role = entry.role.ok_or(ReserveOpsError::MissingType { pid })?;
    let parent = entry.parent.ok_or(ReserveOpsError::MissingParent { pid })?;
    let hash = entry
        .module_hash
        .ok_or(ReserveOpsError::MissingModuleHash { pid })?;

    // Reset controllers before placing back into registry.
    let mut controllers = Config::get().controllers.clone();
    controllers.push(canister_self());
    update_settings(&UpdateSettingsArgs {
        canister_id: pid,
        settings: CanisterSettings {
            controllers: Some(controllers),
            ..Default::default()
        },
    })
    .await?;

    SubnetCanisterRegistryOps::register(pid, &role, parent, hash);
    Ok((role, parent))
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
        CanisterReserveOps::stop();
        Config::reset_for_tests();
        let cfg = ConfigModel::test_default();
        Config::init_from_toml(&toml::to_string(&cfg).unwrap()).unwrap();
        EnvOps::set_subnet_role(SubnetRole::PRIME);

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
        EnvOps::set_subnet_role(SubnetRole::PRIME);

        assert!(CanisterReserveOps::enabled_subnet_config().is_some());
    }
}
