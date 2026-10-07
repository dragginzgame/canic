use crate::dto::root::RootOperationStatusResponse;
use canic_core::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap::compiled::{ConfigModel, RoleRuntimeAuthority},
    control_plane_support::view::fleet_activation::FleetActivationTransition,
    dto::fleet_registry::FleetSubnetRootRegistrySyncRequest,
    dto::fleet_subnet_root::{
        FleetSubnetRootAuthority, FleetSubnetRootCanisterSummary, FleetSubnetRootInitArgs,
        FleetSubnetWasmStoreAdoptionRequest, FleetSubnetWasmStoreAdoptionResponse,
    },
    dto::{
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentDirectoryPageRequest,
            ComponentDirectoryPageResponse, ComponentRegistryActivePartitionRequest,
            ComponentRegistryActivePartitionResponse, ComponentRegistryPartitionRequest,
            ComponentRegistryPartitionResponse, RootComponentAllocationRequest,
            RootComponentAllocationResponse, RootComponentChildAllocationRequest,
            RootComponentChildAllocationResponse, RootComponentDeletionRequest,
            RootComponentDeletionResponse, RootComponentDeletionStatusRequest,
            RootComponentDirectoryPreparationRequest, RootComponentDirectoryPreparationResponse,
            RootComponentDrainingAdvanceRequest, RootComponentDrainingAdvanceResponse,
            RootComponentDrainingRequest, RootComponentDrainingResponse,
            RootComponentDrainingStatusRequest, RootComponentFinalInventoryRequest,
            RootComponentFinalInventoryResponse, RootComponentMembershipActivationRequest,
            RootComponentMembershipActivationResponse, RootComponentQuiescenceRequest,
            RootComponentQuiescenceResponse, RootComponentQuiescenceStatusRequest,
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
            RootComponentRuntimeActivationRequest, RootComponentRuntimeActivationResponse,
            RootComponentSubtreeRemovalRequest, RootComponentSubtreeRemovalResponse,
            RootPeerComponentAllocationRequest,
        },
        fleet_activation::{FleetActivationResumeRequest, FleetActivationStatusResponse},
    },
};
use std::time::Duration;

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    /// Freeze target-bound application bytes after endpoint controller authentication.
    pub fn bind_component_initialization(
        request: canic_core::dto::component_registry::RootComponentInitializationRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::bind_component_initialization(request)
            .map_err(Into::into)
    }

    /// Read release funding evidence after endpoint-level controller authentication.
    pub fn root_funding_release_status(
        start_after: Option<u64>,
    ) -> Result<crate::dto::root::RootFundingReleaseResponse, canic_core::dto::error::Error> {
        crate::workflow::root_funding::release_status(start_after).map_err(Into::into)
    }

    /// Return protected Root funding state after endpoint-level controller authentication.
    pub fn root_funding_status()
    -> Result<crate::dto::root::RootFundingStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::root_funding::status().map_err(Into::into)
    }

    /// Authenticate one Root admission phase before distribution state is read.
    pub fn authorize_root_admission_caller(
        caller: candid::Principal,
    ) -> Result<(), canic_core::dto::error::Error> {
        crate::workflow::root_admission::authorize_coordinator(caller).map_err(Into::into)
    }

    /// Start or replay exact subtree preparation through the Root-owned journal.
    pub fn prepare_root_fleet_admission(
        request: canic_core::dto::fleet_admission::FleetAdmissionPrepareRootRequest,
    ) -> Result<
        canic_core::dto::fleet_admission::FleetAdmissionRootReceipt,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_admission::prepare(request).map_err(Into::into)
    }

    /// Start or replay exact subtree successor activation.
    pub fn activate_root_fleet_admission(
        request: canic_core::dto::fleet_admission::FleetAdmissionActivateRootRequest,
    ) -> Result<
        canic_core::dto::fleet_admission::FleetAdmissionRootReceipt,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_admission::activate(request).map_err(Into::into)
    }

    /// Start or replay exact subtree opening.
    pub fn open_root_fleet_admission(
        request: canic_core::dto::fleet_admission::FleetAdmissionOpenRootRequest,
    ) -> Result<
        canic_core::dto::fleet_admission::FleetAdmissionRootReceipt,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_admission::open(request).map_err(Into::into)
    }

    /// Return one bounded controller-only Root admission progress page.
    pub fn root_admission_status(
        request: canic_core::dto::page::PageRequest,
    ) -> Result<
        canic_core::dto::fleet_admission::FleetAdmissionRootStatusResponse,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_admission::status(request).map_err(Into::into)
    }

    /// Suspend the exact Root control-plane owners before core authority sealing.
    pub async fn prepare_authority_snapshot(
        request: canic_core::dto::authority_restore::AuthoritySnapshotRequest,
    ) -> Result<
        canic_core::dto::authority_restore::AuthorityRestoreFenceStatusResponse,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_funding::require_authority_snapshot_resumable()
            .map_err(canic_core::dto::error::Error::from)?;
        canic_core::api::timer::TimerApi::require_root_authority_snapshot_resumable().map_err(
            |_error| canic_core::control_plane_support::error::InternalError::invariant(),
        )?;
        crate::workflow::canister_pool::suspend_for_authority_snapshot().map_err(|_error| {
            canic_core::control_plane_support::error::InternalError::invariant()
        })?;
        match canic_core::api::authority_restore::AuthorityRestoreApi::prepare_root_snapshot(
            request,
        )
        .await
        {
            Ok(status) => Ok(status),
            Err(error) => {
                crate::workflow::canister_pool::resume_after_authority_snapshot().unwrap_or_else(
                    |resume_error| {
                        ic_cdk::trap(format!(
                            "Root timer rollback failed after snapshot rejection: {resume_error}"
                        ))
                    },
                );
                Err(error)
            }
        }
    }

    /// Resume core authority and then reconstruct exact Root control-plane demand.
    pub async fn resume_authority_snapshot(
        request: canic_core::dto::authority_restore::AuthoritySnapshotRequest,
    ) -> Result<
        canic_core::dto::authority_restore::AuthorityRestoreFenceStatusResponse,
        canic_core::dto::error::Error,
    > {
        let status =
            canic_core::api::authority_restore::AuthorityRestoreApi::resume_root_snapshot(request)
                .await?;
        crate::workflow::canister_pool::resume_after_authority_snapshot().unwrap_or_else(|error| {
            ic_cdk::trap(format!(
                "Root timer reconstruction failed while authority remained sealed: {error}"
            ))
        });
        Ok(status)
    }

    /// Resolve one indexed Root-owned durable operation for the consolidated status lane.
    pub fn root_operation_status(
        operation_id: [u8; 32],
        caller: candid::Principal,
        caller_is_controller: bool,
    ) -> Result<RootOperationStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::root_status::operation_status(operation_id, caller, caller_is_controller)
            .map_err(Into::into)
    }

    /// Resolve one exact Root Component-provisioning operation without an
    /// ambiguous lookup across unrelated durable operation owners.
    pub fn root_component_provisioning_status(
        operation_id: [u8; 32],
        caller: candid::Principal,
        caller_is_controller: bool,
    ) -> Result<
        canic_core::dto::component_provisioning::RootComponentProvisioningStatusResponse,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_status::component_provisioning_status(
            operation_id,
            caller,
            caller_is_controller,
        )
        .map_err(Into::into)
    }

    /// Resolve one exact Root child-allocation operation without an ambiguous
    /// lookup across unrelated durable operation owners.
    pub fn root_component_child_provisioning_status(
        operation_id: [u8; 32],
        caller: candid::Principal,
        caller_is_controller: bool,
    ) -> Result<RootComponentChildAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::root_status::component_child_provisioning_status(
            operation_id,
            caller,
            caller_is_controller,
        )
        .map_err(Into::into)
    }

    /// Delegate root init-time runtime seeding to the current core implementation.
    pub fn init_root_canister_before_bootstrap(
        args: FleetSubnetRootInitArgs,
        embedded_release_build_id: Option<&str>,
        runtime_authority: RoleRuntimeAuthority,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        let bootstrap_initialization = args
            .capacity_import_bootstrap
            .as_ref()
            .map(|_| args.clone());
        let canister_pool_config = args.authority.binding.limits.canister_pool.clone();
        let awaiting_capacity = args.capacity_import_bootstrap.is_some();
        let canister_pool_imports = args.canister_pool_imports.clone();
        let wasm_store = args.authority.wasm_store_authority.wasm_store;
        crate::runtime::root_funding::register();
        canic_core::api::lifecycle::root::LifecycleApi::init_root_canister_before_bootstrap(
            args,
            embedded_release_build_id,
            runtime_authority,
            config,
            config_source,
            config_path,
        );
        if let Some(args) = bootstrap_initialization {
            crate::ops::canister_pool::capacity_import::bootstrap::initialize(&args)
                .unwrap_or_else(|error| {
                    ic_cdk::trap(format!("Capacity bootstrap initialization failed: {error}"))
                });
        }
        crate::workflow::root_funding::initialize().unwrap_or_else(|error| {
            ic_cdk::trap(format!("Root funding initialization failed: {error}"))
        });
        crate::workflow::canister_pool::declare();
        let now_ns = canic_core::control_plane_support::ops::ic::IcOps::now_nanos();
        crate::ops::canister_pool::CanisterPoolOps::initialize_store(wasm_store, now_ns)
            .and_then(|()| {
                if awaiting_capacity {
                    return Ok(());
                }
                crate::ops::canister_pool::CanisterPoolOps::initialize_imports(
                    &canister_pool_config,
                    &canister_pool_imports,
                    now_ns,
                )
            })
            .unwrap_or_else(|error| {
                ic_cdk::trap(format!("Canister pool initialization failed: {error}"))
            });
    }

    pub fn fleet_subnet_root_authority()
    -> Result<FleetSubnetRootAuthority, canic_core::dto::error::Error> {
        canic_core::api::fleet_activation::FleetActivationApi::root_authority()
    }

    /// Authenticate one Root funding acceptance before protected workflow reads.
    pub fn authorize_root_funding_caller(
        caller: candid::Principal,
    ) -> Result<(), canic_core::dto::error::Error> {
        crate::workflow::root_funding::authorize_coordinator(caller).map_err(Into::into)
    }

    /// Accept one exact Coordinator grant or return its zero-accept replay receipt.
    pub fn accept_root_funding(
        request: canic_core::dto::fleet_funding::FleetRootFundingAcceptanceRequest,
    ) -> Result<
        canic_core::dto::fleet_funding::FleetRootFundingAcceptanceReceipt,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_funding::accept(request).map_err(Into::into)
    }

    /// Retain one exact Coordinator-authored policy-rotation fence.
    pub fn prepare_root_funding_policy_rotation(
        request: canic_core::dto::fleet_funding::FleetFundingPolicyRotationRootPrepareRequest,
    ) -> Result<
        canic_core::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_funding::prepare_policy_rotation(request).map_err(Into::into)
    }

    /// Converge protected Root authority and its Registry mirror to one successor.
    pub async fn activate_root_funding_policy_rotation(
        request: canic_core::dto::fleet_funding::FleetFundingPolicyRotationRootActivateRequest,
    ) -> Result<
        canic_core::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt,
        canic_core::dto::error::Error,
    > {
        crate::workflow::root_funding::activate_policy_rotation(request)
            .await
            .map_err(Into::into)
    }

    pub fn fleet_subnet_root_canister_summary()
    -> Result<FleetSubnetRootCanisterSummary, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::canister_summary().map_err(Into::into)
    }

    pub async fn adopt_fleet_subnet_wasm_store(
        request: FleetSubnetWasmStoreAdoptionRequest,
    ) -> Result<FleetSubnetWasmStoreAdoptionResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::adopt_wasm_store(request)
            .await
            .map_err(Into::into)
    }

    pub async fn accept_fleet_registry_synchronization(
        request: FleetSubnetRootRegistrySyncRequest,
    ) -> Result<canic_core::dto::role::OperationReceipt, canic_core::dto::error::Error> {
        crate::workflow::fleet_registry_mirror::accept_synchronization(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_component_registry(
        request: RootComponentRegistryPreparationRequest,
    ) -> Result<RootComponentRegistryStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::prepare(request)
            .await
            .map_err(Into::into)
    }

    pub fn local_component_registry_status(
        request: RootComponentRegistryPreparationRequest,
    ) -> Result<RootComponentRegistryStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::local_status(request).map_err(Into::into)
    }

    pub async fn reserve_component_allocation(
        request: RootComponentAllocationRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::reserve_allocation(request)
            .await
            .map_err(Into::into)
    }

    pub async fn reserve_peer_component_allocation(
        request: RootPeerComponentAllocationRequest,
    ) -> Result<RootComponentAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::reserve_peer_allocation(request)
            .await
            .map_err(Into::into)
    }

    /// Authorize one peer-allocation command before endpoint workflow dispatch.
    pub fn authorize_peer_component_allocation_caller(
        request: &RootPeerComponentAllocationRequest,
        caller: candid::Principal,
    ) -> Result<(), canic_core::dto::error::Error> {
        crate::workflow::component_registry::authorize_peer_allocation_caller(request, caller)
            .map_err(Into::into)
    }

    /// Detach private autonomous advancement for one accepted top-level allocation.
    pub fn schedule_component_allocation(operation_id: [u8; 32]) {
        crate::workflow::component_registry::schedule_component_allocation(operation_id);
    }

    /// Detach private autonomous advancement for one accepted direct-child allocation.
    pub fn schedule_component_child_allocation(
        component: canic_core::ids::ComponentInstanceId,
        operation_id: [u8; 32],
    ) {
        crate::workflow::component_registry::schedule_component_child_allocation(
            component,
            operation_id,
        );
    }

    /// Detach private autonomous advancement for one accepted subtree removal.
    pub fn schedule_component_subtree_removal(
        component: canic_core::ids::ComponentInstanceId,
        operation_id: [u8; 32],
    ) {
        crate::workflow::component_registry::schedule_subtree_removal(component, operation_id);
    }

    /// Detach private autonomous advancement for one accepted top-level Component removal.
    pub fn schedule_component_removal(
        component: canic_core::ids::ComponentInstanceId,
        operation_id: [u8; 32],
    ) {
        crate::workflow::component_registry::schedule_component_removal(component, operation_id);
    }

    pub fn accept_fleet_subnet_root_removal(
        request: canic_core::dto::role::RootRemovalRequest,
    ) -> Result<canic_core::dto::role::OperationReceipt, canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::accept_root_removal(request).map_err(Into::into)
    }

    pub fn authorize_fleet_subnet_root_removal_caller(
        caller: candid::Principal,
        caller_is_controller: bool,
    ) -> Result<(), canic_core::dto::error::Error> {
        crate::workflow::fleet_subnet_root::authorize_root_removal_caller(
            caller,
            caller_is_controller,
        )
        .map_err(Into::into)
    }

    pub async fn reserve_component_child(
        request: RootComponentChildAllocationRequest,
    ) -> Result<RootComponentChildAllocationResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::reserve_child_allocation(request)
            .await
            .map_err(Into::into)
    }

    /// Authorize one direct-child command before endpoint workflow dispatch.
    pub fn authorize_component_child_caller(
        request: &RootComponentChildAllocationRequest,
        caller: candid::Principal,
    ) -> Result<(), canic_core::dto::error::Error> {
        crate::workflow::component_registry::authorize_child_allocation_caller(request, caller)
            .map_err(Into::into)
    }

    pub async fn begin_component_draining(
        request: RootComponentDrainingRequest,
    ) -> Result<RootComponentDrainingResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::begin_component_draining(request)
            .await
            .map_err(Into::into)
    }

    pub fn component_draining_status(
        request: RootComponentDrainingStatusRequest,
    ) -> Result<RootComponentDrainingResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::component_draining_status(request).map_err(Into::into)
    }

    pub async fn quiesce_component(
        request: RootComponentQuiescenceRequest,
    ) -> Result<RootComponentQuiescenceResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::quiesce_component(
            request,
        ))
        .await
        .map_err(Into::into)
    }

    pub fn component_quiescence_status(
        request: RootComponentQuiescenceStatusRequest,
    ) -> Result<RootComponentQuiescenceResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::component_quiescence_status(request)
            .map_err(Into::into)
    }

    pub async fn advance_component_draining(
        request: RootComponentDrainingAdvanceRequest,
    ) -> Result<RootComponentDrainingAdvanceResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::advance_component_draining(request))
            .await
            .map_err(Into::into)
    }

    pub async fn finalize_component_inventory(
        request: RootComponentFinalInventoryRequest,
    ) -> Result<RootComponentFinalInventoryResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::finalize_component_inventory(request)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_component(
        request: RootComponentDeletionRequest,
    ) -> Result<RootComponentDeletionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::delete_component(request)
            .await
            .map_err(Into::into)
    }

    pub fn remove_component_membership(
        request: RootComponentDeletionRequest,
    ) -> Result<RootComponentDeletionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::remove_component_membership(request)
            .map_err(Into::into)
    }

    pub fn component_deletion_status(
        request: RootComponentDeletionStatusRequest,
    ) -> Result<RootComponentDeletionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::component_deletion_status(request).map_err(Into::into)
    }

    pub async fn begin_component_subtree_removal(
        request: RootComponentSubtreeRemovalRequest,
    ) -> Result<RootComponentSubtreeRemovalResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::begin_subtree_removal(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_component_directories(
        request: RootComponentDirectoryPreparationRequest,
    ) -> Result<RootComponentDirectoryPreparationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::prepare_component_directories(request))
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_peer_component_directories(
        request: RootComponentDirectoryPreparationRequest,
    ) -> Result<RootComponentDirectoryPreparationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::prepare_peer_component_directories(request))
            .await
            .map_err(Into::into)
    }

    pub async fn activate_component_runtime(
        request: RootComponentRuntimeActivationRequest,
    ) -> Result<RootComponentRuntimeActivationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::activate_component_runtime(request))
            .await
            .map_err(Into::into)
    }

    pub async fn activate_peer_component_runtime(
        request: RootComponentRuntimeActivationRequest,
    ) -> Result<RootComponentRuntimeActivationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::activate_peer_component_runtime(request))
            .await
            .map_err(Into::into)
    }

    pub async fn activate_component_membership(
        request: RootComponentMembershipActivationRequest,
    ) -> Result<RootComponentMembershipActivationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::activate_component_membership(request))
            .await
            .map_err(Into::into)
    }

    pub async fn activate_peer_component_membership(
        request: RootComponentMembershipActivationRequest,
    ) -> Result<RootComponentMembershipActivationResponse, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::component_registry::activate_peer_component_membership(request))
            .await
            .map_err(Into::into)
    }

    pub fn component_registry_partition(
        request: ComponentRegistryPartitionRequest,
    ) -> Result<ComponentRegistryPartitionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::registry_partition(request).map_err(Into::into)
    }

    pub fn component_registry_active_partition(
        request: ComponentRegistryActivePartitionRequest,
    ) -> Result<ComponentRegistryActivePartitionResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::active_registry_partition(request).map_err(Into::into)
    }

    pub fn component_directory_head(
        request: ComponentDirectoryHeadRequest,
    ) -> Result<ComponentDirectoryHead, canic_core::dto::error::Error> {
        crate::workflow::component_registry::directory_head(request).map_err(Into::into)
    }

    pub fn component_directory_page(
        request: ComponentDirectoryPageRequest,
    ) -> Result<ComponentDirectoryPageResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::directory_page(request).map_err(Into::into)
    }

    /// Read complete membership after the Root status endpoint authenticates its controller.
    pub fn controller_component_directory_page(
        request: ComponentDirectoryPageRequest,
    ) -> Result<ComponentDirectoryPageResponse, canic_core::dto::error::Error> {
        crate::workflow::component_registry::controller_directory_page(request).map_err(Into::into)
    }

    pub async fn prepare_fleet_activation()
    -> Result<FleetActivationStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::runtime::fleet_activation::prepare_root()
            .await
            .map_err(Into::into)
    }

    pub async fn resume_fleet_activation(
        request: FleetActivationResumeRequest,
    ) -> Result<FleetActivationTransition, canic_core::dto::error::Error> {
        Box::pin(crate::workflow::runtime::fleet_activation::resume_root(
            request,
        ))
        .await
        .map_err(Into::into)
    }

    /// Delegate root post-upgrade runtime restore to the current core implementation.
    #[must_use]
    pub fn post_upgrade_root_canister_before_bootstrap(
        embedded_release_build_id: Option<&str>,
        runtime_authority: RoleRuntimeAuthority,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) -> bool {
        crate::runtime::root_funding::register();
        let active =
            canic_core::api::lifecycle::root::LifecycleApi::post_upgrade_root_canister_before_bootstrap(
                embedded_release_build_id,
                runtime_authority,
                config,
                config_source,
                config_path,
            );
        crate::workflow::component_registry::caller_authority::restore().unwrap_or_else(|error| {
            ic_cdk::trap(format!("Root caller publication restore failed: {error}"))
        });
        crate::workflow::canister_pool::declare();
        crate::workflow::component_provisioning::resume_after_restart().unwrap_or_else(|error| {
            ic_cdk::trap(format!(
                "Root Component provisioning resume failed: {error}"
            ))
        });
        if active {
            crate::workflow::canister_pool::start().unwrap_or_else(|error| {
                ic_cdk::trap(format!("Canister pool maintenance start failed: {error}"))
            });
        }
        active
    }

    /// Delegate root post-upgrade bootstrap scheduling to the current core implementation.
    pub fn schedule_post_upgrade_root_bootstrap() {
        LifecycleMetricsApi::record_bootstrap(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Scheduled,
        );

        canic_core::api::timer::TimerApi::defer_lifecycle_required(
            Duration::ZERO,
            "canic:bootstrap:post_upgrade_root_canister",
            async {
                crate::workflow::bootstrap::root::bootstrap_post_upgrade_root_canister().await;
            },
        );
    }
}
