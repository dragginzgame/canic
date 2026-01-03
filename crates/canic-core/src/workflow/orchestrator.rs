use crate::{
    Error, ThisError,
    cdk::types::Principal,
    domain::policy::upgrade::plan_upgrade,
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{
        ic::mgmt::{canister_status, upgrade_canister},
        runtime::wasm::WasmOps,
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{
        WorkflowError,
        cascade::{state::root_cascade_state, topology::root_cascade_topology_for_pid},
        ic::provision::{create_and_install_canister, rebuild_directories_from_registry},
        topology::directory::builder::{RootAppDirectoryBuilder, RootSubnetDirectoryBuilder},
    },
};

///
/// OrchestratorError
///

#[derive(Debug, ThisError)]
pub enum OrchestratorError {
    #[error("parent {0} not found in registry")]
    ParentNotFound(Principal),

    #[error("registry entry missing for {0}")]
    RegistryEntryMissing(Principal),

    #[error("immediate-parent mismatch: canister {pid} expects parent {expected}, got {found:?}")]
    ImmediateParentMismatch {
        pid: Principal,
        expected: Principal,
        found: Option<Principal>,
    },

    #[error("module hash mismatch for {0}")]
    ModuleHashMismatch(Principal),

    #[error("app directory diverged from registry")]
    AppDirectoryDiverged,

    #[error("subnet directory diverged from registry")]
    SubnetDirectoryDiverged,
}

impl From<OrchestratorError> for Error {
    fn from(err: OrchestratorError) -> Self {
        WorkflowError::from(err).into()
    }
}

pub enum LifecycleEvent {
    Create {
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    },
    Upgrade {
        pid: Principal,
    },
}

#[derive(Default)]
pub struct LifecycleResult {
    pub new_canister_pid: Option<Principal>,
}

impl LifecycleResult {
    #[must_use]
    pub const fn created(pid: Principal) -> Self {
        Self {
            new_canister_pid: Some(pid),
        }
    }
}

pub struct CanisterLifecycleOrchestrator;

impl CanisterLifecycleOrchestrator {
    pub(crate) async fn apply(event: LifecycleEvent) -> Result<LifecycleResult, Error> {
        match event {
            // -----------------------------------------------------------------
            // CREATE
            // -----------------------------------------------------------------
            LifecycleEvent::Create {
                role,
                parent,
                extra_arg,
            } => Self::apply_create(role, parent, extra_arg).await,

            // -----------------------------------------------------------------
            // UPGRADE
            // -----------------------------------------------------------------
            LifecycleEvent::Upgrade { pid } => Self::apply_upgrade(pid).await,
        }
    }

    async fn apply_create(
        role: CanisterRole,
        parent: Principal,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<LifecycleResult, Error> {
        assert_parent_exists(parent)?;

        let pid = create_and_install_canister(&role, parent, extra_arg).await?;

        assert_immediate_parent(pid, parent)?;

        cascade_all(Some(&role), Some(pid)).await?;

        Ok(LifecycleResult::created(pid))
    }

    async fn apply_upgrade(pid: Principal) -> Result<LifecycleResult, Error> {
        let entry =
            SubnetRegistryOps::get(pid).ok_or(OrchestratorError::RegistryEntryMissing(pid))?;

        let wasm = WasmOps::try_get(&entry.role)?;
        let target_hash = wasm.module_hash();
        let status = canister_status(pid).await?;
        let plan = plan_upgrade(status.module_hash, target_hash.clone());

        if let Some(parent_pid) = entry.parent_pid {
            assert_parent_exists(parent_pid)?;
            assert_immediate_parent(pid, parent_pid)?;
        }

        if !plan.should_upgrade {
            log!(
                Topic::CanisterLifecycle,
                Info,
                "canister_upgrade: {pid} already running target module"
            );
            SubnetRegistryOps::update_module_hash(pid, target_hash.clone());
            assert_module_hash(pid, target_hash)?;

            return Ok(LifecycleResult::default());
        }

        upgrade_canister(pid, wasm.bytes()).await?;
        SubnetRegistryOps::update_module_hash(pid, target_hash.clone());
        assert_module_hash(pid, target_hash)?;

        Ok(LifecycleResult::default())
    }
}

//
// Cascades
//

async fn cascade_all(
    role_opt: Option<&CanisterRole>,
    topology_target: Option<Principal>,
) -> Result<(), Error> {
    if let Some(target) = topology_target {
        root_cascade_topology_for_pid(target).await?;
    }

    if let Some(role) = role_opt {
        // Ensure newly created/adopted canisters inherit the current app
        // and subnet states
        let snapshot = rebuild_directories_from_registry(Some(role))
            .await?
            .with_app_state()
            .with_subnet_state()
            .build();

        root_cascade_state(&snapshot).await?;
        assert_directories_match_registry()?;
    }

    Ok(())
}

//
// Invariants
//

fn assert_parent_exists(parent_pid: Principal) -> Result<(), OrchestratorError> {
    SubnetRegistryOps::get(parent_pid).ok_or(OrchestratorError::ParentNotFound(parent_pid))?;
    Ok(())
}

fn assert_module_hash(pid: Principal, expected_hash: Vec<u8>) -> Result<(), OrchestratorError> {
    let entry = SubnetRegistryOps::get(pid).ok_or(OrchestratorError::RegistryEntryMissing(pid))?;
    if entry.module_hash == Some(expected_hash) {
        Ok(())
    } else {
        Err(OrchestratorError::ModuleHashMismatch(pid))
    }
}

fn assert_directories_match_registry() -> Result<(), Error> {
    let app_built = RootAppDirectoryBuilder::build_from_registry();
    let app_snapshot = AppDirectoryOps::snapshot();

    if app_built != app_snapshot {
        return Err(OrchestratorError::AppDirectoryDiverged.into());
    }

    let subnet_built = RootSubnetDirectoryBuilder::build_from_registry();
    let subnet_snapshot = SubnetDirectoryOps::snapshot();

    if subnet_built != subnet_snapshot {
        return Err(OrchestratorError::SubnetDirectoryDiverged.into());
    }

    Ok(())
}

fn assert_immediate_parent(
    pid: Principal,
    expected_parent: Principal,
) -> Result<(), OrchestratorError> {
    let entry = SubnetRegistryOps::get(pid).ok_or(OrchestratorError::RegistryEntryMissing(pid))?;

    match entry.parent_pid {
        Some(pp) if pp == expected_parent => Ok(()),
        other => Err(OrchestratorError::ImmediateParentMismatch {
            pid,
            expected: expected_parent,
            found: other,
        }),
    }
}
