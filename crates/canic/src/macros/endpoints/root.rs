//! Module: macros::endpoints::root
//!
//! Responsibility: emit root-canister endpoint macros for control and authority surfaces.
//! Does not own: root state, pool policy, auth proof issuance, or wasm-store workflows.
//! Boundary: exposes facade macros that delegate immediately to core/control-plane APIs.

/// Emit the Fleet Subnet Root's typed control-plane updates.
#[macro_export]
macro_rules! canic_emit_root_command_endpoint {
    () => {
        $crate::__canic_emit_root_capability_endpoints!();
        $crate::__canic_emit_root_observation_updates!();
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::application_checkpoint::BEGIN, request = ::canic::protocol::application_checkpoint::LIMITS, response = ::canic::protocol::application_checkpoint::LIMITS))]
        fn canic_root_checkpoint_begin(request: ::canic::dto::application_checkpoint::RootCheckpointBeginRequest) -> Result<::canic::dto::application_checkpoint::RootCheckpointStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::application_checkpoint::RootCheckpointApi::begin(request)
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::application_checkpoint::ADVANCE, request = ::canic::protocol::application_checkpoint::LIMITS, response = ::canic::protocol::application_checkpoint::LIMITS))]
        async fn canic_root_checkpoint_advance(request: ::canic::dto::application_checkpoint::RootCheckpointRequest) -> Result<::canic::dto::application_checkpoint::RootCheckpointStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::application_checkpoint::RootCheckpointApi::advance(request).await
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::application_checkpoint::RELEASE, request = ::canic::protocol::application_checkpoint::LIMITS, response = ::canic::protocol::application_checkpoint::LIMITS))]
        fn canic_root_checkpoint_release(request: ::canic::dto::application_checkpoint::RootCheckpointRequest) -> Result<::canic::dto::application_checkpoint::RootCheckpointStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::application_checkpoint::RootCheckpointApi::release(request)
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::component_initialization::METHOD_ID, request = ::canic::protocol::component_initialization::REQUEST_LIMITS, response = ::canic::protocol::component_initialization::RESPONSE_LIMITS))]
        fn canic_root_bind_component_initialization(request: ::canic::dto::component_initialization::RootComponentInitializationRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_bind_component_initialization", true)?;
            let operation_id = request.operation_id;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::bind_component_initialization(request.into())?;
            Ok(::canic::dto::role::OperationReceipt { operation_id })
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_activation::prepare::METHOD_ID, request = ::canic::protocol::root_activation::REQUEST_LIMITS, response = ::canic::protocol::root_activation::RESPONSE_LIMITS))]
        async fn canic_root_prepare_activation() -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_prepare_activation", true)?;
            let response = $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_fleet_activation().await?;
            Ok(::canic::dto::role::OperationReceipt { operation_id: response.identity.operation_id })
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_activation::resume::METHOD_ID, request = ::canic::protocol::root_activation::REQUEST_LIMITS, response = ::canic::protocol::root_activation::RESPONSE_LIMITS))]
        async fn canic_root_resume_activation(request: ::canic::dto::fleet_activation::FleetActivationResumeRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_resume_activation", true)?;
            let operation_id = request.operation_id;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::resume_fleet_activation(request).await?;
            Ok(::canic::dto::role::OperationReceipt { operation_id })
        }

        #[$crate::canic_update(public, wire(id = ::canic::protocol::funding_control::root_accept::METHOD_ID, request = ::canic::protocol::funding_control::REQUEST_LIMITS, response = ::canic::protocol::funding_control::RESPONSE_LIMITS))]
        fn canic_root_accept_funding(request: ::canic::dto::funding_control::AcceptanceRequest) -> Result<::canic::dto::funding_control::AcceptanceReceipt, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_root_funding_caller(caller)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_accept_funding", true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::accept_root_funding(request.into()).map(Into::into)
        }
        #[$crate::canic_update(public, wire(id = ::canic::protocol::funding_control::root_prepare::METHOD_ID, request = ::canic::protocol::funding_control::REQUEST_LIMITS, response = ::canic::protocol::funding_control::RESPONSE_LIMITS))]
        fn canic_root_prepare_funding_policy_rotation(request: ::canic::dto::funding_control::PrepareRequest) -> Result<::canic::dto::funding_control::RotationReceipt, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_root_funding_caller(caller)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_prepare_funding_policy_rotation", false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_root_funding_policy_rotation(request.into()).map(Into::into)
        }
        #[$crate::canic_update(public, wire(id = ::canic::protocol::funding_control::root_activate::METHOD_ID, request = ::canic::protocol::funding_control::REQUEST_LIMITS, response = ::canic::protocol::funding_control::RESPONSE_LIMITS))]
        async fn canic_root_activate_funding_policy_rotation(request: ::canic::dto::funding_control::ActivateRequest) -> Result<::canic::dto::funding_control::RotationReceipt, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_root_funding_caller(caller)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_activate_funding_policy_rotation", false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_root_funding_policy_rotation(request.into()).await.map(Into::into)
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::cycle_refill::preview::METHOD_ID, request = ::canic::protocol::cycle_refill::REQUEST_LIMITS, response = ::canic::protocol::cycle_refill::RESPONSE_LIMITS))]
        async fn canic_root_preview_cycle_refill(request: ::canic::dto::icp_refill::CycleRefillInput) -> Result<::canic::dto::icp_refill::IcpRefillDryRun, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_preview_cycle_refill", false)?;

                    let response = $crate::__internal::core::api::icp_refill::IcpRefillApi::refill(
                        ::canic::dto::icp_refill::IcpRefillRequest {
                            operation_id: request.operation_id,
                            source_subaccount: request.source_subaccount,
                            amount_e8s: request.amount_e8s,
                            dry_run: true,
                        },
                    )
                    .await?;
                    match response {
                        ::canic::dto::icp_refill::IcpRefillEndpointResponse::DryRun(response) => {
                            Ok(response)
                        }
                        _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    }

        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::cycle_refill::execute::METHOD_ID, request = ::canic::protocol::cycle_refill::REQUEST_LIMITS, response = ::canic::protocol::cycle_refill::RESPONSE_LIMITS))]
        async fn canic_root_refill_cycles(request: ::canic::dto::icp_refill::CycleRefillInput) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_refill_cycles", false)?;

                    let operation_id = request.operation_id;
                    let response = $crate::__internal::core::api::icp_refill::IcpRefillApi::refill(
                        ::canic::dto::icp_refill::IcpRefillRequest {
                            operation_id,
                            source_subaccount: request.source_subaccount,
                            amount_e8s: request.amount_e8s,
                            dry_run: false,
                        },
                    )
                    .await?;
                    if !matches!(
                        response,
                        ::canic::dto::icp_refill::IcpRefillEndpointResponse::Refill(_)
                    ) {
                        return Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into());
                    }
                    Ok(::canic::dto::role::OperationReceipt { operation_id })

        }

        #[$crate::canic_update(public, wire(id = ::canic::protocol::component_control::child::METHOD_ID, request = ::canic::protocol::component_control::REQUEST_LIMITS, response = ::canic::protocol::component_control::RESPONSE_LIMITS))]
        async fn canic_root_provision_child(request: ::canic::dto::component_registry::RootComponentChildAllocationRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {

            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_component_child_caller(&request, $crate::__internal::cdk::api::msg_caller().into())?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_provision_child", false)?;

                    let operation_id = request.operation_id;
                    let component = request.component;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reserve_component_child(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_child_allocation(component, operation_id);
                    Ok(::canic::dto::role::OperationReceipt { operation_id })

        }
        #[$crate::canic_update(public, wire(id = ::canic::protocol::component_control::peer::METHOD_ID, request = ::canic::protocol::component_control::REQUEST_LIMITS, response = ::canic::protocol::component_control::RESPONSE_LIMITS))]
        async fn canic_root_provision_peer(request: ::canic::dto::component_control::PeerRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            let request: ::canic::dto::component_registry::RootPeerComponentAllocationRequest = request.into();
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_peer_component_allocation_caller(&request, $crate::__internal::cdk::api::msg_caller().into())?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_provision_peer", false)?;

                    let operation_id = request.operation_id;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reserve_peer_component_allocation(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_allocation(operation_id);
                    Ok(::canic::dto::role::OperationReceipt { operation_id })

        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::component_control::remove::METHOD_ID, request = ::canic::protocol::component_control::REQUEST_LIMITS, response = ::canic::protocol::component_control::RESPONSE_LIMITS))]
        async fn canic_root_remove_component(request: ::canic::dto::component_registry::RootComponentDrainingRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {

            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_remove_component", false)?;

                    let operation_id = request.operation_id;
                    let component = request.component;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::begin_component_draining(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_removal(component, operation_id);
                    Ok(::canic::dto::role::OperationReceipt { operation_id })

        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::component_control::subtree::METHOD_ID, request = ::canic::protocol::component_control::REQUEST_LIMITS, response = ::canic::protocol::component_control::RESPONSE_LIMITS))]
        async fn canic_root_remove_subtree(request: ::canic::dto::component_control::SubtreeRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            let request: ::canic::dto::component_registry::RootComponentSubtreeRemovalRequest = request.into();

            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_remove_subtree", false)?;

                    let operation_id = request.operation_id;
                    let component = request.component;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::begin_component_subtree_removal(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_subtree_removal(component, operation_id);
                    Ok(::canic::dto::role::OperationReceipt { operation_id })

        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::pool_control::import::METHOD_ID, request = ::canic::protocol::pool_control::REQUEST_LIMITS, response = ::canic::protocol::pool_control::RESPONSE_LIMITS))]
        async fn canic_root_import_canister(request: ::canic::dto::pool_control::ImportRequest) -> Result<::canic::dto::pool::PoolImportResponse, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_import_canister", true)?;
                    let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(
                        ::canic::dto::pool::PoolAdminCommand::Import {
                            canister_id: request.canister_id.into(),
                        },
                    )
                    .await?;
                    let response = match response {
                        ::canic::dto::pool::PoolAdminResponse::Imported { canister_id } => {
                            ::canic::dto::pool::PoolImportResponse::Imported { canister_id: canister_id.into() }
                        }
                        ::canic::dto::pool::PoolAdminResponse::ResetFailed {
                            canister_id,
                            reason,
                        } => ::canic::dto::pool::PoolImportResponse::ResetFailed {
                            canister_id: canister_id.into(),
                            reason,
                        },
                        _ => return Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    };
                    Ok(response)
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::pool_control::handoff::METHOD_ID, request = ::canic::protocol::pool_control::REQUEST_LIMITS, response = ::canic::protocol::pool_control::RESPONSE_LIMITS))]
        async fn canic_root_handoff_pool_canister(request: ::canic::dto::pool_control::HandoffRequest) -> Result<::canic::dto::pool_control::HandoffResponse, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_handoff_pool_canister", true)?;
            let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(::canic::dto::pool::PoolAdminCommand::Handoff { canister_id: request.canister_id.into(), recipient: request.recipient.into() }).await?;
            match response {
                ::canic::dto::pool::PoolAdminResponse::HandedOff { canister_id, recipient } => Ok(::canic::dto::pool_control::HandoffResponse { canister_id: canister_id.into(), recipient: recipient.into() }),
                _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
            }
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_store_control::adopt::METHOD_ID, request = ::canic::protocol::root_store_control::REQUEST_LIMITS, response = ::canic::protocol::root_store_control::RESPONSE_LIMITS))]
        async fn canic_root_adopt_store(request: ::canic::dto::root_store_control::AdoptRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_adopt_store", true)?;
            let request: ::canic::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest = request.into();

                    let operation_id = request.operation_id;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::adopt_fleet_subnet_wasm_store(request).await?;
                    Ok(::canic::dto::role::OperationReceipt { operation_id })

        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_store_control::bootstrap::METHOD_ID, request = ::canic::protocol::root_store_control::REQUEST_LIMITS, response = ::canic::protocol::root_store_control::RESPONSE_LIMITS))]
        async fn canic_root_bootstrap_store(request: ::canic::dto::root_store::RootStoreBootstrapRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_bootstrap_store", true)?;

                    let operation_id = request.operation_id;
                    ::canic::api::canister::template::WasmStoreBootstrapApi::bootstrap_root_store(request).await?;
                    Ok(::canic::dto::role::OperationReceipt { operation_id })

        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_store_control::prepare::METHOD_ID, request = ::canic::protocol::root_store_control::REQUEST_LIMITS, response = ::canic::protocol::root_store_control::RESPONSE_LIMITS))]
        async fn canic_root_prepare_store_fixture(request: ::canic::dto::root_store::RootStoreFixturePrepareRequest) -> Result<Result<::canic::dto::fixture_provisioning::FixtureSourceStatus, ::canic::dto::fixture_provisioning::FixtureStoreError>, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_prepare_store_fixture", true)?;

                    ::canic::api::canister::template::WasmStoreBootstrapApi::prepare_root_store_fixture(request)
                        .await

        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_store_control::publish::METHOD_ID, request = ::canic::protocol::root_store_control::REQUEST_LIMITS, response = ::canic::protocol::root_store_control::RESPONSE_LIMITS))]
        async fn canic_root_publish_release_set() -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_publish_release_set", true)?;
            ::canic::api::canister::template::WasmStorePublicationApi::admin(::canic::dto::template::WasmStoreAdminCommand::PublishActiveReleaseSet).await.map(|::canic::dto::template::WasmStoreAdminResponse::PublishedActiveReleaseSet| ())
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_control::synchronize::METHOD_ID, request = ::canic::protocol::registry_control::REQUEST_LIMITS, response = ::canic::protocol::registry_control::RESPONSE_LIMITS))]
        async fn canic_root_synchronize_registry(request: ::canic::dto::registry_control::SynchronizeRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_synchronize_registry", true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::accept_fleet_registry_synchronization(request.into()).await
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::pool_control::maintain::METHOD_ID, request = ::canic::protocol::pool_control::REQUEST_LIMITS, response = ::canic::protocol::pool_control::RESPONSE_LIMITS))]
        async fn canic_root_maintain_pool() -> Result<::canic::dto::pool_control::MaintenanceResponse, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_maintain_pool", true)?;

                    let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(
                        ::canic::dto::pool::PoolAdminCommand::Maintain,
                    )
                    .await?;
                    let response = match response {
                        ::canic::dto::pool::PoolAdminResponse::Maintained => {
                            ::canic::dto::pool_control::MaintenanceResponse::Maintained
                        }
                        ::canic::dto::pool::PoolAdminResponse::MaintenancePaused { reason } => {
                            ::canic::dto::pool_control::MaintenanceResponse::MaintenancePaused { reason }
                        }
                        ::canic::dto::pool::PoolAdminResponse::Created { canister_id } => {
                            ::canic::dto::pool_control::MaintenanceResponse::Created { canister_id: canister_id.into() }
                        }
                        ::canic::dto::pool::PoolAdminResponse::RefillWaitingForCycles {
                            available,
                            attempt_count,
                            creation_amount,
                            execution_margin,
                            last_attempt_at_ns,
                            ledger_fee,
                            readiness_floor,
                            required,
                            retry_at_ns,
                            shortfall,
                        } => ::canic::dto::pool_control::MaintenanceResponse::RefillWaitingForCycles {
                            available,
                            attempt_count,
                            creation_amount,
                            execution_margin,
                            last_attempt_at_ns,
                            ledger_fee,
                            readiness_floor,
                            required,
                            retry_at_ns,
                            shortfall,
                        },
                        ::canic::dto::pool::PoolAdminResponse::RefillPending {
                            operation_id,
                            uncertain_result,
                        } => ::canic::dto::pool_control::MaintenanceResponse::RefillPending {
                            operation_id,
                            uncertain_result,
                        },
                        ::canic::dto::pool::PoolAdminResponse::RefillBlocked {
                            operation_id,
                            failure,
                        } => ::canic::dto::pool_control::MaintenanceResponse::RefillBlocked {
                            operation_id,
                            failure,
                        },
                        ::canic::dto::pool::PoolAdminResponse::ResetReady { canister_id } => {
                            ::canic::dto::pool_control::MaintenanceResponse::ResetReady { canister_id: canister_id.into() }
                        }
                        ::canic::dto::pool::PoolAdminResponse::ResetFailed {
                            canister_id,
                            reason,
                        } => ::canic::dto::pool_control::MaintenanceResponse::ResetFailed {
                            canister_id: canister_id.into(),
                            reason,
                        },
                        _ => return Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    };
                    Ok(response)

        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::component_control::provision::METHOD_ID, request = ::canic::protocol::component_control::REQUEST_LIMITS, response = ::canic::protocol::component_control::RESPONSE_LIMITS))]
        async fn canic_root_provision_component(request: ::canic::dto::component_registry::RootComponentAllocationRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_provision_component", true)?;
            let operation_id = request.operation_id;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reserve_component_allocation(request).await?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_allocation(operation_id);
            Ok(::canic::dto::role::OperationReceipt { operation_id })
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::component_control::prepare::METHOD_ID, request = ::canic::protocol::component_control::REQUEST_LIMITS, response = ::canic::protocol::component_control::RESPONSE_LIMITS))]
        async fn canic_root_prepare_component_registry(request: ::canic::dto::component_control::PreparationRequest) -> Result<::canic::dto::component_control::RegistryStatus, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_prepare_component_registry", true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_component_registry(request.into()).await.map(Into::into)
        }

        #[$crate::canic_update(public, wire(id = ::canic::protocol::retirement_control::root_remove::METHOD_ID, request = ::canic::protocol::retirement_control::REQUEST_LIMITS, response = ::canic::protocol::retirement_control::RESPONSE_LIMITS))]
        fn canic_root_remove_root(request: ::canic::dto::retirement_control::RootRemovalRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_fleet_subnet_root_removal_caller(caller, $crate::__internal::cdk::api::is_controller(&caller.into()))?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_remove_root", false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::accept_fleet_subnet_root_removal(request.into())
        }

        #[$crate::canic_update(public, wire(id = ::canic::protocol::batch_control::accept::METHOD_ID, request = ::canic::protocol::batch_control::ROOT_LIMITS, response = ::canic::protocol::batch_control::RESPONSE_LIMITS))]
        async fn canic_root_provision_components(request: ::canic::dto::batch_control::RootRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::authorize_coordinator_caller(caller)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_provision_components", true)?;
            $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::accept(request.into()).await
        }
        #[$crate::canic_update(public, wire(id = ::canic::protocol::batch_control::synchronize::METHOD_ID, request = ::canic::protocol::batch_control::SYNCHRONIZE_LIMITS, response = ::canic::protocol::batch_control::RESPONSE_LIMITS))]
        async fn canic_root_synchronize_component_directories(request: ::canic::dto::batch_control::SynchronizeRequest) -> Result<::canic::dto::batch_control::Synchronization, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::authorize_coordinator_caller(caller)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_synchronize_component_directories", true)?;
            $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::synchronize_directories(request.into()).await.map(Into::into)
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::inspection_history::METHOD_ID, request = ::canic::protocol::inspection_history::REQUEST_LIMITS, response = ::canic::protocol::inspection_history::RESPONSE_LIMITS))]
        async fn canic_root_inspect_history(request: ::canic::dto::inspection_history::Request) -> Result<::canic::dto::canister::CanisterHistoryResponse, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(true)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_inspect_history", true)?;
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_history(request.canister_id.into()).await
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::import_control::reserve::METHOD_ID, request = ::canic::protocol::import_control::REQUEST_LIMITS, response = ::canic::protocol::import_control::RESPONSE_LIMITS))]
        async fn canic_root_import_reserve(request: ::canic::dto::pool_census::Reservation) -> Result<::canic::dto::pool_census::ImportStatus, ::canic::Error> {
            let command = ::canic::dto::pool_import::PoolImportCommand::Reserve(Box::new(request.into()));
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            let operator = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_operator(&command)?;
            if caller != operator { return Err($crate::__internal::core::control_plane_support::error::InternalError::forbidden().into()); }
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_import_reserve", true)?;
            $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_command(command).await.map(Into::into)
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::import_control::advance::METHOD_ID, request = ::canic::protocol::import_control::REQUEST_LIMITS, response = ::canic::protocol::import_control::RESPONSE_LIMITS))]
        async fn canic_root_import_advance(request: ::canic::dto::import_control::AdvanceRequest) -> Result<::canic::dto::pool_census::ImportStatus, ::canic::Error> {
            let command = ::canic::dto::pool_import::PoolImportCommand::Advance { identity: request.identity, canister_id: request.canister_id.into() };
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            let operator = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_operator(&command)?;
            if caller != operator { return Err($crate::__internal::core::control_plane_support::error::InternalError::forbidden().into()); }
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_import_advance", true)?;
            $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_command(command).await.map(Into::into)
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::import_control::settle::METHOD_ID, request = ::canic::protocol::import_control::REQUEST_LIMITS, response = ::canic::protocol::import_control::RESPONSE_LIMITS))]
        async fn canic_root_import_settle(request: ::canic::dto::pool_import::PoolImportIdentity) -> Result<::canic::dto::pool_census::ImportStatus, ::canic::Error> {
            let command = ::canic::dto::pool_import::PoolImportCommand::Settle(request);
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            let operator = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_operator(&command)?;
            if caller != operator { return Err($crate::__internal::core::control_plane_support::error::InternalError::forbidden().into()); }
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_import_settle", true)?;
            $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_command(command).await.map(Into::into)
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::import_control::release::METHOD_ID, request = ::canic::protocol::import_control::REQUEST_LIMITS, response = ::canic::protocol::import_control::RESPONSE_LIMITS))]
        async fn canic_root_import_release(request: ::canic::dto::import_control::ReleaseRequest) -> Result<::canic::dto::pool_census::ImportStatus, ::canic::Error> {
            let command = ::canic::dto::pool_import::PoolImportCommand::Release { identity: request.identity, publication_sha256: request.publication_sha256 };
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            let operator = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_operator(&command)?;
            if caller != operator { return Err($crate::__internal::core::control_plane_support::error::InternalError::forbidden().into()); }
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_import_release", true)?;
            $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_command(command).await.map(Into::into)
        }

        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::inspection::METHOD_ID, request = ::canic::protocol::inspection::REQUEST_LIMITS, response = ::canic::protocol::inspection::RESPONSE_LIMITS))]
        async fn canic_root_inspect_canister(request: ::canic::dto::inspection::Request) -> Result<::canic::dto::inspection::Response, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(true)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_inspect_canister", true)?;
            let outcome = $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_inspection(request.canister_id.into()).await?;
            outcome.try_into().map_err(|_| ::canic::Error::from_registered($crate::__internal::contracts::diagnostics::codes::REQUEST_CAPACITY))
        }

    };
}

/// Emit the Fleet Subnet Root's role-owned status query.
#[macro_export]
macro_rules! canic_emit_root_status_endpoint {
    () => {
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::application_checkpoint::STATUS, request = ::canic::protocol::application_checkpoint::LIMITS, response = ::canic::protocol::application_checkpoint::LIMITS))]
        fn canic_root_checkpoint_status() -> Result<Option<::canic::dto::application_checkpoint::RootCheckpointStatus>, ::canic::Error> {
            $crate::__internal::control_plane::api::application_checkpoint::RootCheckpointApi::status()
        }
        // An update permits ordinary inter-canister discovery. It does not mutate
        // membership or issue lasting authorization for effects at another receiver.
        #[$crate::canic_update(requires(any(
            caller::is_controller(),
            custom(::canic::__internal::control_plane::api::component_auth::ActiveComponentMemberPredicate)
        )), wire(id = ::canic::protocol::component_read::membership::METHOD_ID, request = ::canic::protocol::component_read::REQUEST_LIMITS, response = ::canic::protocol::component_read::RESPONSE_LIMITS))]
        fn canic_root_membership(
            request: ::canic::dto::component_registry::RootMembershipRequest,
        ) -> Result<::canic::dto::component_registry::RootMembershipResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::component_auth::RootComponentMembershipApi::lookup(request)
        }


        $crate::__canic_emit_public_health_endpoint!();
        #[cfg(canic_capability_observability_metrics)]
        $crate::__canic_emit_public_metrics_endpoint!();

        #[cfg(canic_capability_observability_history)]
        $crate::__canic_emit_public_history_endpoint!();

        #[$crate::canic_query(public, wire(id = ::canic::protocol::public_overview::METHOD_ID, request = ::canic::protocol::public_overview::REQUEST_LIMITS, response = ::canic::protocol::public_overview::RESPONSE_LIMITS))]
        fn canic_public_overview() -> Result<::canic::dto::role::RoleOverviewResponse, ::canic::Error> {
            let capabilities = $crate::__canic_compiled_role_capabilities!();
            Ok(
                $crate::__internal::core::api::role::RoleOverviewApi::overview(
                    $crate::__internal::contracts::ids::CanisterRole::from(env!(
                        "CANIC_CANISTER_ROLE"
                    )),
                    &capabilities,
                    $crate::__canic_protocol_profile_digest!(),
                    $crate::__internal::core::api::metadata::CanicMetadataApi::metadata_for(
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION"),
                        env!("CARGO_PKG_DESCRIPTION"),
                        $crate::VERSION,
                        $crate::__internal::cdk::api::canister_version(),
                    ),
                    $crate::__internal::core::api::ready::ReadyApi::bootstrap_status(),
                ),
            )
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::public_children::METHOD_ID, request = ::canic::protocol::public_children::REQUEST_LIMITS, response = ::canic::protocol::public_children::RESPONSE_LIMITS))]
        fn canic_public_children(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_public_children", false)?;
            Ok($crate::__internal::core::api::topology::children::CanisterChildrenApi::page(page))
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::component_directory::MEMBER_PAGE_ID, request = ::canic::protocol::component_directory::REQUEST_LIMITS, response = ::canic::protocol::component_directory::RESPONSE_LIMITS))]
        fn canic_component_directory_page(request: ::canic::dto::component_directory::ComponentDirectoryPageRequest) -> Result<::canic::dto::component_directory::ComponentDirectoryPageResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_directory_page(request)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::component_directory::HEAD_ID, request = ::canic::protocol::component_directory::REQUEST_LIMITS, response = ::canic::protocol::component_directory::RESPONSE_LIMITS))]
        fn canic_root_directory_head(request: ::canic::dto::component_registry::ComponentDirectoryHeadRequest) -> Result<::canic::dto::component_directory::ComponentDirectoryHead, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_directory_head", true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_directory_head(request)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::component_directory::PAGE_ID, request = ::canic::protocol::component_directory::REQUEST_LIMITS, response = ::canic::protocol::component_directory::RESPONSE_LIMITS))]
        fn canic_root_directory_page(request: ::canic::dto::component_directory::ComponentDirectoryPageRequest) -> Result<::canic::dto::component_directory::ComponentDirectoryPageResponse, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_directory_page", true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::controller_component_directory_page(request)
        }

        $crate::__canic_emit_cycle_balance_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_cycle_balance", false)?;]);

        $crate::__canic_emit_protected_health_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_health", false)?;]);
        $crate::__canic_emit_protected_readiness_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_readiness", false)?;]);
        #[cfg(canic_capability_observability_diagnostics)]
        $crate::__canic_emit_protected_runtime_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_runtime", false)?;]);

        $crate::__canic_emit_child_funding_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_child_funding", true)?;]);

        #[cfg(canic_capability_observability_logs)]
        $crate::__canic_emit_protected_logs_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_logs", false)?;]);

        #[cfg(canic_capability_observability_metrics)]
        $crate::__canic_emit_protected_metrics_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_metrics", false)?;]);

        #[cfg(canic_capability_observability_diagnostics)]
        $crate::__canic_emit_protected_memory_allocations_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_memory_allocations", false)?;]);

        #[cfg(canic_capability_observability_history)]
        $crate::__canic_emit_cycle_history_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_cycle_history", false)?;]);

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::inspection_reserve::METHOD_ID, request = ::canic::protocol::inspection_reserve::REQUEST_LIMITS, response = ::canic::protocol::inspection_reserve::RESPONSE_LIMITS))]
        fn canic_root_inspection_reserve(target: $crate::__internal::ic_platform::Principal) -> Result<::canic::dto::inspection_reserve::InspectionReserve, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_inspection_reserve", true)?;
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_inspection_reserve(target.into()).map(Into::into)
        }

        #[cfg(canic_capability_role_attestation_signer)]
        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::role_attestation::prepare::METHOD_ID, request = ::canic::protocol::role_attestation::REQUEST_LIMITS, response = ::canic::protocol::role_attestation::RESPONSE_LIMITS))]
        fn canic_root_prepare_role_attestation(request: ::canic::dto::role_attestation::RoleAttestationRequest) -> Result<::canic::dto::role_attestation::RoleAttestationPrepareResponse, ::canic::Error> {
            use $crate::__internal::core::access::expr::AccessCheck as _;
            let call = ::canic::ids::EndpointCall { endpoint: ::canic::ids::EndpointId::new(::canic::protocol::role_attestation::prepare::METHOD), kind: ::canic::ids::EndpointCallKind::Update };
            $crate::__internal::core::access::expr::eval_default_fleet_guard($crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates, call).map_err(::canic::Error::from)?;
            let context = $crate::__internal::core::access::expr::AccessContext { caller: $crate::__internal::cdk::api::msg_caller().into(), call };
            $crate::__internal::control_plane::api::component_auth::ActiveComponentMemberPredicate.eval(&context).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_prepare_role_attestation", false)?;
            $crate::__internal::control_plane::api::component_auth::ComponentAuthApi::prepare_role_attestation(request.into()).map(Into::into)
        }

        #[cfg(canic_capability_role_attestation_signer)]
        #[$crate::canic_query(public, wire(id = ::canic::protocol::role_attestation::get::METHOD_ID, request = ::canic::protocol::role_attestation::REQUEST_LIMITS, response = ::canic::protocol::role_attestation::RESPONSE_LIMITS))]
        fn canic_root_role_attestation(request: ::canic::dto::auth::RoleAttestationGetRequest) -> Result<::canic::dto::role_attestation::SignedRoleAttestation, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_role_attestation", false)?;
            $crate::__internal::control_plane::api::component_auth::ComponentAuthApi::get_role_attestation(request).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::component_read::child_allocation::METHOD_ID, request = ::canic::protocol::component_read::REQUEST_LIMITS, response = ::canic::protocol::component_read::RESPONSE_LIMITS))]
        fn canic_root_child_allocation(operation_id: [u8; 32]) -> Result<::canic::dto::component_read::ChildAllocation, ::canic::Error> {
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::component_read::child_allocation::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_component_child_provisioning_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::component_read::allocation::METHOD_ID, request = ::canic::protocol::component_read::REQUEST_LIMITS, response = ::canic::protocol::component_read::RESPONSE_LIMITS))]
        fn canic_root_component_allocation(operation_id: [u8; 32]) -> Result<::canic::dto::component_read::ComponentAllocation, ::canic::Error> {
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::component_read::allocation::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_component_allocation_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::subtree_read::METHOD_ID, request = ::canic::protocol::subtree_read::REQUEST_LIMITS, response = ::canic::protocol::subtree_read::RESPONSE_LIMITS))]
        fn canic_root_subtree_removal(operation_id: [u8; 32]) -> Result<::canic::dto::subtree_read::Status, ::canic::Error> {
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::subtree_read::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_subtree_removal_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::component_removal_read::METHOD_ID, request = ::canic::protocol::component_removal_read::REQUEST_LIMITS, response = ::canic::protocol::component_removal_read::RESPONSE_LIMITS))]
        fn canic_root_component_removal(operation_id: [u8; 32]) -> Result<::canic::dto::component_removal_read::Status, ::canic::Error> {
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::component_removal_read::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_component_removal_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::component_read::provisioning::METHOD_ID, request = ::canic::protocol::component_read::REQUEST_LIMITS, response = ::canic::protocol::component_read::RESPONSE_LIMITS))]
        fn canic_root_component_provisioning(operation_id: [u8; 32]) -> Result<::canic::dto::component_read::Provisioning, ::canic::Error> {
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::component_read::provisioning::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_component_provisioning_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::root_store_control::adoption::METHOD_ID, request = ::canic::protocol::root_store_control::REQUEST_LIMITS, response = ::canic::protocol::root_store_control::RESPONSE_LIMITS))]
        fn canic_root_store_adoption(operation_id: [u8; 32]) -> Result<::canic::dto::root_store_control::Adoption, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::root_store_control::adoption::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_store_adoption_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::root_store_control::bootstrap_status::METHOD_ID, request = ::canic::protocol::root_store_control::REQUEST_LIMITS, response = ::canic::protocol::root_store_control::RESPONSE_LIMITS))]
        fn canic_root_store_bootstrap(operation_id: [u8; 32]) -> Result<::canic::dto::root_store_control::Bootstrap, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::root_store_control::bootstrap_status::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_store_bootstrap_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::root_activation::status::METHOD_ID, request = ::canic::protocol::root_activation::REQUEST_LIMITS, response = ::canic::protocol::root_activation::RESPONSE_LIMITS))]
        fn canic_root_activation_status(operation_id: [u8; 32]) -> Result<::canic::dto::runtime_reporting::FleetActivationStatusResponse, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::root_activation::status::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_activation_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::cycle_refill::status::METHOD_ID, request = ::canic::protocol::cycle_refill::REQUEST_LIMITS, response = ::canic::protocol::cycle_refill::RESPONSE_LIMITS))]
        fn canic_root_cycle_refill_status(operation_id: [u8; 32]) -> Result<::canic::dto::funding_read::RefillResult, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::cycle_refill::status::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_refill_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).and_then(|value| value.try_into().map_err(|_| ::canic::Error::from_registered(::canic::diagnostics::codes::REQUEST_CAPACITY)))
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::registry_control::synchronization_status::METHOD_ID, request = ::canic::protocol::registry_control::REQUEST_LIMITS, response = ::canic::protocol::registry_control::RESPONSE_LIMITS))]
        fn canic_root_registry_synchronization(operation_id: [u8; 32]) -> Result<::canic::dto::registry_control::SynchronizationStatus, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::registry_control::synchronization_status::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_registry_synchronization_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(public, wire(id = ::canic::protocol::root_removal_read::METHOD_ID, request = ::canic::protocol::root_removal_read::REQUEST_LIMITS, response = ::canic::protocol::root_removal_read::RESPONSE_LIMITS))]
        fn canic_root_removal(operation_id: [u8; 32]) -> Result<::canic::dto::root_removal_read::Status, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::root_removal_read::METHOD, true)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_removal_status(operation_id, caller, $crate::__internal::cdk::api::is_controller(&caller.into())).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::root_config::METHOD_ID, request = ::canic::protocol::root_config::REQUEST_LIMITS, response = ::canic::protocol::root_config::RESPONSE_LIMITS))]
        fn canic_root_config() -> Result<::canic::dto::role::ConfigStatusResponse, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_config", false)?;
            $crate::__internal::core::api::config::ConfigApi::export_toml()
                .map(|toml| ::canic::dto::role::ConfigStatusResponse { toml })
        }

        $crate::__canic_emit_release_evidence_endpoints!(before = [], root_gate = $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed);

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_discovery::root_inventory::METHOD_ID, request = ::canic::protocol::registry_discovery::REQUEST_LIMITS, response = ::canic::protocol::registry_discovery::RESPONSE_LIMITS))]
        fn canic_root_inventory() -> Result<::canic::dto::fleet_snapshot::RootInventory, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_inventory", false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_canister_summary().map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::admission_control::root_status::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_root_admission(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::admission_control::RootStatus, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_admission", false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_admission_status(page).map(Into::into)
        }

        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::admission_control::root_prepare::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_root_admission_prepare(request: ::canic::dto::admission_control::PrepareRootRequest) -> Result<::canic::dto::admission_control::RootReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall { endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::admission_control::root_prepare::METHOD), kind: $crate::__internal::contracts::ids::EndpointCallKind::Update },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_root_admission_caller(caller)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_admission_prepare", false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_root_fleet_admission(request.into()).map(Into::into)
        }

        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::admission_control::root_activate::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_root_admission_activate(request: ::canic::dto::admission_control::ActivateRootRequest) -> Result<::canic::dto::admission_control::RootReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall { endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::admission_control::root_activate::METHOD), kind: $crate::__internal::contracts::ids::EndpointCallKind::Update },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_root_admission_caller(caller)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_admission_activate", false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_root_fleet_admission(request.into()).map(Into::into)
        }

        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::admission_control::root_open::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_root_admission_open(request: ::canic::dto::admission_control::OpenRootRequest) -> Result<::canic::dto::admission_control::RootReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall { endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::admission_control::root_open::METHOD), kind: $crate::__internal::contracts::ids::EndpointCallKind::Update },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_root_admission_caller(caller)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_admission_open", false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::open_root_fleet_admission(request.into()).map(Into::into)
        }

        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::pool_retry::refill::METHOD_ID, request = ::canic::protocol::pool_retry::REQUEST_LIMITS, response = ::canic::protocol::pool_retry::RESPONSE_LIMITS))]
        async fn canic_root_pool_retry_refill(_request: ()) -> Result<::canic::dto::pool_retry::RefillResponse, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall {
                    endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::pool_retry::refill::METHOD),
                    kind: $crate::__internal::contracts::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_controller(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_pool_retry_refill", true)?;
            let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(::canic::dto::pool::PoolAdminCommand::RetryRefill).await?;
            match response {
                ::canic::dto::pool::PoolAdminResponse::RefillRetryScheduled { previous_operation_id } => Ok(::canic::dto::pool_retry::RefillResponse { previous_operation_id }),
                _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
            }
        }

        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::pool_retry::reset::METHOD_ID, request = ::canic::protocol::pool_retry::REQUEST_LIMITS, response = ::canic::protocol::pool_retry::RESPONSE_LIMITS))]
        async fn canic_root_pool_retry_reset(request: ::canic::dto::pool_retry::ResetRequest) -> Result<::canic::dto::pool_retry::ResetResponse, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall {
                    endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::pool_retry::reset::METHOD),
                    kind: $crate::__internal::contracts::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_controller(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_pool_retry_reset", true)?;
            let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(request.into()).await?;
            match response {
                ::canic::dto::pool::PoolAdminResponse::ResetQueued { canister_id } => Ok(canister_id.into()),
                _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
            }
        }

        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::fleet_state_control::funding::METHOD_ID, request = ::canic::protocol::fleet_state_control::REQUEST_LIMITS, response = ::canic::protocol::fleet_state_control::RESPONSE_LIMITS))]
        async fn canic_root_set_cycles_funding(request: ::canic::dto::state::SetCyclesFundingRequest) -> Result<::canic::dto::fleet_state_control::CommandResult<bool>, ::canic::Error> {
            // State recovery must remain reachable while ordinary updates are fenced.
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_controller(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_set_cycles_funding", false)?;
            let response = $crate::__internal::control_plane::api::state::FleetStateApi::execute_command(::canic::dto::state::FleetCommand::SetCyclesFundingEnabled(request.enabled)).await?;
            match response.change {
                ::canic::dto::state::FleetCommandResponse::CyclesFundingEnabled(change) => Ok(::canic::dto::state::FleetStateCommandResult {
                    change, propagation: response.propagation, reconciliation_error: response.reconciliation_error,
                }.into()),
                _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
            }
        }

        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::fleet_state_control::status::METHOD_ID, request = ::canic::protocol::fleet_state_control::REQUEST_LIMITS, response = ::canic::protocol::fleet_state_control::RESPONSE_LIMITS))]
        async fn canic_root_set_fleet_status(request: ::canic::dto::state::SetFleetStatusRequest) -> Result<::canic::dto::fleet_state_control::CommandResult<::canic::dto::state::FleetStatus>, ::canic::Error> {
            // State recovery must remain reachable while ordinary updates are fenced.
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_controller(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_set_fleet_status", false)?;
            let response = $crate::__internal::control_plane::api::state::FleetStateApi::execute_command(::canic::dto::state::FleetCommand::SetStatus(request.status)).await?;
            match response.change {
                ::canic::dto::state::FleetCommandResponse::Status(change) => Ok(::canic::dto::state::FleetStateCommandResult {
                    change, propagation: response.propagation, reconciliation_error: response.reconciliation_error,
                }.into()),
                _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
            }
        }

        $crate::__canic_emit_authority_snapshot_endpoints!(
            before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_set_fleet_status", false)?;],
            prepare = $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_authority_snapshot,
            resume = $crate::__internal::control_plane::api::lifecycle::LifecycleApi::resume_authority_snapshot
        );

        $crate::__canic_emit_authority_restore_status_endpoint!(before = [$crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_authority_restore_status", false)?;]);

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_state::fleet_state::METHOD_ID, request = ::canic::protocol::protected_state::fleet_state::REQUEST_LIMITS, response = ::canic::protocol::protected_state::fleet_state::RESPONSE_LIMITS))]
        fn canic_root_fleet_state() -> Result<::canic::dto::state::FleetStateResponse, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_fleet_state", false)?;
            Ok($crate::__internal::core::api::state::FleetStateQuery::snapshot())
        }

        #[cfg(canic_capability_root_delegation)]
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::issuer_control::configure::METHOD_ID, request = ::canic::protocol::issuer_control::REQUEST_LIMITS, response = ::canic::protocol::issuer_control::RESPONSE_LIMITS))]
        fn canic_root_configure_issuer(request: ::canic::dto::issuer_control::RootIssuerConfigureRequest) -> Result<::canic::dto::issuer_control::RootIssuerConfigureResponse, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_configure_issuer", false)?;
            $crate::__internal::core::api::auth::AuthApi::configure_issuer_root(request.into()).map(Into::into)
        }

        #[cfg(canic_capability_root_delegation)]
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::issuer_control::public_key::METHOD_ID, request = ::canic::protocol::issuer_control::REQUEST_LIMITS, response = ::canic::protocol::issuer_control::RESPONSE_LIMITS))]
        async fn canic_root_chain_key_public_key(request: ::canic::dto::auth::RootChainKeyPublicKeyRequest) -> Result<Vec<u8>, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_chain_key_public_key", false)?;
            $crate::__internal::core::api::auth::AuthApi::root_chain_key_public_key(request).await
        }

        #[cfg(canic_capability_root_delegation)]
        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::issuer_control::proof::METHOD_ID, request = ::canic::protocol::issuer_control::REQUEST_LIMITS, response = ::canic::protocol::issuer_control::RESPONSE_LIMITS))]
        async fn canic_root_delegation_proof() -> Result<::canic::dto::issuer_control::RootDelegationProofBatchProof, ::canic::Error> {
            use $crate::__internal::core::access::expr::AccessCheck as _;
            let call = ::canic::ids::EndpointCall { endpoint: ::canic::ids::EndpointId::new(::canic::protocol::issuer_control::proof::METHOD), kind: ::canic::ids::EndpointCallKind::Update };
            $crate::__internal::core::access::expr::eval_default_fleet_guard($crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates, call).map_err(::canic::Error::from)?;
            let context = $crate::__internal::core::access::expr::AccessContext { caller: $crate::__internal::cdk::api::msg_caller().into(), call };
            $crate::__internal::control_plane::api::component_auth::ActiveComponentMemberPredicate.eval(&context).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed("canic_root_delegation_proof", false)?;
            $crate::__internal::core::api::auth::AuthApi::get_or_create_chain_key_delegation_proof_root().await.map(Into::into)
        }

        #[cfg(canic_capability_root_delegation)]
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::issuer_renewal::METHOD_ID, request = ::canic::protocol::issuer_renewal::REQUEST_LIMITS, response = ::canic::protocol::issuer_renewal::RESPONSE_LIMITS))]
        fn canic_root_issuer_renewal(request: ::canic::dto::issuer_renewal::RootIssuerRenewalStatusRequest) -> Result<::canic::dto::issuer_renewal::RootIssuerRenewalStatusResponse, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_issuer_renewal", false)?;
            $crate::__internal::core::api::auth::AuthApi::root_issuer_renewal_status_root(request.into()).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::root_store_reporting::overview::METHOD_ID, request = ::canic::protocol::root_store_reporting::REQUEST_LIMITS, response = ::canic::protocol::root_store_reporting::RESPONSE_LIMITS))]
        fn canic_root_store_overview() -> Result<::canic::dto::root_store_reporting::Overview, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_store_overview", true)?;
            ::canic::api::canister::template::WasmStorePublicationApi::overview().map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::provisioning_census::METHOD_ID, request = ::canic::protocol::provisioning_census::REQUEST_LIMITS, response = ::canic::protocol::provisioning_census::RESPONSE_LIMITS))]
        fn canic_root_provisioning_release(start_after: Option<::canic::dto::root::RootProvisioningReleaseKey>) -> Result<::canic::dto::provisioning_census::Page, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_provisioning_release", true)?;
            $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::release_status(start_after).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::pool_census::METHOD_ID, request = ::canic::protocol::pool_census::REQUEST_LIMITS, response = ::canic::protocol::pool_census::RESPONSE_LIMITS))]
        fn canic_root_pool_release() -> Result<::canic::dto::pool_census::Census, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_pool_release", true)?;
            $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::release_status().map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::pool_import_read::context::METHOD_ID, request = ::canic::protocol::pool_import_read::REQUEST_LIMITS, response = ::canic::protocol::pool_import_read::RESPONSE_LIMITS))]
        fn canic_root_pool_import_context() -> Result<::canic::dto::pool_import_read::Context, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_pool_import_context", true)?;
            $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_context().map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::pool_import_read::status::METHOD_ID, request = ::canic::protocol::pool_import_read::REQUEST_LIMITS, response = ::canic::protocol::pool_import_read::RESPONSE_LIMITS))]
        fn canic_root_pool_import_status(identity: ::canic::dto::pool_import::PoolImportIdentity) -> Result<::canic::dto::pool_census::ImportStatus, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed("canic_root_pool_import_status", true)?;
            $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_status(identity).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::root_authority::METHOD_ID, request = ::canic::protocol::root_authority::REQUEST_LIMITS, response = ::canic::protocol::root_authority::RESPONSE_LIMITS))]
        fn canic_root_authority() -> Result<::canic::dto::root_authority::Authority, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_authority().map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_read::status::METHOD_ID, request = ::canic::protocol::registry_read::REQUEST_LIMITS, response = ::canic::protocol::registry_read::RESPONSE_LIMITS))]
        fn canic_root_component_registry(request: ::canic::dto::component_control::PreparationRequest) -> Result<::canic::dto::component_control::RegistryStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::local_component_registry_status(request.into()).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_read::partition::METHOD_ID, request = ::canic::protocol::registry_read::REQUEST_LIMITS, response = ::canic::protocol::registry_read::RESPONSE_LIMITS))]
        fn canic_root_component_partition(request: ::canic::dto::component_registry::ComponentRegistryPartitionRequest) -> Result<::canic::dto::registry_read::Partition, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_registry_partition(request).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_read::active_partition::METHOD_ID, request = ::canic::protocol::registry_read::REQUEST_LIMITS, response = ::canic::protocol::registry_read::RESPONSE_LIMITS))]
        fn canic_root_component_active_partition(request: ::canic::dto::component_registry::ComponentRegistryActivePartitionRequest) -> Result<::canic::dto::registry_read::ActivePartition, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_registry_active_partition(request).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::pool_inventory::METHOD_ID, request = ::canic::protocol::pool_inventory::REQUEST_LIMITS, response = ::canic::protocol::pool_inventory::RESPONSE_LIMITS))]
        fn canic_root_pool(request: ::canic::dto::pool_inventory::Request) -> Result<::canic::dto::pool_inventory::Page, ::canic::Error> {
            $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::status(request.into()).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::funding_read::release::METHOD_ID, request = ::canic::protocol::funding_read::REQUEST_LIMITS, response = ::canic::protocol::funding_read::RESPONSE_LIMITS))]
        fn canic_root_funding_release(start_after: Option<u64>) -> Result<::canic::dto::funding_read::ReleasePage, ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_funding_release_status(start_after)?
                .try_into().map_err(|_| ::canic::Error::from_registered(::canic::diagnostics::codes::REQUEST_CAPACITY))
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::funding_read::root_status::METHOD_ID, request = ::canic::protocol::funding_read::REQUEST_LIMITS, response = ::canic::protocol::funding_read::RESPONSE_LIMITS))]
        fn canic_root_funding() -> Result<::canic::dto::funding_read::RootStatus, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_query_allowed(::canic::protocol::funding_read::root_status::METHOD, false)?;
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_funding_status()?
                .try_into().map_err(|_| ::canic::Error::from_registered(::canic::diagnostics::codes::REQUEST_CAPACITY))
        }
        // This read calls Store; keep it separate from ordinary Root status used by updates.
        #[$crate::canic_query(composite, requires(caller::is_controller()), wire(id = ::canic::protocol::root_store_reporting::fixture::METHOD_ID, request = ::canic::protocol::root_store_reporting::REQUEST_LIMITS, response = ::canic::protocol::root_store_reporting::RESPONSE_LIMITS))]
        async fn canic_root_fixture_status(
            content_id: [u8; 32],
        ) -> Result<Result<::canic::dto::fixture_provisioning::FixtureSourceStatus,
            ::canic::dto::fixture_provisioning::FixtureStoreError>, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::root_fixture_status(content_id).await
        }
    };
}
