//! Module: workflow::component_registry::component_installation
//!
//! Responsibility: reconcile pool claims and exact Component Wasm installation effects.
//! Does not own: allocation authority, Registry membership, runtime activation, or lifecycle scheduling.
//! Boundary: every paid or install effect advances its existing durable operation and is verified live.

use super::*;

pub(super) fn advance_creation(
    operation_id: [u8; 32],
    allocation: RootComponentAllocationView,
    plan: RootComponentCreationPlan,
) -> Result<RootComponentAllocationView, InternalError> {
    let allocation = reconcile_component_pool_claim(operation_id, allocation)?;
    if reconcile_existing_creation(&allocation, &plan)? {
        return Ok(allocation);
    }

    ComponentRegistryOps::validate_creation_capacity(operation_id, &plan)?;
    let pool_claim = CanisterPoolClaimKey {
        component: allocation.component,
        operation_id,
    };
    if let Some(canister) = CanisterPoolOps::claim_smallest_sufficient_ready(
        &pool_claim,
        &plan.initial_cycles,
        IcOps::now_nanos(),
    )? {
        return claim_component_pool_asset(operation_id, plan, pool_claim, canister);
    }
    Err(InternalError::public(
        canic_core::diagnostics::codes::CAPACITY_INSUFFICIENT,
    ))
}

pub(super) fn advance_child_creation(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    allocation: RootComponentChildAllocationView,
    plan: RootComponentCreationPlan,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let allocation = reconcile_component_child_pool_claim(component, operation_id, allocation)?;
    if reconcile_existing_child_creation(&allocation, &plan)? {
        return Ok(child_allocation_response(allocation));
    }

    ComponentRegistryOps::validate_child_creation_capacity(component, operation_id, &plan)?;
    let pool_claim = CanisterPoolClaimKey {
        component,
        operation_id,
    };
    if let Some(canister) = CanisterPoolOps::claim_smallest_sufficient_ready(
        &pool_claim,
        &plan.initial_cycles,
        IcOps::now_nanos(),
    )? {
        return claim_component_child_pool_asset(
            component,
            operation_id,
            plan,
            pool_claim,
            canister,
        );
    }
    Err(InternalError::public(
        canic_core::diagnostics::codes::CAPACITY_INSUFFICIENT,
    ))
}

fn claim_component_pool_asset(
    operation_id: [u8; 32],
    plan: RootComponentCreationPlan,
    claim: CanisterPoolClaimKey,
    canister: candid::Principal,
) -> Result<RootComponentAllocationView, InternalError> {
    let permit = deployment::reserve_component_pool_claim_guard()?;
    let intent = ComponentRegistryOps::begin_creation(
        operation_id,
        plan.clone(),
        permit.replay_settlement(),
    )
    .map_err(|error| CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error))?;
    let RootComponentAllocationProgressView::CreationIntent(effect) = &intent.progress else {
        return Err(CostGuardWorkflow::recover_after_failure(
            &permit,
            IcOps::now_secs(),
            InternalError::invariant(),
        ));
    };
    validate_creation_effect(effect, &plan).map_err(|error| {
        CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error)
    })?;
    let created = ComponentRegistryOps::mark_created(operation_id, canister).map_err(|error| {
        CostGuardWorkflow::complete_after_failure(&permit, IcOps::now_secs(), error)
    })?;
    CostGuardWorkflow::complete(&permit, IcOps::now_secs())?;
    CanisterPoolOps::finalize_claim(&claim, canister, IcOps::now_nanos())?;
    Ok(created)
}

fn claim_component_child_pool_asset(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    plan: RootComponentCreationPlan,
    claim: CanisterPoolClaimKey,
    canister: candid::Principal,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    let permit = deployment::reserve_component_child_pool_claim_guard()?;
    let intent = ComponentRegistryOps::begin_child_creation(
        component,
        operation_id,
        plan.clone(),
        permit.replay_settlement(),
    )
    .map_err(|error| CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error))?;
    let RootComponentChildAllocationProgressView::CreationIntent(effect) = &intent.progress else {
        return Err(CostGuardWorkflow::recover_after_failure(
            &permit,
            IcOps::now_secs(),
            InternalError::invariant(),
        ));
    };
    validate_creation_effect(effect, &plan).map_err(|error| {
        CostGuardWorkflow::recover_after_failure(&permit, IcOps::now_secs(), error)
    })?;
    let created = ComponentRegistryOps::mark_child_created(component, operation_id, canister)
        .map_err(|error| {
            CostGuardWorkflow::complete_after_failure(&permit, IcOps::now_secs(), error)
        })?;
    CostGuardWorkflow::complete(&permit, IcOps::now_secs())?;
    CanisterPoolOps::finalize_claim(&claim, canister, IcOps::now_nanos())?;
    Ok(child_allocation_response(created))
}

fn reconcile_component_pool_claim(
    operation_id: [u8; 32],
    allocation: RootComponentAllocationView,
) -> Result<RootComponentAllocationView, InternalError> {
    let claim = CanisterPoolClaimKey {
        component: allocation.component,
        operation_id,
    };
    let Some(canister) = CanisterPoolOps::claimed_canister(&claim)? else {
        return Ok(allocation);
    };
    let reconciled = match &allocation.progress {
        RootComponentAllocationProgressView::Reserved => return Ok(allocation),
        RootComponentAllocationProgressView::CreationIntent(_) => {
            ComponentRegistryOps::mark_created(operation_id, canister)?
        }
        progress => {
            require_component_progress_canister(progress, canister)?;
            allocation
        }
    };
    CanisterPoolOps::finalize_claim(&claim, canister, IcOps::now_nanos())?;
    Ok(reconciled)
}

fn reconcile_component_child_pool_claim(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    allocation: RootComponentChildAllocationView,
) -> Result<RootComponentChildAllocationView, InternalError> {
    let claim = CanisterPoolClaimKey {
        component,
        operation_id,
    };
    let Some(canister) = CanisterPoolOps::claimed_canister(&claim)? else {
        return Ok(allocation);
    };
    let reconciled = match &allocation.progress {
        RootComponentChildAllocationProgressView::Reserved => return Ok(allocation),
        RootComponentChildAllocationProgressView::CreationIntent(_) => {
            ComponentRegistryOps::mark_child_created(component, operation_id, canister)?
        }
        progress => {
            require_component_child_progress_canister(progress, canister)?;
            allocation
        }
    };
    CanisterPoolOps::finalize_claim(&claim, canister, IcOps::now_nanos())?;
    Ok(reconciled)
}

fn require_component_progress_canister(
    progress: &RootComponentAllocationProgressView,
    expected: candid::Principal,
) -> Result<(), InternalError> {
    let actual = match progress {
        RootComponentAllocationProgressView::Created { canister, .. }
        | RootComponentAllocationProgressView::InstallIntent { canister, .. }
        | RootComponentAllocationProgressView::Installed { canister, .. }
        | RootComponentAllocationProgressView::Verified { canister, .. }
        | RootComponentAllocationProgressView::Committed { canister, .. }
        | RootComponentAllocationProgressView::Removed { canister, .. } => *canister,
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => {
            return Err(InternalError::invariant());
        }
    };
    if actual != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn require_component_child_progress_canister(
    progress: &RootComponentChildAllocationProgressView,
    expected: candid::Principal,
) -> Result<(), InternalError> {
    let actual = match progress {
        RootComponentChildAllocationProgressView::Created { canister, .. }
        | RootComponentChildAllocationProgressView::InstallIntent { canister, .. }
        | RootComponentChildAllocationProgressView::Installed { canister, .. }
        | RootComponentChildAllocationProgressView::Verified { canister, .. }
        | RootComponentChildAllocationProgressView::Committed { canister, .. } => *canister,
        RootComponentChildAllocationProgressView::Reserved
        | RootComponentChildAllocationProgressView::CreationIntent(_) => {
            return Err(InternalError::invariant());
        }
    };
    if actual != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn reconcile_existing_creation(
    allocation: &RootComponentAllocationView,
    plan: &RootComponentCreationPlan,
) -> Result<bool, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Created { effect, .. }
        | RootComponentAllocationProgressView::InstallIntent {
            creation: effect, ..
        }
        | RootComponentAllocationProgressView::Installed {
            creation: effect, ..
        }
        | RootComponentAllocationProgressView::Verified {
            creation: effect, ..
        }
        | RootComponentAllocationProgressView::Committed {
            creation: effect, ..
        }
        | RootComponentAllocationProgressView::Removed {
            creation: effect, ..
        } => {
            validate_creation_effect(effect, plan)?;
            CostGuardWorkflow::complete_replay_settlement(
                &effect.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Ok(true)
        }
        RootComponentAllocationProgressView::CreationIntent(effect) => {
            validate_creation_effect(effect, plan)?;
            CostGuardWorkflow::recover_replay_settlement(
                &effect.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Ok(true)
        }
        RootComponentAllocationProgressView::Reserved => Ok(false),
    }
}

fn reconcile_existing_child_creation(
    allocation: &RootComponentChildAllocationView,
    plan: &RootComponentCreationPlan,
) -> Result<bool, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Created { effect, .. }
        | RootComponentChildAllocationProgressView::InstallIntent {
            creation: effect, ..
        }
        | RootComponentChildAllocationProgressView::Installed {
            creation: effect, ..
        }
        | RootComponentChildAllocationProgressView::Verified {
            creation: effect, ..
        }
        | RootComponentChildAllocationProgressView::Committed {
            creation: effect, ..
        } => {
            validate_creation_effect(effect, plan)?;
            CostGuardWorkflow::complete_replay_settlement(
                &effect.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Ok(true)
        }
        RootComponentChildAllocationProgressView::CreationIntent(effect) => {
            validate_creation_effect(effect, plan)?;
            CostGuardWorkflow::recover_replay_settlement(
                &effect.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            Ok(true)
        }
        RootComponentChildAllocationProgressView::Reserved => Ok(false),
    }
}

#[derive(Clone, Debug)]
pub(super) struct ComponentInstallPlan {
    pub(super) durable: RootComponentInstallPlan,
    pub(super) source: ApprovedModuleSource,
    pub(super) payload: CanisterInitPayload,
    pub(super) deployment: ProtectedComponentDeployment,
    pub(super) canister: candid::Principal,
    pub(super) expected_status_module_hash: [u8; 32],
}

#[derive(Clone, Debug)]
pub(super) struct ComponentChildInstallPlan {
    pub(super) durable: RootComponentChildInstallPlan,
    pub(super) source: ApprovedModuleSource,
    pub(super) payload: CanisterInitPayload,
    pub(super) deployment: ProtectedComponentDeployment,
    pub(super) component_group: Option<ComponentGroupDirectory>,
    pub(super) canister: candid::Principal,
    pub(super) expected_status_module_hash: [u8; 32],
    pub(super) application_init_args: Option<Vec<u8>>,
}

pub(super) async fn component_install_plan(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    allocation: &RootComponentAllocationView,
) -> Result<ComponentInstallPlan, InternalError> {
    component_install_plan_with_deployment(root, store, allocation, None).await
}

pub(super) async fn component_install_plan_with_deployment(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    allocation: &RootComponentAllocationView,
    deployment: Option<ProtectedComponentDeployment>,
) -> Result<ComponentInstallPlan, InternalError> {
    let (creation, canister) = allocation_creation_and_canister(allocation)?;
    let expected_creation = creation_plan(root.fleet_subnet_root, store, allocation)?;
    validate_creation_effect(creation, &expected_creation)?;

    let artifact = exact_store_artifact(store, &allocation.role)?;
    let source = resolved_root_store_module_source(
        store.wasm_store,
        allocation.release_set.release_build_id,
        &allocation.role,
        artifact.payload_hash,
        artifact.payload_size_bytes,
    )
    .await?;
    if source.source_canister() != &store.wasm_store {
        return Err(InternalError::invariant());
    }
    let chunk_hashes = source.chunk_hashes().to_vec();
    if source.module_hash() != artifact.payload_hash
        || source.payload_size_bytes() != artifact.payload_size_bytes
    {
        return Err(InternalError::invariant());
    }

    let binding = ComponentBinding {
        authority: root.authority.clone(),
        component: allocation.component,
        component_spec: allocation.component_spec.clone(),
        spec_hash: allocation.spec_hash,
        role: allocation.role.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        canister_id: canister,
    };
    let topology = ConfigOps::component_topology()?;
    topology
        .validate_component_binding(root, &binding)
        .map_err(|_error| InternalError::invalid_input())?;
    let spec_maximum_registry_bytes = topology
        .get(&allocation.component_spec)
        .ok_or_else(InternalError::invariant)?
        .limits
        .maximum_registry_bytes;
    let deployment =
        deployment.unwrap_or_else(|| ProtectedComponentDeployment::UngroupedOrdinary {
            binding: binding.clone(),
        });
    ConfigOps::validate_protected_component_deployment(&deployment, &binding)?;
    let maximum_registry_bytes = match &deployment {
        ProtectedComponentDeployment::UngroupedOrdinary { .. } => spec_maximum_registry_bytes,
        ProtectedComponentDeployment::GroupMember { limits, .. } => limits.maximum_registry_bytes,
    };
    let durable = RootComponentInstallPlan {
        raw_module_hash: artifact.raw_module_hash,
        protocol_profile_digest: artifact.protocol_profile_digest,
        chunk_hashes,
        binding: binding.clone(),
        maximum_registry_bytes,
    };
    let target = ManagedCanisterBinding::Component(binding.clone());
    let admission = if ConfigOps::role_uses_fleet_admission(&allocation.role)? {
        Some(
            compile_fleet_admission_projection(
                &crate::workflow::root_admission::current_policy()?,
                target,
            )
            .map_err(|_error| InternalError::invariant())?,
        )
    } else {
        None
    };
    let payload = CanisterInitPayload {
        install_id: allocation.operation_id,
        release_build_id: allocation.release_set.release_build_id,
        component_deployment: Box::new(deployment.clone()),
        authority: CanisterInitAuthority::Component {
            root: root.clone(),
            binding,
        },
        admission,
    };

    Ok(ComponentInstallPlan {
        durable,
        source,
        payload,
        deployment,
        canister,
        expected_status_module_hash: artifact.payload_hash,
    })
}

pub(super) async fn child_component_install_plan(
    root: &canic_core::ids::FleetSubnetRootBinding,
    store: &RootStoreBootstrapResponse,
    parent: &ManagedCanisterBinding,
    allocation: &RootComponentChildAllocationView,
) -> Result<ComponentChildInstallPlan, InternalError> {
    let (creation, canister) = child_allocation_creation_and_canister(allocation)?;
    let expected_creation = child_creation_plan(root.fleet_subnet_root, store, allocation)?;
    validate_creation_effect(creation, &expected_creation)?;

    let artifact = exact_store_artifact(store, &allocation.child_role)?;
    let source = resolved_root_store_module_source(
        store.wasm_store,
        allocation.release_set.release_build_id,
        &allocation.child_role,
        artifact.payload_hash,
        artifact.payload_size_bytes,
    )
    .await?;
    if source.source_canister() != &store.wasm_store {
        return Err(InternalError::invariant());
    }
    let chunk_hashes = source.chunk_hashes().to_vec();
    if source.module_hash() != artifact.payload_hash
        || source.payload_size_bytes() != artifact.payload_size_bytes
    {
        return Err(InternalError::invariant());
    }

    let component = match parent {
        ManagedCanisterBinding::Component(binding) => binding.clone(),
        ManagedCanisterBinding::ComponentChild(binding) => binding.component.clone(),
    };
    let binding = canic_core::ids::ComponentChildBinding {
        component,
        parent_canister_id: allocation.parent_canister_id,
        role: allocation.child_role.clone(),
        canister_id: canister,
    };
    ConfigOps::component_topology()?
        .validate_component_child_binding(root, &binding)
        .map_err(|_error| InternalError::invalid_input())?;
    let partition = ComponentRegistryOps::partition(allocation.component)?
        .ok_or_else(InternalError::invariant)?;
    let deployment_authority = RootComponentProvisioningOps::component_deployment_authority(
        &partition.provisioning_origin,
        &binding.component,
    )?;
    let deployment = deployment_authority.deployment;
    ConfigOps::validate_protected_component_deployment(&deployment, &binding.component)?;
    let durable = RootComponentChildInstallPlan {
        raw_module_hash: artifact.raw_module_hash,
        protocol_profile_digest: artifact.protocol_profile_digest,
        chunk_hashes,
        binding: binding.clone(),
        maximum_registry_bytes: allocation.maximum_registry_bytes,
    };
    let target = ManagedCanisterBinding::ComponentChild(binding.clone());
    let admission = if ConfigOps::role_uses_fleet_admission(&allocation.child_role)? {
        Some(
            compile_fleet_admission_projection(
                &crate::workflow::root_admission::current_policy()?,
                target,
            )
            .map_err(|_error| InternalError::invariant())?,
        )
    } else {
        None
    };
    let payload = CanisterInitPayload {
        install_id: allocation.operation_id,
        release_build_id: allocation.release_set.release_build_id,
        component_deployment: Box::new(deployment.clone()),
        authority: CanisterInitAuthority::ComponentChild {
            root: root.clone(),
            binding,
        },
        admission,
    };

    Ok(ComponentChildInstallPlan {
        durable,
        source,
        payload,
        deployment,
        component_group: deployment_authority.component_group,
        canister,
        expected_status_module_hash: artifact.payload_hash,
        application_init_args: allocation.application_init_args.clone(),
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one workflow keeps every durable install and uncertain-outcome phase explicit"
)]
pub(super) async fn advance_child_install(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    allocation: RootComponentChildAllocationView,
    plan: ComponentChildInstallPlan,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    match &allocation.progress {
        RootComponentChildAllocationProgressView::Reserved
        | RootComponentChildAllocationProgressView::CreationIntent(_) => {
            Err(InternalError::conflict())
        }
        RootComponentChildAllocationProgressView::Created { .. } => {
            if observed_child_install_state(&plan).await? {
                return Err(InternalError::conflict());
            }
            ComponentRegistryOps::validate_child_install_capacity(
                component,
                operation_id,
                &plan.durable,
            )?;
            let permit = deployment::reserve_component_child_install_cost_guard()?;
            let intent = match ComponentRegistryOps::begin_child_install(
                component,
                operation_id,
                plan.durable.clone(),
                permit.replay_settlement(),
            ) {
                Ok(intent) => intent,
                Err(error) => {
                    return Err(CostGuardWorkflow::recover_after_failure(
                        &permit,
                        IcOps::now_secs(),
                        error,
                    ));
                }
            };
            let installation = child_install_effect(&intent)?;
            if let Err(error) = validate_child_install_effect(installation, &plan.durable) {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
            perform_child_install(component, operation_id, &plan, &permit).await
        }
        RootComponentChildAllocationProgressView::InstallIntent { installation, .. } => {
            validate_child_install_effect(installation, &plan.durable)?;
            if observed_child_install_state(&plan).await? {
                CostGuardWorkflow::recover_replay_settlement(
                    &installation.cost_guard_settlement,
                    IcOps::now_secs(),
                )?;
                let installed =
                    ComponentRegistryOps::mark_child_installed(component, operation_id)?;
                return verify_and_mark_child_installed(component, operation_id, installed, &plan)
                    .await;
            }

            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            let permit = deployment::reserve_component_child_install_cost_guard()?;
            let renewed = match ComponentRegistryOps::renew_child_install_intent(
                component,
                operation_id,
                &plan.durable,
                permit.replay_settlement(),
            ) {
                Ok(renewed) => renewed,
                Err(error) => {
                    return Err(CostGuardWorkflow::recover_after_failure(
                        &permit,
                        IcOps::now_secs(),
                        error,
                    ));
                }
            };
            let installation = child_install_effect(&renewed)?;
            if let Err(error) = validate_child_install_effect(installation, &plan.durable) {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
            perform_child_install(component, operation_id, &plan, &permit).await
        }
        RootComponentChildAllocationProgressView::Installed { installation, .. } => {
            validate_child_install_effect(installation, &plan.durable)?;
            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            verify_and_mark_child_installed(component, operation_id, allocation, &plan).await
        }
        RootComponentChildAllocationProgressView::Verified { installation, .. }
        | RootComponentChildAllocationProgressView::Committed { installation, .. } => {
            validate_child_install_effect(installation, &plan.durable)?;
            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            verify_installed_child(&plan).await?;
            Ok(child_allocation_response(allocation))
        }
    }
}

async fn perform_child_install(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    plan: &ComponentChildInstallPlan,
    permit: &canic_core::control_plane_support::ops::cost_guard::CostGuardPermit,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    if let Err(error) = ModuleInstallWorkflow::install_with_payload_with_permit(
        permit,
        plan.canister,
        &plan.source,
        plan.payload.clone(),
        plan.application_init_args.clone(),
    )
    .await
    {
        return Err(CostGuardWorkflow::recover_after_failure(
            permit,
            IcOps::now_secs(),
            error,
        ));
    }

    let installed = match ComponentRegistryOps::mark_child_installed(component, operation_id) {
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
    verify_and_mark_child_installed(component, operation_id, installed, plan).await
}

async fn verify_and_mark_child_installed(
    component: canic_core::ids::ComponentInstanceId,
    operation_id: [u8; 32],
    _installed: RootComponentChildAllocationView,
    plan: &ComponentChildInstallPlan,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    verify_installed_child(plan).await?;
    let verified = ComponentRegistryOps::mark_child_verified(component, operation_id)?;
    if !matches!(
        verified.progress,
        RootComponentChildAllocationProgressView::Verified { .. }
    ) {
        return Err(InternalError::invariant());
    }
    Ok(child_allocation_response(verified))
}

async fn observed_child_install_state(
    plan: &ComponentChildInstallPlan,
) -> Result<bool, InternalError> {
    let status = MgmtOps::canister_status(plan.canister).await?;
    if status.settings.controllers != vec![plan.durable.binding.component.fleet_subnet_root] {
        return Err(InternalError::conflict());
    }
    match status.module_hash {
        None => Ok(false),
        Some(module_hash) if module_hash == plan.expected_status_module_hash => Ok(true),
        Some(_) => Err(InternalError::conflict()),
    }
}

pub(super) async fn verify_installed_child(
    plan: &ComponentChildInstallPlan,
) -> Result<(), InternalError> {
    if !observed_child_install_state(plan).await? {
        return Err(InternalError::unavailable());
    }
    let observed = query_managed_binding(plan.canister).await?;
    let expected = ManagedCanisterBinding::ComponentChild(plan.durable.binding.clone());
    if observed != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn removed_allocation_response(
    allocation: RootComponentAllocationView,
    plan: &ComponentInstallPlan,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let RootComponentAllocationProgressView::Removed { installation, .. } = &allocation.progress
    else {
        return Err(InternalError::invariant());
    };
    validate_install_effect(installation, &plan.durable)?;
    CostGuardWorkflow::recover_replay_settlement(
        &installation.cost_guard_settlement,
        IcOps::now_secs(),
    )?;
    allocation_response(allocation)
}

pub(super) async fn advance_install(
    operation_id: [u8; 32],
    allocation: RootComponentAllocationView,
    plan: ComponentInstallPlan,
) -> Result<RootComponentAllocationResponse, InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict()),
        RootComponentAllocationProgressView::Created { .. } => {
            if observed_install_state(&plan).await? {
                return Err(InternalError::conflict());
            }
            ComponentRegistryOps::validate_install_capacity(operation_id, &plan.durable)?;
            let permit = deployment::reserve_component_install_cost_guard()?;
            let intent = match ComponentRegistryOps::begin_install(
                operation_id,
                plan.durable.clone(),
                permit.replay_settlement(),
            ) {
                Ok(intent) => intent,
                Err(error) => {
                    return Err(CostGuardWorkflow::recover_after_failure(
                        &permit,
                        IcOps::now_secs(),
                        error,
                    ));
                }
            };
            let installation = install_effect(&intent)?;
            if let Err(error) = validate_install_effect(installation, &plan.durable) {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
            perform_install(operation_id, &plan, &permit).await
        }
        RootComponentAllocationProgressView::InstallIntent { installation, .. } => {
            validate_install_effect(installation, &plan.durable)?;
            if observed_install_state(&plan).await? {
                CostGuardWorkflow::recover_replay_settlement(
                    &installation.cost_guard_settlement,
                    IcOps::now_secs(),
                )?;
                let installed = ComponentRegistryOps::mark_installed(operation_id)?;
                return verify_and_mark_installed(operation_id, installed, &plan).await;
            }

            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            let permit = deployment::reserve_component_install_cost_guard()?;
            let renewed = match ComponentRegistryOps::renew_install_intent(
                operation_id,
                &plan.durable,
                permit.replay_settlement(),
            ) {
                Ok(renewed) => renewed,
                Err(error) => {
                    return Err(CostGuardWorkflow::recover_after_failure(
                        &permit,
                        IcOps::now_secs(),
                        error,
                    ));
                }
            };
            let installation = install_effect(&renewed)?;
            if let Err(error) = validate_install_effect(installation, &plan.durable) {
                return Err(CostGuardWorkflow::recover_after_failure(
                    &permit,
                    IcOps::now_secs(),
                    error,
                ));
            }
            perform_install(operation_id, &plan, &permit).await
        }
        RootComponentAllocationProgressView::Installed { installation, .. } => {
            validate_install_effect(installation, &plan.durable)?;
            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            verify_and_mark_installed(operation_id, allocation, &plan).await
        }
        RootComponentAllocationProgressView::Verified { installation, .. }
        | RootComponentAllocationProgressView::Committed { installation, .. } => {
            validate_install_effect(installation, &plan.durable)?;
            CostGuardWorkflow::recover_replay_settlement(
                &installation.cost_guard_settlement,
                IcOps::now_secs(),
            )?;
            verify_committed_or_verified_install(&allocation, &plan).await?;
            allocation_response(allocation)
        }
        RootComponentAllocationProgressView::Removed { .. } => {
            removed_allocation_response(allocation, &plan)
        }
    }
}

pub(super) async fn verify_committed_or_verified_install(
    allocation: &RootComponentAllocationView,
    plan: &ComponentInstallPlan,
) -> Result<(), InternalError> {
    match &allocation.progress {
        RootComponentAllocationProgressView::Verified { .. } => {
            verify_prepared_installed_component(plan).await
        }
        RootComponentAllocationProgressView::Committed { .. } => {
            verify_installed_component(plan).await
        }
        _ => Err(InternalError::invariant()),
    }
}

async fn perform_install(
    operation_id: [u8; 32],
    plan: &ComponentInstallPlan,
    permit: &canic_core::control_plane_support::ops::cost_guard::CostGuardPermit,
) -> Result<RootComponentAllocationResponse, InternalError> {
    if let Err(error) = ModuleInstallWorkflow::install_with_payload_with_permit(
        permit,
        plan.canister,
        &plan.source,
        plan.payload.clone(),
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

    let installed = match ComponentRegistryOps::mark_installed(operation_id) {
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
    verify_and_mark_installed(operation_id, installed, plan).await
}

async fn verify_and_mark_installed(
    operation_id: [u8; 32],
    _installed: RootComponentAllocationView,
    plan: &ComponentInstallPlan,
) -> Result<RootComponentAllocationResponse, InternalError> {
    verify_prepared_installed_component(plan).await?;
    let verified = ComponentRegistryOps::mark_verified(operation_id)?;
    if !matches!(
        verified.progress,
        RootComponentAllocationProgressView::Verified { .. }
    ) {
        return Err(InternalError::invariant());
    }
    allocation_response(verified)
}

async fn observed_install_state(plan: &ComponentInstallPlan) -> Result<bool, InternalError> {
    let status = MgmtOps::canister_status(plan.canister).await?;
    if status.settings.controllers != vec![plan.durable.binding.fleet_subnet_root] {
        return Err(InternalError::conflict());
    }
    match status.module_hash {
        None => Ok(false),
        Some(module_hash) if module_hash == plan.expected_status_module_hash => Ok(true),
        Some(_) => Err(InternalError::conflict()),
    }
}

async fn installed_component_status(
    plan: &ComponentInstallPlan,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    if !observed_install_state(plan).await? {
        return Err(InternalError::unavailable());
    }
    let observed = query_managed_binding(plan.canister).await?;
    let expected = ManagedCanisterBinding::Component(plan.durable.binding.clone());
    if observed != expected {
        return Err(InternalError::conflict());
    }
    query_component_runtime_status(plan.canister, plan.payload.install_id).await
}

pub(super) async fn verify_installed_component(
    plan: &ComponentInstallPlan,
) -> Result<(), InternalError> {
    let status = installed_component_status(plan).await?;
    validate_installed_component_status(
        &status,
        plan.payload.install_id,
        &ManagedCanisterBinding::Component(plan.durable.binding.clone()),
        &plan.deployment,
    )
}

async fn verify_prepared_installed_component(
    plan: &ComponentInstallPlan,
) -> Result<(), InternalError> {
    let status = installed_component_status(plan).await?;
    validate_prepared_install_status(
        &status,
        plan.payload.install_id,
        &ManagedCanisterBinding::Component(plan.durable.binding.clone()),
        &plan.deployment,
    )
}

pub(super) fn validate_prepared_install_status(
    status: &ComponentRuntimeStatusResponse,
    operation_id: [u8; 32],
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
) -> Result<(), InternalError> {
    validate_installed_component_status(status, operation_id, binding, deployment)?;
    let directory_is_empty = ComponentRuntimeDirectoryStatusIdentity::from_status(status)
        == ComponentRuntimeDirectoryStatusIdentity::empty();
    let runtime_is_prepared = status.phase == ComponentRuntimePhase::AwaitingDirectory
        && directory_is_empty
        && status.activation.is_none();
    if !runtime_is_prepared {
        return Err(InternalError::conflict());
    }
    Ok(())
}

pub(super) fn validate_installed_component_status(
    status: &ComponentRuntimeStatusResponse,
    operation_id: [u8; 32],
    binding: &ManagedCanisterBinding,
    deployment: &ProtectedComponentDeployment,
) -> Result<(), InternalError> {
    if status.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    if &status.binding != binding {
        return Err(InternalError::conflict());
    }
    if status.deployment.as_ref() != deployment {
        return Err(InternalError::conflict());
    }
    Ok(())
}
