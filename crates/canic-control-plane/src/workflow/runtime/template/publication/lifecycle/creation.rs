//! Module: workflow::runtime::template::publication::lifecycle::creation
//!
//! Responsibility: create, install and activate root-owned Wasm Stores through durable progress.
//! Does not own: artifact admission, publication placement policy, or endpoint authorization.
//! Boundary: freezes authority before every paid effect and commits inventory only after proof.

use super::super::super::store_pid_for_binding;
use super::super::{
    WASM_STORE_ROLE, WasmStorePublicationWorkflow,
    fleet::{PublicationPlacement, PublicationPlacementAction, PublicationStoreFleet},
};
use crate::{
    config,
    ids::{WasmStoreBinding, WasmStoreCreationPurpose},
    ops::storage::state::subnet::{SubnetStateOps, WasmStoreCreationPlan},
    view::state::{WasmStoreCreationProgressView, WasmStoreCreationView},
    workflow::{deployment, runtime::template::publication::error::PublicationWorkflowError},
};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::{
            config::ConfigOps,
            cost_guard::CostGuardPermit,
            ic::{
                IcOps,
                mgmt::{CanisterStatusType, MgmtOps},
            },
            runtime::install_source::{ApprovedModuleSource, resolve_approved_module_source},
        },
        workflow::{
            cost_guard::CostGuardWorkflow,
            ic::provision::ProvisionWorkflow,
            runtime::{fleet_activation::FleetActivationWorkflow, install::ModuleInstallWorkflow},
        },
    },
    log,
    log::Topic,
};

#[derive(Clone, Debug)]
struct StoreCreationRuntimePlan {
    durable: WasmStoreCreationPlan,
    source: ApprovedModuleSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoreModuleState {
    Empty,
    Expected,
}

impl WasmStorePublicationWorkflow {
    /// Ensure bootstrap has exactly one recoverable root-owned Store.
    pub async fn ensure_bootstrap_wasm_store() -> Result<Vec<WasmStoreBinding>, InternalError> {
        if SubnetStateOps::wasm_store_creation().is_some() {
            let _ = Self::resume_pending_wasm_store_creation().await?;
        }
        let stores = SubnetStateOps::wasm_stores();
        if !stores.is_empty() {
            return Ok(stores.into_iter().map(|store| store.binding).collect());
        }
        let binding = Self::create_wasm_store(WasmStoreCreationPurpose::Bootstrap).await?;
        Ok(vec![binding])
    }

    /// Resume any durable Store creation before publication snapshots current inventory.
    pub(in crate::workflow::runtime::template::publication) async fn resume_pending_wasm_store_creation()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let Some(creation) = SubnetStateOps::wasm_store_creation() else {
            return Ok(None);
        };
        let plan = store_creation_plan(creation.purpose).await?;
        validate_creation_authority(&creation, &plan.durable)?;
        advance_store_creation(creation, &plan).await.map(Some)
    }

    async fn create_wasm_store(
        purpose: WasmStoreCreationPurpose,
    ) -> Result<WasmStoreBinding, InternalError> {
        if SubnetStateOps::wasm_store_creation().is_some() {
            return Self::resume_pending_wasm_store_creation()
                .await?
                .ok_or_else(|| {
                    InternalError::invariant(
                        InternalErrorOrigin::Storage,
                        "pending Wasm Store creation disappeared during resume",
                    )
                });
        }

        let plan = store_creation_plan(purpose).await?;
        let permit = reserve_store_creation(&plan)?;
        let creation = match SubnetStateOps::begin_wasm_store_creation(
            &plan.durable,
            permit.replay_settlement(),
            IcOps::now_secs(),
        ) {
            Ok(creation) => creation,
            Err(error) => {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
        };

        let pid = match MgmtOps::create_canister_with_permit(
            &permit,
            plan.durable.controllers.clone(),
            Cycles::new(plan.durable.initial_cycles),
        )
        .await
        {
            Ok(pid) => pid,
            Err(error) => {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
        };
        let created = match SubnetStateOps::mark_wasm_store_created(
            creation.sequence,
            pid,
            IcOps::now_secs(),
        ) {
            Ok(created) => created,
            Err(error) => {
                return Err(CostGuardWorkflow::complete_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
        };
        CostGuardWorkflow::complete(&permit, IcOps::now_secs())?;
        advance_store_creation(created, &plan).await
    }

    // Allocate one additional Store and add it to the managed publication fleet.
    pub(in crate::workflow::runtime::template::publication) async fn create_store_for_fleet(
        fleet: &mut PublicationStoreFleet,
        _publication_permit: &CostGuardPermit,
    ) -> Result<PublicationPlacement, InternalError> {
        let binding = match fleet.preferred_binding.clone() {
            Some(_) => Self::create_wasm_store(WasmStoreCreationPurpose::Publication).await?,
            None => Self::create_and_activate_first_publication_store().await?,
        };
        let store_pid = store_pid_for_binding(&binding)?;
        let record = SubnetStateOps::wasm_stores()
            .into_iter()
            .find(|record| record.binding == binding)
            .ok_or_else(|| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("new ws '{binding}' missing from root-owned Store inventory"),
                )
            })?;

        fleet.push_store(record, config::fleet_subnet_root_default_wasm_store());
        if fleet.preferred_binding.is_none() {
            fleet.preferred_binding = Some(binding.clone());
        }
        fleet.reserved_state = SubnetStateOps::publication_store_state();

        Ok(PublicationPlacement {
            binding,
            pid: store_pid,
            action: PublicationPlacementAction::Create,
        })
    }

    async fn create_and_activate_first_publication_store() -> Result<WasmStoreBinding, InternalError>
    {
        let binding = Self::create_wasm_store(WasmStoreCreationPurpose::Publication).await?;
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let activated =
            SubnetStateOps::activate_publication_store_binding(binding.clone(), changed_at);
        let current = SubnetStateOps::publication_store_state();
        if !activated && current.active_binding.as_ref() != Some(&binding) {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("new ws '{binding}' was not activated"),
            ));
        }
        Self::log_publication_state_transition(
            "activate_first_publication_binding",
            &previous,
            &current,
            changed_at,
        );
        Ok(binding)
    }
}

async fn store_creation_plan(
    purpose: WasmStoreCreationPurpose,
) -> Result<StoreCreationRuntimePlan, InternalError> {
    let source = resolve_approved_module_source(&WASM_STORE_ROLE).await?;
    let expected_module_hash = <[u8; 32]>::try_from(source.module_hash()).map_err(|_| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "approved Wasm Store module hash is not 32 bytes",
        )
    })?;
    let root = IcOps::canister_self();
    let mut controllers = ConfigOps::controllers()?;
    controllers.push(root);
    controllers.sort();
    controllers.dedup();
    let initial_cycles = ConfigOps::try_get_canister_by_role(&WASM_STORE_ROLE)?
        .initial_cycles
        .to_u128();
    Ok(StoreCreationRuntimePlan {
        durable: WasmStoreCreationPlan {
            purpose,
            expected_module_hash,
            payload_size_bytes: source.payload_size_bytes(),
            controllers,
            initial_cycles,
        },
        source,
    })
}

fn reserve_store_creation(
    plan: &StoreCreationRuntimePlan,
) -> Result<CostGuardPermit, InternalError> {
    deployment::reserve_wasm_store_creation_cost_guard(
        create_command_kind(plan.durable.purpose),
        &Cycles::new(plan.durable.initial_cycles),
    )
}

fn reserve_store_install(
    purpose: WasmStoreCreationPurpose,
) -> Result<CostGuardPermit, InternalError> {
    deployment::reserve_wasm_store_install_cost_guard(install_command_kind(purpose))
}

const fn create_command_kind(purpose: WasmStoreCreationPurpose) -> &'static str {
    match purpose {
        WasmStoreCreationPurpose::Bootstrap => deployment::BOOTSTRAP_WASM_STORE_CREATE_COMMAND_KIND,
        WasmStoreCreationPurpose::Publication => {
            deployment::PUBLICATION_WASM_STORE_CREATE_COMMAND_KIND
        }
    }
}

const fn install_command_kind(purpose: WasmStoreCreationPurpose) -> &'static str {
    match purpose {
        WasmStoreCreationPurpose::Bootstrap => {
            deployment::BOOTSTRAP_WASM_STORE_INSTALL_COMMAND_KIND
        }
        WasmStoreCreationPurpose::Publication => {
            deployment::PUBLICATION_WASM_STORE_INSTALL_COMMAND_KIND
        }
    }
}

async fn advance_store_creation(
    creation: WasmStoreCreationView,
    plan: &StoreCreationRuntimePlan,
) -> Result<WasmStoreBinding, InternalError> {
    validate_creation_authority(&creation, &plan.durable)?;
    match creation.progress {
        WasmStoreCreationProgressView::CreationIntent => {
            CostGuardWorkflow::recover_replay_settlement(
                &creation.creation_cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Err(InternalError::unavailable(
                "Wasm Store creation outcome is unknown; no second paid creation was attempted",
            ))
        }
        WasmStoreCreationProgressView::Created { pid, .. } => {
            CostGuardWorkflow::complete_replay_settlement(
                &creation.creation_cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            if observe_store_module(pid, &plan.durable).await? != StoreModuleState::Empty {
                return Err(InternalError::conflict(
                    "Wasm Store has unjournalled installed code before install intent",
                ));
            }
            begin_and_perform_store_install(&creation, plan, pid).await
        }
        WasmStoreCreationProgressView::InstallIntent {
            pid,
            cost_guard_settlement,
            ..
        } => {
            CostGuardWorkflow::complete_replay_settlement(
                &creation.creation_cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            match observe_store_module(pid, &plan.durable).await? {
                StoreModuleState::Expected => {
                    CostGuardWorkflow::recover_replay_settlement(
                        &cost_guard_settlement,
                        IcOps::now_secs(),
                    )?;
                    let installed = SubnetStateOps::mark_wasm_store_installed(creation.sequence)?;
                    finalize_store_creation(installed, plan, pid).await
                }
                StoreModuleState::Empty => {
                    CostGuardWorkflow::recover_replay_settlement(
                        &cost_guard_settlement,
                        IcOps::now_secs(),
                    )?;
                    renew_and_perform_store_install(&creation, plan, pid).await
                }
            }
        }
        WasmStoreCreationProgressView::Installed {
            pid,
            cost_guard_settlement,
            ..
        } => {
            CostGuardWorkflow::complete_replay_settlement(
                &creation.creation_cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            CostGuardWorkflow::recover_replay_settlement(
                &cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            finalize_store_creation(creation, plan, pid).await
        }
    }
}

async fn begin_and_perform_store_install(
    creation: &WasmStoreCreationView,
    plan: &StoreCreationRuntimePlan,
    pid: Principal,
) -> Result<WasmStoreBinding, InternalError> {
    let permit = reserve_store_install(creation.purpose)?;
    if let Err(error) =
        SubnetStateOps::begin_wasm_store_install(creation.sequence, permit.replay_settlement())
    {
        return Err(CostGuardWorkflow::recover_after_failure(
            &permit,
            IcOps::now_secs(),
            error,
        ));
    }
    perform_store_install(creation.sequence, plan, pid, &permit).await
}

async fn renew_and_perform_store_install(
    creation: &WasmStoreCreationView,
    plan: &StoreCreationRuntimePlan,
    pid: Principal,
) -> Result<WasmStoreBinding, InternalError> {
    let permit = reserve_store_install(creation.purpose)?;
    if let Err(error) =
        SubnetStateOps::renew_wasm_store_install(creation.sequence, permit.replay_settlement())
    {
        return Err(CostGuardWorkflow::recover_after_failure(
            &permit,
            IcOps::now_secs(),
            error,
        ));
    }
    perform_store_install(creation.sequence, plan, pid, &permit).await
}

async fn perform_store_install(
    sequence: u64,
    plan: &StoreCreationRuntimePlan,
    pid: Principal,
    permit: &CostGuardPermit,
) -> Result<WasmStoreBinding, InternalError> {
    let root = IcOps::canister_self();
    let payload = ProvisionWorkflow::build_nonroot_init_payload(pid, &WASM_STORE_ROLE, root)?;
    if let Err(error) = ModuleInstallWorkflow::install_with_payload_with_permit(
        permit,
        pid,
        &plan.source,
        payload,
        None,
    )
    .await
    {
        return Err(CostGuardWorkflow::recover_after_failure(
            permit,
            IcOps::now_secs(),
            error,
        ));
    }
    let installed = match SubnetStateOps::mark_wasm_store_installed(sequence) {
        Ok(installed) => installed,
        Err(error) => {
            return Err(CostGuardWorkflow::recover_after_failure(
                permit,
                IcOps::now_secs(),
                error,
            ));
        }
    };
    CostGuardWorkflow::recover(permit, IcOps::now_secs())?;
    finalize_store_creation(installed, plan, pid).await
}

async fn finalize_store_creation(
    creation: WasmStoreCreationView,
    plan: &StoreCreationRuntimePlan,
    pid: Principal,
) -> Result<WasmStoreBinding, InternalError> {
    if observe_store_module(pid, &plan.durable).await? != StoreModuleState::Expected {
        return Err(InternalError::conflict(
            "journalled Wasm Store installation has no installed module",
        ));
    }
    FleetActivationWorkflow::complete_provisioned_wasm_store_activation(pid).await?;
    let binding = WasmStorePublicationWorkflow::binding_for_store_pid(pid);
    let store = SubnetStateOps::commit_wasm_store_creation(creation.sequence, binding.clone())?;
    if store.pid != pid || store.binding != binding {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "committed Wasm Store inventory differs from creation authority",
        ));
    }
    log!(Topic::Wasm, Ok, "ws created {} ({})", binding, pid);
    Ok(binding)
}

async fn observe_store_module(
    pid: Principal,
    plan: &WasmStoreCreationPlan,
) -> Result<StoreModuleState, InternalError> {
    let status = MgmtOps::canister_status(pid).await?;
    let mut controllers = status.settings.controllers;
    controllers.sort();
    controllers.dedup();
    if controllers != plan.controllers || status.status != CanisterStatusType::Running {
        return Err(PublicationWorkflowError::InvalidState(
            "Wasm Store management state differs from durable creation authority".to_string(),
        )
        .into());
    }
    match status.module_hash {
        None => Ok(StoreModuleState::Empty),
        Some(module_hash) if module_hash.as_slice() == plan.expected_module_hash => {
            Ok(StoreModuleState::Expected)
        }
        Some(_) => Err(PublicationWorkflowError::InvalidState(
            "Wasm Store module differs from durable creation authority".to_string(),
        )
        .into()),
    }
}

fn validate_creation_authority(
    creation: &WasmStoreCreationView,
    plan: &WasmStoreCreationPlan,
) -> Result<(), InternalError> {
    let authority_is_exact = [
        creation.sequence > 0,
        creation.purpose == plan.purpose,
        creation.expected_module_hash == plan.expected_module_hash,
        creation.payload_size_bytes == plan.payload_size_bytes,
        creation.controllers == plan.controllers,
        creation.initial_cycles == plan.initial_cycles,
        creation.prepared_at > 0,
    ]
    .into_iter()
    .all(|valid| valid);
    if !authority_is_exact {
        return Err(PublicationWorkflowError::InvalidState(
            "durable Wasm Store creation authority differs from the current root plan".to_string(),
        )
        .into());
    }
    Ok(())
}
