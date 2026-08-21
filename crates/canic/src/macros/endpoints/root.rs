//! Module: macros::endpoints::root
//!
//! Responsibility: emit root-canister endpoint macros for control and authority surfaces.
//! Does not own: root state, pool policy, auth proof issuance, or wasm-store workflows.
//! Boundary: exposes facade macros that delegate immediately to core/control-plane APIs.

/// Emit the Fleet Subnet Root's role-owned command update.
#[macro_export]
macro_rules! canic_emit_root_command_endpoint {
    () => {
        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootCommand {
            AdoptStore(::canic::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest),
            BootstrapStore(::canic::dto::root_store::RootStoreBootstrapRequest),
            GetOrCreateDelegationProof,
            HandoffPoolCanister(::canic::dto::pool::PoolHandoffRequest),
            ImportPoolCanister(::canic::dto::pool::PoolCanisterRequest),
            InspectCanister(::canic::dto::canister::CanisterInspectionRequest),
            MaintainPool,
            PrepareAuthoritySnapshot(::canic::dto::authority_restore::AuthoritySnapshotRequest),
            PrepareComponentRegistry(
                ::canic::dto::component_registry::RootComponentRegistryPreparationRequest,
            ),
            PrepareFleetActivation,
            #[cfg(canic_capability_role_attestation_signer)]
            PrepareRoleAttestation(::canic::dto::auth::RoleAttestationRequest),
            PreviewCycleRefill(::canic::dto::icp_refill::CycleRefillInput),
            ProvisionChild(::canic::dto::component_registry::RootComponentChildAllocationRequest),
            ProvisionComponent(::canic::dto::component_registry::RootComponentAllocationRequest),
            ProvisionComponents(
                ::canic::dto::component_provisioning::RootComponentProvisioningAcceptanceRequest,
            ),
            ProvisionPeer(::canic::dto::component_registry::RootPeerComponentAllocationRequest),
            PublishReleaseSet(::canic::dto::template::WasmStoreAdminCommand),
            RefillCycles(::canic::dto::icp_refill::CycleRefillInput),
            RemoveComponent(::canic::dto::component_registry::RootComponentDrainingRequest),
            RemoveRoot(::canic::dto::role::RootRemovalRequest),
            RemoveSubtree(::canic::dto::component_registry::RootComponentSubtreeRemovalRequest),
            RespondCapability(::canic::dto::capability::RootCapabilityEnvelopeV1),
            ResumeAuthoritySnapshot(::canic::dto::authority_restore::AuthoritySnapshotRequest),
            ResumeFleetActivation(::canic::dto::fleet_activation::FleetActivationResumeRequest),
            RetryPoolRefill,
            RetryPoolReset(::canic::dto::pool::PoolCanisterRequest),
            SetCyclesFunding(::canic::dto::state::SetCyclesFundingRequest),
            SetFleetStatus(::canic::dto::state::SetFleetStatusRequest),
            SynchronizeComponentDirectories(
                ::canic::dto::component_provisioning::RootComponentDirectorySynchronizationRequest,
            ),
            SynchronizeRegistry(::canic::dto::fleet_registry::FleetSubnetRootRegistrySyncRequest),
            UpsertIssuerPolicy(::canic::dto::auth::RootIssuerPolicyUpsertRequest),
            UpsertIssuerRenewalTemplate(
                ::canic::dto::auth::RootIssuerRenewalTemplateUpsertRequest,
            ),
        }

        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootCommandResponse {
            GetOrCreateDelegationProof(::canic::dto::auth::RootDelegationProofBatchProof),
            HandoffPoolCanister(::canic::dto::pool::PoolHandoffResponse),
            ImportPoolCanister(::canic::dto::pool::PoolImportResponse),
            InspectCanister(::canic::dto::canister::CanisterStatusResponse),
            MaintainPool(::canic::dto::pool::PoolMaintenanceResponse),
            OperationAccepted(::canic::dto::role::OperationReceipt),
            PrepareAuthoritySnapshot(
                ::canic::dto::authority_restore::AuthorityRestoreFenceStatusResponse,
            ),
            PrepareComponentRegistry(
                ::canic::dto::component_registry::RootComponentRegistryStatusResponse,
            ),
            #[cfg(canic_capability_role_attestation_signer)]
            PrepareRoleAttestation(::canic::dto::auth::RoleAttestationPrepareResponse),
            PreviewCycleRefill(::canic::dto::icp_refill::IcpRefillDryRun),
            PublishReleaseSet(::canic::dto::template::WasmStoreAdminResponse),
            RespondCapability(::canic::dto::capability::RootCapabilityResponseV1),
            ResumeAuthoritySnapshot(
                ::canic::dto::authority_restore::AuthorityRestoreFenceStatusResponse,
            ),
            RetryPoolRefill(::canic::dto::pool::PoolRefillRetryResponse),
            RetryPoolReset(::canic::dto::pool::PoolResetRetryResponse),
            SetCyclesFunding(::canic::dto::state::SetStateResponse<bool>),
            SetFleetStatus(
                ::canic::dto::state::SetStateResponse<::canic::dto::state::FleetStatus>,
            ),
            SynchronizeComponentDirectories(
                ::canic::dto::component_provisioning::RootComponentDirectorySynchronizationResponse,
            ),
            UpsertIssuerPolicy(::canic::dto::auth::RootIssuerPolicyResponse),
            UpsertIssuerRenewalTemplate(
                ::canic::dto::auth::RootIssuerRenewalTemplateResponse,
            ),
        }

        impl RootCommand {
            #[doc(hidden)]
            const fn __canic_payload_max_bytes(&self) -> usize {
                match self {
                    RootCommand::ProvisionComponents(_) => {
                        ::canic::__internal::core::control_plane_support::ops::component_provisioning_plan::MAX_FLEET_SUBNET_ROOT_PROVISIONING_ACCEPTANCE_PAYLOAD_BYTES
                    }
                    _ => {
                        ::canic::__internal::core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES
                    }
                }
            }
        }

        #[doc(hidden)]
        fn __canic_inspect_root_update_message() {
            if $crate::__internal::core::ingress::payload::current_method_name()
                != $crate::__internal::core::protocol::CANIC_COMMAND
            {
                $crate::__internal::core::ingress::payload::inspect_update_message();
                return;
            }

            let bytes = $crate::__internal::core::ingress::payload::current_payload_bytes();
            if bytes.len()
                > ::canic::__internal::core::control_plane_support::ops::component_provisioning_plan::MAX_FLEET_SUBNET_ROOT_PROVISIONING_ACCEPTANCE_PAYLOAD_BYTES
            {
                return;
            }
            let Ok(command) = ::canic::__internal::candid::decode_one::<RootCommand>(&bytes) else {
                return;
            };
            if $crate::__internal::core::ingress::payload::payload_within_limit(
                bytes.len(),
                command.__canic_payload_max_bytes(),
            ) {
                $crate::__internal::core::ingress::payload::accept_current_message();
            }
        }

        #[$crate::canic_update(
            public,
            payload(max_bytes = ::canic::__internal::core::control_plane_support::ops::component_provisioning_plan::MAX_FLEET_SUBNET_ROOT_PROVISIONING_ACCEPTANCE_PAYLOAD_BYTES)
        )]
        async fn canic_command(
            command: RootCommand,
        ) -> Result<RootCommandResponse, ::canic::Error> {
            ::std::boxed::Box::pin(#[expect(
                clippy::large_stack_frames,
                reason = "the large root command dispatch future is immediately heap-boxed"
            )] async move {
            if !$crate::__internal::core::ingress::payload::payload_within_limit(
                $crate::__internal::cdk::raw::msg_arg_data_size(),
                command.__canic_payload_max_bytes(),
            ) {
                return Err(::canic::Error::from_registered(
                    $crate::__internal::core::diagnostics::codes::REQUEST_CAPACITY,
                ));
            }
            let caller = $crate::__internal::cdk::api::msg_caller();
            let controller_command = matches!(
                &command,
                RootCommand::AdoptStore(_)
                    | RootCommand::BootstrapStore(_)
                    | RootCommand::HandoffPoolCanister(_)
                    | RootCommand::ImportPoolCanister(_)
                    | RootCommand::InspectCanister(_)
                    | RootCommand::MaintainPool
                    | RootCommand::PrepareAuthoritySnapshot(_)
                    | RootCommand::PrepareComponentRegistry(_)
                    | RootCommand::PrepareFleetActivation
                    | RootCommand::PreviewCycleRefill(_)
                    | RootCommand::ProvisionComponent(_)
                    | RootCommand::PublishReleaseSet(_)
                    | RootCommand::RefillCycles(_)
                    | RootCommand::RemoveComponent(_)
                    | RootCommand::RemoveSubtree(_)
                    | RootCommand::ResumeAuthoritySnapshot(_)
                    | RootCommand::ResumeFleetActivation(_)
                    | RootCommand::RetryPoolRefill
                    | RootCommand::RetryPoolReset(_)
                    | RootCommand::SetCyclesFunding(_)
                    | RootCommand::SetFleetStatus(_)
                    | RootCommand::SynchronizeRegistry(_)
                    | RootCommand::UpsertIssuerPolicy(_)
                    | RootCommand::UpsertIssuerRenewalTemplate(_)
            );
            if controller_command {
                $crate::__internal::core::access::auth::is_controller(caller)
                    .await
                    .map_err(::canic::Error::from)?;
            }

            if matches!(&command, RootCommand::RemoveRoot(_)) {
                $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_fleet_subnet_root_removal_caller(
                    caller,
                    $crate::__internal::cdk::api::is_controller(&caller),
                )?;
            }

            if matches!(&command, RootCommand::GetOrCreateDelegationProof) {
                use $crate::__internal::core::access::expr::AsyncAccessPredicate as _;
                let context = $crate::__internal::core::access::expr::AccessContext {
                    caller,
                    call: $crate::__internal::core::ids::EndpointCall {
                        endpoint: $crate::__internal::core::ids::EndpointId::new("canic_command"),
                        kind: $crate::__internal::core::ids::EndpointCallKind::Update,
                    },
                };
                $crate::__internal::control_plane::api::component_auth::ActiveComponentMemberPredicate
                    .eval(&context)
                    .await
                    .map_err(::canic::Error::from)?;
            }

            #[cfg(canic_capability_role_attestation_signer)]
            if matches!(&command, RootCommand::PrepareRoleAttestation(_)) {
                use $crate::__internal::core::access::expr::AsyncAccessPredicate as _;
                let context = $crate::__internal::core::access::expr::AccessContext {
                    caller,
                    call: $crate::__internal::core::ids::EndpointCall {
                        endpoint: $crate::__internal::core::ids::EndpointId::new("canic_command"),
                        kind: $crate::__internal::core::ids::EndpointCallKind::Update,
                    },
                };
                $crate::__internal::control_plane::api::component_auth::ActiveComponentMemberPredicate
                    .eval(&context)
                    .await
                    .map_err(::canic::Error::from)?;
            }

            match &command {
                RootCommand::ProvisionChild(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_component_child_caller(request, caller)?;
                }
                RootCommand::ProvisionComponents(_)
                | RootCommand::SynchronizeComponentDirectories(_) => {
                    $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::authorize_coordinator_caller(caller)?;
                }
                RootCommand::ProvisionPeer(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_peer_component_allocation_caller(request, caller)?;
                }
                _ => {}
            }

            if matches!(&command, RootCommand::RespondCapability(_)) {
                use $crate::__internal::core::access::expr::AsyncAccessPredicate as _;
                let context = $crate::__internal::core::access::expr::AccessContext {
                    caller,
                    call: $crate::__internal::core::ids::EndpointCall {
                        endpoint: $crate::__internal::core::ids::EndpointId::new("canic_command"),
                        kind: $crate::__internal::core::ids::EndpointCallKind::Update,
                    },
                };
                $crate::__internal::control_plane::api::component_rpc::RootCapabilityCallerPredicate
                    .eval(&context)
                    .await
                    .map_err(::canic::Error::from)?;
            }

            let recovery_command = matches!(
                &command,
                RootCommand::PrepareAuthoritySnapshot(_) | RootCommand::ResumeAuthoritySnapshot(_)
            );
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(
                recovery_command,
            )?;
            let prepared_command = matches!(
                &command,
                RootCommand::AdoptStore(_)
                    | RootCommand::BootstrapStore(_)
                    | RootCommand::HandoffPoolCanister(_)
                    | RootCommand::ImportPoolCanister(_)
                    | RootCommand::MaintainPool
                    | RootCommand::PrepareComponentRegistry(_)
                    | RootCommand::PrepareFleetActivation
                    | RootCommand::ProvisionComponent(_)
                    | RootCommand::ProvisionComponents(_)
                    | RootCommand::PublishReleaseSet(_)
                    | RootCommand::ResumeFleetActivation(_)
                    | RootCommand::RetryPoolRefill
                    | RootCommand::RetryPoolReset(_)
                    | RootCommand::SynchronizeComponentDirectories(_)
                    | RootCommand::SynchronizeRegistry(_)
            );
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_command_variant_allowed(
                prepared_command,
            )?;
            match command {
                RootCommand::AdoptStore(request) => {
                    let operation_id = request.operation_id;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::adopt_fleet_subnet_wasm_store(request).await?;
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::BootstrapStore(request) => {
                    let operation_id = request.operation_id;
                    ::canic::api::canister::template::WasmStoreBootstrapApi::bootstrap_root_store(request).await?;
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::GetOrCreateDelegationProof => {
                    $crate::__internal::core::api::auth::AuthApi::get_or_create_chain_key_delegation_proof_root()
                        .await
                        .map(RootCommandResponse::GetOrCreateDelegationProof)
                }
                RootCommand::HandoffPoolCanister(request) => {
                    let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(
                        ::canic::dto::pool::PoolAdminCommand::Handoff {
                            canister_id: request.canister_id,
                            recipient: request.recipient,
                        },
                    )
                    .await?;
                    match response {
                        ::canic::dto::pool::PoolAdminResponse::HandedOff {
                            canister_id,
                            recipient,
                        } => Ok(RootCommandResponse::HandoffPoolCanister(
                            ::canic::dto::pool::PoolHandoffResponse {
                                canister_id,
                                recipient,
                            },
                        )),
                        _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    }
                }
                RootCommand::ImportPoolCanister(request) => {
                    let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(
                        ::canic::dto::pool::PoolAdminCommand::Import {
                            canister_id: request.canister_id,
                        },
                    )
                    .await?;
                    let response = match response {
                        ::canic::dto::pool::PoolAdminResponse::Imported { canister_id } => {
                            ::canic::dto::pool::PoolImportResponse::Imported { canister_id }
                        }
                        ::canic::dto::pool::PoolAdminResponse::ResetFailed {
                            canister_id,
                            reason,
                        } => ::canic::dto::pool::PoolImportResponse::ResetFailed {
                            canister_id,
                            reason,
                        },
                        _ => return Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    };
                    Ok(RootCommandResponse::ImportPoolCanister(response))
                }
                RootCommand::InspectCanister(request) => {
                    $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_status(
                        request.canister_id,
                    )
                    .await
                    .map(RootCommandResponse::InspectCanister)
                }
                RootCommand::MaintainPool => {
                    let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(
                        ::canic::dto::pool::PoolAdminCommand::Maintain,
                    )
                    .await?;
                    let response = match response {
                        ::canic::dto::pool::PoolAdminResponse::Maintained => {
                            ::canic::dto::pool::PoolMaintenanceResponse::Maintained
                        }
                        ::canic::dto::pool::PoolAdminResponse::MaintenancePaused { reason } => {
                            ::canic::dto::pool::PoolMaintenanceResponse::MaintenancePaused { reason }
                        }
                        ::canic::dto::pool::PoolAdminResponse::Created { canister_id } => {
                            ::canic::dto::pool::PoolMaintenanceResponse::Created { canister_id }
                        }
                        ::canic::dto::pool::PoolAdminResponse::RefillWaitingForCycles {
                            available,
                            creation_amount,
                        } => ::canic::dto::pool::PoolMaintenanceResponse::RefillWaitingForCycles {
                            available,
                            creation_amount,
                        },
                        ::canic::dto::pool::PoolAdminResponse::RefillPending {
                            operation_id,
                            uncertain_result,
                        } => ::canic::dto::pool::PoolMaintenanceResponse::RefillPending {
                            operation_id,
                            uncertain_result,
                        },
                        ::canic::dto::pool::PoolAdminResponse::RefillBlocked {
                            operation_id,
                            failure,
                        } => ::canic::dto::pool::PoolMaintenanceResponse::RefillBlocked {
                            operation_id,
                            failure,
                        },
                        ::canic::dto::pool::PoolAdminResponse::ResetReady { canister_id } => {
                            ::canic::dto::pool::PoolMaintenanceResponse::ResetReady { canister_id }
                        }
                        ::canic::dto::pool::PoolAdminResponse::ResetFailed {
                            canister_id,
                            reason,
                        } => ::canic::dto::pool::PoolMaintenanceResponse::ResetFailed {
                            canister_id,
                            reason,
                        },
                        _ => return Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    };
                    Ok(RootCommandResponse::MaintainPool(response))
                }
                RootCommand::PrepareAuthoritySnapshot(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_authority_snapshot(request)
                        .await
                        .map(RootCommandResponse::PrepareAuthoritySnapshot)
                }
                RootCommand::PrepareComponentRegistry(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_component_registry(request)
                        .await
                        .map(RootCommandResponse::PrepareComponentRegistry)
                }
                RootCommand::PrepareFleetActivation => {
                    __canic_run_prepared_root_init_block().await;
                    let response = $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_fleet_activation().await?;
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt {
                            operation_id: response.identity.operation_id,
                        },
                    ))
                }
                #[cfg(canic_capability_role_attestation_signer)]
                RootCommand::PrepareRoleAttestation(request) => {
                    $crate::__internal::control_plane::api::component_auth::ComponentAuthApi::prepare_role_attestation(request)
                        .map(RootCommandResponse::PrepareRoleAttestation)
                }
                RootCommand::PreviewCycleRefill(request) => {
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
                            Ok(RootCommandResponse::PreviewCycleRefill(response))
                        }
                        _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    }
                }
                RootCommand::ProvisionChild(request) => {
                    let operation_id = request.operation_id;
                    let component = request.component;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reserve_component_child(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_child_allocation(component, operation_id);
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::ProvisionComponent(request) => {
                    let operation_id = request.operation_id;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reserve_component_allocation(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_allocation(operation_id);
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::ProvisionComponents(request) => {
                    $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::accept(request)
                        .await
                        .map(RootCommandResponse::OperationAccepted)
                }
                RootCommand::ProvisionPeer(request) => {
                    let operation_id = request.operation_id;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::reserve_peer_component_allocation(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_allocation(operation_id);
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::PublishReleaseSet(command) => {
                    ::canic::api::canister::template::WasmStorePublicationApi::admin(command)
                        .await
                        .map(RootCommandResponse::PublishReleaseSet)
                }
                RootCommand::RefillCycles(request) => {
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
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::RemoveComponent(request) => {
                    let operation_id = request.operation_id;
                    let component = request.component;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::begin_component_draining(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_removal(component, operation_id);
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::RemoveRoot(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::accept_fleet_subnet_root_removal(request)
                        .map(RootCommandResponse::OperationAccepted)
                }
                RootCommand::RemoveSubtree(request) => {
                    let operation_id = request.operation_id;
                    let component = request.component;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::begin_component_subtree_removal(request).await?;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_component_subtree_removal(component, operation_id);
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::RespondCapability(envelope) => {
                    $crate::__internal::control_plane::api::component_rpc::ComponentRpcApi::response_capability_v1_root(envelope)
                        .await
                        .map(RootCommandResponse::RespondCapability)
                }
                RootCommand::ResumeAuthoritySnapshot(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::resume_authority_snapshot(request)
                        .await
                        .map(RootCommandResponse::ResumeAuthoritySnapshot)
                }
                RootCommand::ResumeFleetActivation(request) => {
                    let operation_id = request.operation_id;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::resume_fleet_activation(request).await?;
                    __canic_schedule_prepared_activation_init();
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::RetryPoolRefill => {
                    let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(
                        ::canic::dto::pool::PoolAdminCommand::RetryRefill,
                    )
                    .await?;
                    match response {
                        ::canic::dto::pool::PoolAdminResponse::RefillRetryScheduled {
                            previous_operation_id,
                        } => Ok(RootCommandResponse::RetryPoolRefill(
                            ::canic::dto::pool::PoolRefillRetryResponse {
                                previous_operation_id,
                            },
                        )),
                        _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    }
                }
                RootCommand::RetryPoolReset(request) => {
                    let response = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::admin(
                        ::canic::dto::pool::PoolAdminCommand::RetryReset {
                            canister_id: request.canister_id,
                        },
                    )
                    .await?;
                    match response {
                        ::canic::dto::pool::PoolAdminResponse::ResetQueued { canister_id } => {
                            Ok(RootCommandResponse::RetryPoolReset(
                                ::canic::dto::pool::PoolResetRetryResponse { canister_id },
                            ))
                        }
                        _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    }
                }
                RootCommand::SetCyclesFunding(request) => {
                    let response = $crate::__internal::control_plane::api::state::FleetStateApi::execute_command(
                        ::canic::dto::state::FleetCommand::SetCyclesFundingEnabled(request.enabled),
                    )
                    .await?;
                    match response {
                        ::canic::dto::state::FleetCommandResponse::CyclesFundingEnabled(response) => {
                            Ok(RootCommandResponse::SetCyclesFunding(response))
                        }
                        _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    }
                }
                RootCommand::SetFleetStatus(request) => {
                    let response = $crate::__internal::control_plane::api::state::FleetStateApi::execute_command(
                        ::canic::dto::state::FleetCommand::SetStatus(request.status),
                    )
                    .await?;
                    match response {
                        ::canic::dto::state::FleetCommandResponse::Status(response) => {
                            Ok(RootCommandResponse::SetFleetStatus(response))
                        }
                        _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    }
                }
                RootCommand::SynchronizeComponentDirectories(request) => {
                    $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::synchronize_directories(request)
                        .await
                        .map(RootCommandResponse::SynchronizeComponentDirectories)
                }
                RootCommand::SynchronizeRegistry(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::accept_fleet_registry_synchronization(request)
                        .await
                        .map(RootCommandResponse::OperationAccepted)
                }
                RootCommand::UpsertIssuerPolicy(request) => {
                    $crate::__internal::core::api::auth::AuthApi::upsert_root_issuer_policy_root(request)
                        .map(RootCommandResponse::UpsertIssuerPolicy)
                }
                RootCommand::UpsertIssuerRenewalTemplate(request) => {
                    $crate::__internal::core::api::auth::AuthApi::upsert_root_issuer_renewal_template_root(request)
                        .map(RootCommandResponse::UpsertIssuerRenewalTemplate)
                }
            }
            })
            .await
        }
    };
}

/// Emit the Fleet Subnet Root's role-owned status query.
#[macro_export]
macro_rules! canic_emit_root_status_endpoint {
    () => {
        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootStatusRequest {
            AuthorityRestore,
            Children(::canic::dto::page::PageRequest),
            ComponentDirectoryHead(
                ::canic::dto::component_registry::ComponentDirectoryHeadRequest,
            ),
            ComponentDirectoryPage(
                ::canic::dto::component_registry::ComponentDirectoryPageRequest,
            ),
            ComponentRegistryPartition(
                ::canic::dto::component_registry::ComponentRegistryPartitionRequest,
            ),
            Config,
            CycleBalance,
            CycleHistory(::canic::dto::page::PageRequest),
            FleetAuthority,
            FleetState,
            Health,
            Inventory,
            IssuerRenewal(::canic::dto::auth::RootIssuerRenewalStatusRequest),
            Logs(::canic::dto::role::LogStatusRequest),
            Metrics(::canic::dto::role::MetricsStatusRequest),
            Operation(::canic::dto::role::OperationStatusRequest),
            Overview,
            Pool(::canic::dto::pool::CanisterPoolStatusRequest),
            Readiness,
            #[cfg(canic_capability_role_attestation_signer)]
            RoleAttestation(::canic::dto::auth::RoleAttestationGetRequest),
            Runtime,
            StoreOverview,
        }

        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootStatusResponse {
            AuthorityRestore(::canic::dto::authority_restore::AuthorityRestoreFenceStatusResponse),
            Children(
                ::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>,
            ),
            ComponentDirectoryHead(
                ::canic::dto::component_registry::ComponentDirectoryHead,
            ),
            ComponentDirectoryPage(
                ::canic::dto::component_registry::ComponentDirectoryPageResponse,
            ),
            ComponentRegistryPartition(
                ::canic::dto::component_registry::ComponentRegistryPartitionResponse,
            ),
            Config(::canic::dto::role::ConfigStatusResponse),
            CycleBalance(::canic::dto::role::CycleBalanceStatusResponse),
            CycleHistory(
                ::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>,
            ),
            FleetAuthority(::canic::dto::fleet_subnet_root::FleetSubnetRootAuthority),
            FleetState(::canic::dto::state::FleetStateResponse),
            Health(::canic::dto::runtime::CanicHealthStatus),
            Inventory(::canic::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary),
            IssuerRenewal(::canic::dto::auth::RootIssuerRenewalStatusResponse),
            Logs(::canic::dto::page::Page<::canic::dto::log::LogEntry>),
            Metrics(::canic::dto::page::Page<::canic::dto::metrics::MetricEntry>),
            Operation(::canic::dto::root::RootOperationStatusResponse),
            Overview(::canic::dto::role::RoleOverviewResponse),
            Pool(::canic::dto::pool::CanisterPoolResponse),
            Readiness(::canic::dto::runtime::CanicReadinessStatus),
            #[cfg(canic_capability_role_attestation_signer)]
            RoleAttestation(::canic::dto::auth::SignedRoleAttestation),
            Runtime(::canic::dto::runtime::CanicRuntimeStatus),
            StoreOverview(::canic::dto::template::WasmStoreOverviewResponse),
        }

        #[$crate::canic_query(public)]
        async fn canic_status(
            request: RootStatusRequest,
        ) -> Result<RootStatusResponse, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller();
            let prepared_status = matches!(
                &request,
                RootStatusRequest::ComponentDirectoryHead(_)
                    | RootStatusRequest::ComponentDirectoryPage(_)
                    | RootStatusRequest::ComponentRegistryPartition(_)
                    | RootStatusRequest::FleetAuthority
                    | RootStatusRequest::Operation(_)
                    | RootStatusRequest::Overview
                    | RootStatusRequest::Pool(_)
                    | RootStatusRequest::StoreOverview
            );
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_status_variant_allowed(
                prepared_status,
            )?;
            match &request {
                RootStatusRequest::Children(_)
                | RootStatusRequest::ComponentDirectoryPage(_)
                | RootStatusRequest::CycleBalance
                | RootStatusRequest::CycleHistory(_)
                | RootStatusRequest::Metrics(_)
                | RootStatusRequest::Overview => {}
                #[cfg(canic_capability_role_attestation_signer)]
                RootStatusRequest::RoleAttestation(_) => {}
                RootStatusRequest::Operation(_) => {
                    // The durable operation owner supplies the exact public, peer, or
                    // controller authority used by the dispatch arm below.
                }
                RootStatusRequest::AuthorityRestore
                | RootStatusRequest::ComponentDirectoryHead(_)
                | RootStatusRequest::ComponentRegistryPartition(_)
                | RootStatusRequest::Config
                | RootStatusRequest::FleetAuthority
                | RootStatusRequest::FleetState
                | RootStatusRequest::Health
                | RootStatusRequest::Inventory
                | RootStatusRequest::IssuerRenewal(_)
                | RootStatusRequest::Logs(_)
                | RootStatusRequest::Pool(_)
                | RootStatusRequest::Readiness
                | RootStatusRequest::Runtime
                | RootStatusRequest::StoreOverview => {
                    $crate::__internal::core::access::auth::is_controller(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                }
            }

            match request {
                RootStatusRequest::AuthorityRestore => {
                    $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::status()
                        .map(RootStatusResponse::AuthorityRestore)
                }
                RootStatusRequest::Children(page) => Ok(RootStatusResponse::Children(
                    $crate::__internal::core::api::topology::children::CanisterChildrenApi::page(
                        page,
                    ),
                )),
                RootStatusRequest::ComponentDirectoryHead(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_directory_head(request)
                        .map(RootStatusResponse::ComponentDirectoryHead)
                }
                RootStatusRequest::ComponentDirectoryPage(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_directory_page(request)
                        .map(RootStatusResponse::ComponentDirectoryPage)
                }
                RootStatusRequest::ComponentRegistryPartition(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_registry_partition(request)
                        .map(RootStatusResponse::ComponentRegistryPartition)
                }
                RootStatusRequest::Config => {
                    $crate::__internal::core::api::config::ConfigApi::export_toml().map(|toml| {
                        RootStatusResponse::Config(::canic::dto::role::ConfigStatusResponse { toml })
                    })
                }
                RootStatusRequest::CycleBalance => Ok(RootStatusResponse::CycleBalance(
                    ::canic::dto::role::CycleBalanceStatusResponse {
                        cycles: $crate::__internal::cdk::api::canister_cycle_balance(),
                    },
                )),
                RootStatusRequest::CycleHistory(page) => {
                    Ok(RootStatusResponse::CycleHistory(
                        $crate::__internal::core::api::cycles::CycleTrackerQuery::page(page),
                    ))
                }
                RootStatusRequest::FleetAuthority => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_authority()
                        .map(RootStatusResponse::FleetAuthority)
                }
                RootStatusRequest::FleetState => Ok(RootStatusResponse::FleetState(
                    $crate::__internal::core::api::state::FleetStateQuery::snapshot(),
                )),
                RootStatusRequest::Health => Ok(RootStatusResponse::Health(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::health(Some(
                        $crate::__internal::cdk::api::time(),
                    )),
                )),
                RootStatusRequest::Inventory => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_canister_summary()
                        .map(RootStatusResponse::Inventory)
                }
                RootStatusRequest::IssuerRenewal(request) => {
                    $crate::__internal::core::api::auth::AuthApi::root_issuer_renewal_status_root(
                        request,
                    )
                    .map(RootStatusResponse::IssuerRenewal)
                }
                RootStatusRequest::Logs(request) => Ok(RootStatusResponse::Logs(
                    $crate::__internal::core::api::log::LogQuery::page(
                        request.crate_name,
                        request.topic,
                        request.min_level,
                        request.page,
                    ),
                )),
                RootStatusRequest::Metrics(request) => {
                    $crate::__canic_role_metrics_status!(request).map(RootStatusResponse::Metrics)
                }
                RootStatusRequest::Operation(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_operation_status(
                        request.operation_id,
                        caller,
                        $crate::__internal::cdk::api::is_controller(&caller),
                    )
                    .map(RootStatusResponse::Operation)
                }
                RootStatusRequest::Overview => {
                    let capabilities = $crate::__canic_compiled_role_capabilities!();
                    Ok(RootStatusResponse::Overview(
                        $crate::__internal::core::api::role::RoleOverviewApi::overview(
                            $crate::__internal::core::ids::CanisterRole::from(env!(
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
                    ))
                }
                RootStatusRequest::Pool(request) => {
                    $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::status(
                        request,
                    )
                    .map(RootStatusResponse::Pool)
                }
                RootStatusRequest::Readiness => Ok(RootStatusResponse::Readiness(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::readiness(
                        $crate::__internal::cdk::api::time(),
                    ),
                )),
                #[cfg(canic_capability_role_attestation_signer)]
                RootStatusRequest::RoleAttestation(request) => {
                    $crate::__internal::control_plane::api::component_auth::ComponentAuthApi::get_role_attestation(request)
                        .map(RootStatusResponse::RoleAttestation)
                }
                RootStatusRequest::Runtime => Ok(RootStatusResponse::Runtime(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::runtime_status(
                        $crate::__internal::cdk::api::time(),
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION"),
                        $crate::VERSION,
                        $crate::__internal::cdk::api::canister_version(),
                    ),
                )),
                RootStatusRequest::StoreOverview => {
                    ::canic::api::canister::template::WasmStorePublicationApi::overview()
                        .map(RootStatusResponse::StoreOverview)
                }
            }
        }
    };
}
