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
            AcceptFunding(::canic::dto::fleet_funding::FleetRootFundingAcceptanceRequest),
            ActivateFleetAdmission(
                ::canic::dto::fleet_admission::FleetAdmissionActivateRootRequest,
            ),
            ActivateFundingPolicyRotation(
                ::canic::dto::fleet_funding::FleetFundingPolicyRotationRootActivateRequest,
            ),
            AdoptStore(::canic::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest),
            BootstrapStore(::canic::dto::root_store::RootStoreBootstrapRequest),
            #[cfg(canic_capability_root_delegation)]
            GetOrCreateDelegationProof,
            #[cfg(canic_capability_root_delegation)]
            GetChainKeyPublicKey(::canic::dto::auth::RootChainKeyPublicKeyRequest),
            HandoffPoolCanister(::canic::dto::pool::PoolHandoffRequest),
            ImportPoolCanister(::canic::dto::pool::PoolCanisterRequest),
            ImportPoolCapacity(::canic::dto::pool_import::PoolImportCommand),
            InspectCanister(::canic::dto::canister::CanisterInspectionRequest),
            InspectCanisterHistory(::canic::dto::canister::CanisterInspectionRequest),
            MaintainPool,
            ObserveCanister(::canic::dto::observability::FleetCanisterObservabilityRequest),
            OpenFleetAdmission(
                ::canic::dto::fleet_admission::FleetAdmissionOpenRootRequest,
            ),
            PrepareAuthoritySnapshot(::canic::dto::authority_restore::AuthoritySnapshotRequest),
            PrepareComponentRegistry(
                ::canic::dto::component_registry::RootComponentRegistryPreparationRequest,
            ),
            PrepareFleetActivation,
            PrepareFleetAdmission(
                ::canic::dto::fleet_admission::FleetAdmissionPrepareRootRequest,
            ),
            PrepareFundingPolicyRotation(
                ::canic::dto::fleet_funding::FleetFundingPolicyRotationRootPrepareRequest,
            ),
            #[cfg(canic_capability_role_attestation_signer)]
            PrepareRoleAttestation(::canic::dto::auth::RoleAttestationRequest),
            PrepareStoreFixture(::canic::dto::root_store::RootStoreFixturePrepareRequest),
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
            #[cfg(canic_capability_root_delegation)]
            ConfigureIssuer(::canic::dto::auth::RootIssuerConfigureRequest),
        }

        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootCommandResponse {
            PrepareStoreFixture(Result<::canic::dto::fixture_provisioning::FixtureSourceStatus,
                ::canic::dto::fixture_provisioning::FixtureStoreError>),
            AcceptFunding(::canic::dto::fleet_funding::FleetRootFundingAcceptanceReceipt),
            ActivateFleetAdmission(
                ::canic::dto::fleet_admission::FleetAdmissionRootReceipt,
            ),
            ActivateFundingPolicyRotation(
                ::canic::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt,
            ),
            #[cfg(canic_capability_root_delegation)]
            GetOrCreateDelegationProof(::canic::dto::auth::RootDelegationProofBatchProof),
            #[cfg(canic_capability_root_delegation)]
            GetChainKeyPublicKey(Vec<u8>),
            HandoffPoolCanister(::canic::dto::pool::PoolHandoffResponse),
            ImportPoolCanister(::canic::dto::pool::PoolImportResponse),
            ImportPoolCapacity(::canic::dto::pool_import::PoolImportStatus),
            InspectCanister(::canic::dto::canister::CanisterStatusResponse),
            InspectionReserveRequired(::canic::dto::canister::CanisterInspectionReserveResponse),
            InspectCanisterHistory(::canic::dto::canister::CanisterHistoryResponse),
            MaintainPool(::canic::dto::pool::PoolMaintenanceResponse),
            ObserveCanister(::canic::dto::observability::CanisterObservabilityResponse),
            OperationAccepted(::canic::dto::role::OperationReceipt),
            OpenFleetAdmission(::canic::dto::fleet_admission::FleetAdmissionRootReceipt),
            PrepareAuthoritySnapshot(
                ::canic::dto::authority_restore::AuthorityRestoreFenceStatusResponse,
            ),
            PrepareComponentRegistry(
                ::canic::dto::component_registry::RootComponentRegistryStatusResponse,
            ),
            PrepareFleetAdmission(::canic::dto::fleet_admission::FleetAdmissionRootReceipt),
            PrepareFundingPolicyRotation(
                ::canic::dto::fleet_funding::FleetFundingPolicyRotationRootReceipt,
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
            SetCyclesFunding(::canic::dto::state::FleetStateCommandResult<bool>),
            SetFleetStatus(
                ::canic::dto::state::FleetStateCommandResult<::canic::dto::state::FleetStatus>,
            ),
            SynchronizeComponentDirectories(
                ::canic::dto::component_provisioning::RootComponentDirectorySynchronizationResponse,
            ),
            #[cfg(canic_capability_root_delegation)]
            ConfigureIssuer(::canic::dto::auth::RootIssuerConfigureResponse),
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
                != $crate::__internal::core::protocol::CANIC_ROOT_COMMAND
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
            internal,
            public,
            payload(max_bytes = ::canic::__internal::core::control_plane_support::ops::component_provisioning_plan::MAX_FLEET_SUBNET_ROOT_PROVISIONING_ACCEPTANCE_PAYLOAD_BYTES)
        )]
        async fn canic_root_command(
            command: RootCommand,
        ) -> Result<RootCommandResponse, ::canic::Error> {
            ::std::boxed::Box::pin(#[expect(
                clippy::large_stack_frames,
                reason = "the large root command dispatch future is immediately heap-boxed"
            )] async move {
            // State recovery must remain reachable while ordinary updates are fenced.
            if !matches!(&command, RootCommand::SetCyclesFunding(_) | RootCommand::SetFleetStatus(_)) {
                $crate::__internal::core::access::expr::eval_default_fleet_guard(
                    $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                    $crate::__internal::core::ids::EndpointCall {
                        endpoint: $crate::__internal::core::ids::EndpointId::new(
                            $crate::__internal::core::protocol::CANIC_ROOT_COMMAND,
                        ),
                        kind: $crate::__internal::core::ids::EndpointCallKind::Update,
                    },
                )
                    .map_err(::canic::Error::from)?;
            }
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
                    | RootCommand::PrepareStoreFixture(_)
                    | RootCommand::HandoffPoolCanister(_)
                    | RootCommand::ImportPoolCanister(_)
                    | RootCommand::ImportPoolCapacity(_)
                    | RootCommand::InspectCanister(_)
                    | RootCommand::InspectCanisterHistory(_)
                    | RootCommand::MaintainPool
                    | RootCommand::ObserveCanister(_)
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
            );
            #[cfg(canic_capability_root_delegation)]
            let controller_command = controller_command || matches!(&command, RootCommand::GetChainKeyPublicKey(_) | RootCommand::ConfigureIssuer(_));
            if controller_command {
                $crate::__internal::core::access::auth::is_controller(caller)
                    .await
                    .map_err(::canic::Error::from)?;
            }

            if let RootCommand::ImportPoolCapacity(request) = &command {
                let operator = $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_operator(request)?;
                if caller != operator {
                    return Err($crate::__internal::core::control_plane_support::error::InternalError::forbidden().into());
                }
            }

            if matches!(&command, RootCommand::RemoveRoot(_)) {
                $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_fleet_subnet_root_removal_caller(
                    caller,
                    $crate::__internal::cdk::api::is_controller(&caller),
                )?;
            }

            if matches!(
                &command,
                RootCommand::AcceptFunding(_)
                    | RootCommand::ActivateFleetAdmission(_)
                    | RootCommand::ActivateFundingPolicyRotation(_)
                    | RootCommand::OpenFleetAdmission(_)
                    | RootCommand::PrepareFleetAdmission(_)
                    | RootCommand::PrepareFundingPolicyRotation(_)
            ) {
                if matches!(
                    &command,
                    RootCommand::ActivateFleetAdmission(_)
                        | RootCommand::OpenFleetAdmission(_)
                        | RootCommand::PrepareFleetAdmission(_)
                ) {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_root_admission_caller(caller)?;
                } else {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::authorize_root_funding_caller(caller)?;
                }
            }

            #[cfg(canic_capability_root_delegation)]
            if matches!(&command, RootCommand::GetOrCreateDelegationProof) {
                use $crate::__internal::core::access::expr::AsyncAccessPredicate as _;
                let context = $crate::__internal::core::access::expr::AccessContext {
                    caller,
                    call: $crate::__internal::core::ids::EndpointCall {
                        endpoint: $crate::__internal::core::ids::EndpointId::new(
                            $crate::__internal::core::protocol::CANIC_ROOT_COMMAND,
                        ),
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
                        endpoint: $crate::__internal::core::ids::EndpointId::new(
                            $crate::__internal::core::protocol::CANIC_ROOT_COMMAND,
                        ),
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
                        endpoint: $crate::__internal::core::ids::EndpointId::new(
                            $crate::__internal::core::protocol::CANIC_ROOT_COMMAND,
                        ),
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
                    | RootCommand::InspectCanister(_)
                    | RootCommand::InspectCanisterHistory(_)
            );
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(
                recovery_command,
            )?;
            let prepared_command = matches!(
                &command,
                RootCommand::AcceptFunding(_)
                    | RootCommand::AdoptStore(_)
                    | RootCommand::BootstrapStore(_)
                    | RootCommand::PrepareStoreFixture(_)
                    | RootCommand::HandoffPoolCanister(_)
                    | RootCommand::ImportPoolCanister(_)
                    | RootCommand::ImportPoolCapacity(_)
                    | RootCommand::InspectCanister(_)
                    | RootCommand::InspectCanisterHistory(_)
                    | RootCommand::MaintainPool
                    | RootCommand::PrepareComponentRegistry(_)
                    | RootCommand::PrepareFleetActivation
                    | RootCommand::ProvisionComponent(_)
                    | RootCommand::ProvisionComponents(_)
                    | RootCommand::PublishReleaseSet(_)
                    | RootCommand::ResumeFleetActivation(_)
                    | RootCommand::RetryPoolRefill
                    | RootCommand::RetryPoolReset(_)
                    | RootCommand::RespondCapability(_)
                    | RootCommand::SynchronizeComponentDirectories(_)
                    | RootCommand::SynchronizeRegistry(_)
            );
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_command_variant_allowed(
                prepared_command,
            )?;
            match command {
                RootCommand::AcceptFunding(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::accept_root_funding(request)
                        .map(RootCommandResponse::AcceptFunding)
                }
                RootCommand::ActivateFleetAdmission(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_root_fleet_admission(request)
                        .map(RootCommandResponse::ActivateFleetAdmission)
                }
                RootCommand::ActivateFundingPolicyRotation(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::activate_root_funding_policy_rotation(request)
                        .await
                        .map(RootCommandResponse::ActivateFundingPolicyRotation)
                }
                RootCommand::AdoptStore(request) => {
                    let operation_id = request.operation_id;
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::adopt_fleet_subnet_wasm_store(request).await?;
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                RootCommand::PrepareStoreFixture(request) => {
                    ::canic::api::canister::template::WasmStoreBootstrapApi::prepare_root_store_fixture(request)
                        .await.map(RootCommandResponse::PrepareStoreFixture)
                }
                RootCommand::BootstrapStore(request) => {
                    let operation_id = request.operation_id;
                    ::canic::api::canister::template::WasmStoreBootstrapApi::bootstrap_root_store(request).await?;
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                #[cfg(canic_capability_root_delegation)]
                RootCommand::GetOrCreateDelegationProof => {
                    $crate::__internal::core::api::auth::AuthApi::get_or_create_chain_key_delegation_proof_root()
                        .await
                        .map(RootCommandResponse::GetOrCreateDelegationProof)
                }
                #[cfg(canic_capability_root_delegation)]
                RootCommand::GetChainKeyPublicKey(request) => {
                    $crate::__internal::core::api::auth::AuthApi::root_chain_key_public_key(request)
                        .await
                        .map(RootCommandResponse::GetChainKeyPublicKey)
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
                RootCommand::ImportPoolCapacity(request) => {
                    $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_command(request)
                        .await.map(RootCommandResponse::ImportPoolCapacity)
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
                    $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_inspection(
                        request.canister_id,
                    )
                    .await
                    .map(|outcome| match outcome {
                        ::canic::dto::canister::CanisterInspectionOutcome::Status(status) => {
                            RootCommandResponse::InspectCanister(*status)
                        }
                        ::canic::dto::canister::CanisterInspectionOutcome::ReserveRequired(evidence) => {
                            RootCommandResponse::InspectionReserveRequired(evidence)
                        }
                    })
                }
                RootCommand::InspectCanisterHistory(request) => {
                    $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_history(request.canister_id)
                        .await
                        .map(RootCommandResponse::InspectCanisterHistory)
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
                            attempt_count,
                            creation_amount,
                            execution_margin,
                            last_attempt_at_ns,
                            ledger_fee,
                            readiness_floor,
                            required,
                            retry_at_ns,
                            shortfall,
                        } => ::canic::dto::pool::PoolMaintenanceResponse::RefillWaitingForCycles {
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
                RootCommand::ObserveCanister(request) => {
                    $crate::__internal::core::api::observability::ObservabilityApi::observe_root_controlled_canister(
                        request.canister_id,
                        request.request,
                    )
                    .await
                    .map(RootCommandResponse::ObserveCanister)
                }
                RootCommand::OpenFleetAdmission(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::open_root_fleet_admission(request)
                        .map(RootCommandResponse::OpenFleetAdmission)
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
                    let response = $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_fleet_activation().await?;
                    Ok(RootCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt {
                            operation_id: response.identity.operation_id,
                        },
                    ))
                }
                RootCommand::PrepareFleetAdmission(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_root_fleet_admission(request)
                        .map(RootCommandResponse::PrepareFleetAdmission)
                }
                RootCommand::PrepareFundingPolicyRotation(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::prepare_root_funding_policy_rotation(request)
                        .map(RootCommandResponse::PrepareFundingPolicyRotation)
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
                    match response.change {
                        ::canic::dto::state::FleetCommandResponse::CyclesFundingEnabled(change) => {
                            Ok(RootCommandResponse::SetCyclesFunding(::canic::dto::state::FleetStateCommandResult {
                                change, propagation: response.propagation, reconciliation_error: response.reconciliation_error,
                            }))
                        }
                        _ => Err($crate::__internal::core::control_plane_support::error::InternalError::invariant().into()),
                    }
                }
                RootCommand::SetFleetStatus(request) => {
                    let response = $crate::__internal::control_plane::api::state::FleetStateApi::execute_command(
                        ::canic::dto::state::FleetCommand::SetStatus(request.status),
                    )
                    .await?;
                    match response.change {
                        ::canic::dto::state::FleetCommandResponse::Status(change) => {
                            Ok(RootCommandResponse::SetFleetStatus(::canic::dto::state::FleetStateCommandResult {
                                change, propagation: response.propagation, reconciliation_error: response.reconciliation_error,
                            }))
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
                #[cfg(canic_capability_root_delegation)]
                RootCommand::ConfigureIssuer(request) => {
                    $crate::__internal::core::api::auth::AuthApi::configure_issuer_root(request)
                        .map(RootCommandResponse::ConfigureIssuer)
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
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum PublicStatusRequest {
            Health,
            #[cfg(canic_capability_observability_metrics)]
            Metrics(::canic::dto::public_status::PublicMetricsRequest),
            #[cfg(canic_capability_observability_history)]
            History(::canic::dto::public_status::PublicHistoryRequest),
            Overview,
            Children(::canic::dto::page::PageRequest),
            ComponentDirectoryPage(::canic::dto::component_registry::ComponentDirectoryPageRequest),
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum PublicStatusResponse {
            Health(::canic::dto::public_status::PublicHealth),
            #[cfg(canic_capability_observability_metrics)]
            Metrics(::canic::dto::public_status::PublicMetricsSnapshot),
            #[cfg(canic_capability_observability_history)]
            History(::canic::dto::public_status::PublicHistorySnapshot),
            Overview(::canic::dto::role::RoleOverviewResponse),
            Children(::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>),
            ComponentDirectoryPage(::canic::dto::component_registry::ComponentDirectoryPageResponse),
        }
        #[$crate::canic_query(public)]
        async fn canic_public_status(
            request: PublicStatusRequest,
        ) -> Result<PublicStatusResponse, ::canic::Error> {
            match request {
                PublicStatusRequest::Health => Ok(PublicStatusResponse::Health(::canic::__internal::core::api::public_status::PublicStatusApi::health())),
                #[cfg(canic_capability_observability_metrics)]
                PublicStatusRequest::Metrics(request) => Ok(PublicStatusResponse::Metrics(::canic::__internal::core::api::public_status::PublicStatusApi::metrics(request))),
                #[cfg(canic_capability_observability_history)]
                PublicStatusRequest::History(request) => Ok(PublicStatusResponse::History(::canic::__internal::core::api::public_status::PublicStatusApi::history(request))),
                PublicStatusRequest::Overview => {
                    let capabilities = $crate::__canic_compiled_role_capabilities!();
                    Ok(PublicStatusResponse::Overview(
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
                PublicStatusRequest::Children(page) => {
                    $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_status_variant_allowed(false)?;
                    Ok(PublicStatusResponse::Children(
                    $crate::__internal::core::api::topology::children::CanisterChildrenApi::page(
                        page,
                    ),
                    ))
                }
                PublicStatusRequest::ComponentDirectoryPage(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_directory_page(request)
                        .map(PublicStatusResponse::ComponentDirectoryPage)
                }
            }
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum ObservabilityRequest {
            ChildFunding(::canic::__internal::candid::Principal),
            CycleBalance,
            #[cfg(canic_capability_observability_history)]
            CycleHistory(::canic::dto::page::PageRequest),
            Health,
            InspectionReserve(::canic::dto::canister::CanisterInspectionRequest),
            #[cfg(canic_capability_observability_logs)]
            Logs(::canic::dto::role::LogStatusRequest),
            #[cfg(canic_capability_observability_diagnostics)]
            MemoryAllocations,
            #[cfg(canic_capability_observability_metrics)]
            Metrics(::canic::dto::role::MetricsStatusRequest),
            Readiness,
            #[cfg(canic_capability_observability_diagnostics)]
            Runtime,
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum ObservabilityResponse {
            ChildFunding(::canic::dto::observability::ChildFundingUsage),
            CycleBalance(::canic::dto::role::CycleBalanceStatusResponse),
            #[cfg(canic_capability_observability_history)]
            CycleHistory(::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>),
            Health(::canic::dto::runtime::CanicHealthStatus),
            InspectionReserve(::canic::dto::canister::CanisterInspectionReserveResponse),
            #[cfg(canic_capability_observability_logs)]
            Logs(::canic::dto::page::Page<::canic::dto::log::LogEntry>),
            #[cfg(canic_capability_observability_diagnostics)]
            MemoryAllocations(::canic::dto::memory::MemoryAllocationsResponse),
            #[cfg(canic_capability_observability_metrics)]
            Metrics(::canic::dto::page::Page<::canic::dto::metrics::MetricEntry>),
            Readiness(::canic::dto::runtime::CanicReadinessStatus),
            #[cfg(canic_capability_observability_diagnostics)]
            Runtime(::canic::dto::runtime::CanicRuntimeStatus),
        }
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_observability(
            request: ObservabilityRequest,
        ) -> Result<ObservabilityResponse, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_status_variant_allowed(
                matches!(&request, ObservabilityRequest::InspectionReserve(_) | ObservabilityRequest::ChildFunding(_)),
            )?;

            match request {
                ObservabilityRequest::ChildFunding(child) => {
                    $crate::__internal::core::api::observability::ObservabilityApi::child_funding(child)
                        .map(ObservabilityResponse::ChildFunding)
                }
                ObservabilityRequest::CycleBalance => Ok(ObservabilityResponse::CycleBalance(
                    ::canic::dto::role::CycleBalanceStatusResponse {
                        cycles: $crate::__internal::cdk::api::canister_cycle_balance(),
                    },
                )),
                #[cfg(canic_capability_observability_history)]
                ObservabilityRequest::CycleHistory(page) => {
                    Ok(ObservabilityResponse::CycleHistory(
                        $crate::__internal::core::api::cycles::CycleTrackerQuery::page(page),
                    ))
                }
                ObservabilityRequest::Health => Ok(ObservabilityResponse::Health(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::health(Some(
                        $crate::__internal::cdk::api::time(),
                    )),
                )),
                ObservabilityRequest::InspectionReserve(request) => {
                    $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_inspection_reserve(request.canister_id)
                        .map(ObservabilityResponse::InspectionReserve)
                }
                #[cfg(canic_capability_observability_logs)]
                ObservabilityRequest::Logs(request) => Ok(ObservabilityResponse::Logs(
                    $crate::__internal::core::api::log::LogQuery::page(
                        request.crate_name,
                        request.topic,
                        request.min_level,
                        request.page,
                    ),
                )),
                #[cfg(canic_capability_observability_diagnostics)]
                ObservabilityRequest::MemoryAllocations => {
                    $crate::__internal::core::api::memory::MemoryQuery::allocations()
                        .map(ObservabilityResponse::MemoryAllocations)
                }
                #[cfg(canic_capability_observability_metrics)]
                ObservabilityRequest::Metrics(request) => {
                    $crate::__canic_role_metrics_status!(request).map(ObservabilityResponse::Metrics)
                }
                ObservabilityRequest::Readiness => Ok(ObservabilityResponse::Readiness(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::readiness(
                        $crate::__internal::cdk::api::time(),
                    ),
                )),
                #[cfg(canic_capability_observability_diagnostics)]
                ObservabilityRequest::Runtime => Ok(ObservabilityResponse::Runtime(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::runtime_status(
                        $crate::__internal::cdk::api::time(),
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION"),
                        $crate::VERSION,
                        $crate::__internal::cdk::api::canister_version(),
                    ),
                )),
            }
        }
        #[cfg(canic_capability_role_attestation_signer)]
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootAuthStatusRequest {
            #[cfg(canic_capability_role_attestation_signer)]
            RoleAttestation(::canic::dto::auth::RoleAttestationGetRequest),
        }
        #[cfg(canic_capability_role_attestation_signer)]
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootAuthStatusResponse {
            #[cfg(canic_capability_role_attestation_signer)]
            RoleAttestation(::canic::dto::auth::SignedRoleAttestation),
        }
        #[cfg(canic_capability_role_attestation_signer)]
        #[$crate::canic_query(public)]
        async fn canic_root_auth_status(
            request: RootAuthStatusRequest,
        ) -> Result<RootAuthStatusResponse, ::canic::Error> {
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_status_variant_allowed(false)?;

            match request {
                #[cfg(canic_capability_role_attestation_signer)]
                RootAuthStatusRequest::RoleAttestation(request) => {
                    $crate::__internal::control_plane::api::component_auth::ComponentAuthApi::get_role_attestation(request)
                        .map(RootAuthStatusResponse::RoleAttestation)
                }
            }
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootOperationStatusRequest {
            ComponentChildProvisioning(::canic::dto::role::OperationStatusRequest),
            ComponentProvisioning(::canic::dto::role::OperationStatusRequest),
            Operation(::canic::dto::role::OperationStatusRequest),
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootOperationStatusResponse {
            ComponentChildProvisioning(
                ::canic::dto::component_registry::RootComponentChildAllocationResponse,
            ),
            ComponentProvisioning(
                ::canic::dto::component_provisioning::RootComponentProvisioningStatusResponse,
            ),
            Operation(::canic::dto::root::RootOperationStatusResponse),
        }
        #[$crate::canic_query(public)]
        async fn canic_root_operation_status(
            request: RootOperationStatusRequest,
        ) -> Result<RootOperationStatusResponse, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller();
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_status_variant_allowed(true)?;

            match request {
                RootOperationStatusRequest::ComponentChildProvisioning(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_component_child_provisioning_status(
                        request.operation_id,
                        caller,
                        $crate::__internal::cdk::api::is_controller(&caller),
                    )
                    .map(RootOperationStatusResponse::ComponentChildProvisioning)
                }
                RootOperationStatusRequest::ComponentProvisioning(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_component_provisioning_status(
                        request.operation_id,
                        caller,
                        $crate::__internal::cdk::api::is_controller(&caller),
                    )
                    .map(RootOperationStatusResponse::ComponentProvisioning)
                }
                RootOperationStatusRequest::Operation(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_operation_status(
                        request.operation_id,
                        caller,
                        $crate::__internal::cdk::api::is_controller(&caller),
                    )
                    .map(RootOperationStatusResponse::Operation)
                }
            }
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootStatusRequest {
            Admission(::canic::dto::page::PageRequest),
            AuthorityRestore,
            ComponentDirectoryHead(::canic::dto::component_registry::ComponentDirectoryHeadRequest),
            ComponentDirectoryPage(::canic::dto::component_registry::ComponentDirectoryPageRequest),
            ComponentRegistry(::canic::dto::component_registry::RootComponentRegistryPreparationRequest),
            ComponentRegistryActivePartition(
                ::canic::dto::component_registry::ComponentRegistryActivePartitionRequest,
            ),
            ComponentRegistryPartition(::canic::dto::component_registry::ComponentRegistryPartitionRequest),
            Config,
            FleetAuthority,
            FleetState,
            Funding,
            FundingRelease(Option<u64>),
            Inventory,
            #[cfg(canic_capability_root_delegation)]
            IssuerRenewal(::canic::dto::auth::RootIssuerRenewalStatusRequest),
            Pool(::canic::dto::pool::CanisterPoolStatusRequest),
            PoolImport(::canic::dto::pool_import::PoolImportIdentity),
            PoolImportContext,
            PoolRelease,
            ProvisioningRelease(Option<::canic::dto::root::RootProvisioningReleaseKey>),
            ReplayRelease(Option<[u8; 32]>),
            StoreOverview,
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum RootStatusResponse {
            Admission(::canic::dto::fleet_admission::FleetAdmissionRootStatusResponse),
            AuthorityRestore(::canic::dto::authority_restore::AuthorityRestoreFenceStatusResponse),
            ComponentDirectoryHead(::canic::dto::component_registry::ComponentDirectoryHead),
            ComponentDirectoryPage(::canic::dto::component_registry::ComponentDirectoryPageResponse),
            ComponentRegistry(::canic::dto::component_registry::RootComponentRegistryStatusResponse),
            ComponentRegistryActivePartition(
                ::canic::dto::component_registry::ComponentRegistryActivePartitionResponse,
            ),
            ComponentRegistryPartition(
                ::canic::dto::component_registry::ComponentRegistryPartitionResponse,
            ),
            Config(::canic::dto::role::ConfigStatusResponse),
            FleetAuthority(::canic::dto::fleet_subnet_root::FleetSubnetRootAuthority),
            FleetState(::canic::dto::state::FleetStateResponse),
            Funding(::canic::dto::root::RootFundingStatusResponse),
            FundingRelease(::canic::dto::root::RootFundingReleaseResponse),
            Inventory(::canic::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary),
            #[cfg(canic_capability_root_delegation)]
            IssuerRenewal(::canic::dto::auth::RootIssuerRenewalStatusResponse),
            Pool(::canic::dto::pool::CanisterPoolResponse),
            PoolImport(::canic::dto::pool_import::PoolImportStatus),
            PoolImportContext(::canic::dto::pool_import::PoolImportContext),
            PoolRelease(::canic::dto::root::RootPoolReleaseResponse),
            ProvisioningRelease(::canic::dto::root::RootProvisioningReleaseResponse),
            ReplayRelease(::canic::dto::release_receipts::ReplayReleaseResponse),
            StoreOverview(::canic::dto::template::WasmStoreOverviewResponse),
        }
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_status(
            request: RootStatusRequest,
        ) -> Result<RootStatusResponse, ::canic::Error> {
            let prepared = matches!(
                &request,
                RootStatusRequest::ComponentDirectoryHead(_)
                    | RootStatusRequest::ComponentDirectoryPage(_)
                    | RootStatusRequest::ComponentRegistry(_)
                    | RootStatusRequest::ComponentRegistryActivePartition(_)
                    | RootStatusRequest::ComponentRegistryPartition(_)
                    | RootStatusRequest::FleetAuthority
                    | RootStatusRequest::Pool(_)
                    | RootStatusRequest::PoolImport(_)
                    | RootStatusRequest::PoolImportContext
                    | RootStatusRequest::PoolRelease
                    | RootStatusRequest::ProvisioningRelease(_)
                    | RootStatusRequest::ReplayRelease(_)
                    | RootStatusRequest::StoreOverview
            );
            $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_status_variant_allowed(prepared)?;

            match request {
                RootStatusRequest::Admission(page) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_admission_status(page)
                        .map(RootStatusResponse::Admission)
                }
                RootStatusRequest::AuthorityRestore => {
                    $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::status()
                        .map(RootStatusResponse::AuthorityRestore)
                }
                RootStatusRequest::ComponentDirectoryHead(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_directory_head(request)
                        .map(RootStatusResponse::ComponentDirectoryHead)
                }
                RootStatusRequest::ComponentDirectoryPage(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::controller_component_directory_page(request)
                        .map(RootStatusResponse::ComponentDirectoryPage)
                }
                RootStatusRequest::ComponentRegistry(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::local_component_registry_status(request)
                        .map(RootStatusResponse::ComponentRegistry)
                }
                RootStatusRequest::ComponentRegistryActivePartition(request) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::component_registry_active_partition(request)
                        .map(RootStatusResponse::ComponentRegistryActivePartition)
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
                RootStatusRequest::FleetAuthority => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_authority()
                        .map(RootStatusResponse::FleetAuthority)
                }
                RootStatusRequest::FleetState => Ok(RootStatusResponse::FleetState(
                    $crate::__internal::core::api::state::FleetStateQuery::snapshot(),
                )),
                RootStatusRequest::Funding => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_funding_status()
                        .map(RootStatusResponse::Funding)
                }
                RootStatusRequest::FundingRelease(start_after) => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::root_funding_release_status(start_after)
                        .map(RootStatusResponse::FundingRelease)
                }
                RootStatusRequest::Inventory => {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::fleet_subnet_root_canister_summary()
                        .map(RootStatusResponse::Inventory)
                }
                #[cfg(canic_capability_root_delegation)]
                RootStatusRequest::IssuerRenewal(request) => {
                    $crate::__internal::core::api::auth::AuthApi::root_issuer_renewal_status_root(
                        request,
                    )
                    .map(RootStatusResponse::IssuerRenewal)
                }
                RootStatusRequest::PoolImport(identity) => {
                    $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_status(identity)
                        .map(RootStatusResponse::PoolImport)
                }
                RootStatusRequest::PoolImportContext => {
                    $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::import_context()
                        .map(RootStatusResponse::PoolImportContext)
                }
                RootStatusRequest::PoolRelease => {
                    $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::release_status()
                        .map(RootStatusResponse::PoolRelease)
                }
                RootStatusRequest::ProvisioningRelease(start_after) => {
                    $crate::__internal::control_plane::api::component_provisioning::RootComponentProvisioningApi::release_status(start_after)
                        .map(RootStatusResponse::ProvisioningRelease)
                }
                RootStatusRequest::ReplayRelease(start_after) => {
                    $crate::__internal::core::api::observability::ObservabilityApi::release_receipts(start_after)
                        .map(RootStatusResponse::ReplayRelease)
                }
                RootStatusRequest::Pool(request) => {
                    $crate::__internal::control_plane::api::canister_pool::CanisterPoolApi::status(
                        request,
                    )
                    .map(RootStatusResponse::Pool)
                }
                RootStatusRequest::StoreOverview => {
                    ::canic::api::canister::template::WasmStorePublicationApi::overview()
                        .map(RootStatusResponse::StoreOverview)
                }
                        }
        }
        // This read calls Store; keep it separate from ordinary Root status used by updates.
        #[$crate::canic_query(composite, requires(caller::is_controller()))]
        async fn canic_root_fixture_status(
            content_id: [u8; 32],
        ) -> Result<Result<::canic::dto::fixture_provisioning::FixtureSourceStatus,
            ::canic::dto::fixture_provisioning::FixtureStoreError>, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::root_fixture_status(content_id).await
        }
    };
}
