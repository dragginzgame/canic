//! Module: macros::endpoints::role
//!
//! Responsibility: materialize the build-resolved role capability set for status discovery.
//! Does not own: capability derivation, endpoint authorization, or status dispatch.
//! Boundary: consumes only closed cfgs emitted by `canic::build!` from the validated role contract.

/// Emit the response subset that the shared Root relay can decode.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_relay_observability_response {
    () => {
        ::canic::__internal::contracts::__canic_relay_wire_types! {
            serde_crate = "::canic::__internal::serde",
            history_attributes = [#[cfg(canic_capability_observability_history)]],
            topup_history_attributes = [#[cfg(all( canic_capability_observability_history, canic_capability_automatic_topup ))]],
            diagnostics_attributes = [#[cfg(canic_capability_observability_diagnostics)]],
            metrics_attributes = [#[cfg(canic_capability_observability_metrics)]],
        }
    };
}

/// Build the exact typed capability set resolved for this configured canister role.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_compiled_role_capabilities {
    () => {{
        let mut capabilities = ::std::collections::BTreeSet::new();

        #[cfg(not(canic_capability_runtime))]
        compile_error!("configured Canic roles must include the Runtime capability");
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::Runtime);

        #[cfg(canic_capability_caller_authority)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::CallerAuthority);

        #[cfg(canic_capability_automatic_topup)]
        capabilities
            .insert($crate::__internal::core::role_contract::RoleCapabilityKey::AutomaticTopup);
        #[cfg(canic_capability_child_provisioning)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::ChildProvisioning,
        );
        #[cfg(canic_capability_delegated_token_issuer)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::DelegatedTokenIssuer,
        );
        #[cfg(canic_capability_delegated_token_verifier)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::DelegatedTokenVerifier,
        );
        #[cfg(canic_capability_fleet_admission_projection)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::FleetAdmissionProjection,
        );
        #[cfg(canic_capability_fleet_coordinator)]
        capabilities
            .insert($crate::__internal::core::role_contract::RoleCapabilityKey::FleetCoordinator);
        #[cfg(canic_capability_icrc21)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::Icrc21);
        #[cfg(canic_capability_index)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::Index);
        #[cfg(canic_capability_local_application_authorization)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::LocalApplicationAuthorization,
        );
        #[cfg(canic_capability_observability_diagnostics)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::ObservabilityDiagnostics);
        #[cfg(canic_capability_observability_history)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::ObservabilityHistory);
        #[cfg(canic_capability_observability_logs)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::ObservabilityLogs);
        #[cfg(canic_capability_observability_metrics)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::ObservabilityMetrics);
        #[cfg(canic_capability_role_attestation_signer)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::RoleAttestationSigner,
        );
        #[cfg(canic_capability_role_attestation_verifier)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::RoleAttestationVerifier,
        );
        #[cfg(canic_capability_root)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::Root);
        #[cfg(canic_capability_root_delegation)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::RootDelegation);
        #[cfg(canic_capability_root_control_plane)]
        capabilities
            .insert($crate::__internal::core::role_contract::RoleCapabilityKey::RootControlPlane);
        #[cfg(canic_capability_scaling)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::Scaling);
        #[cfg(canic_capability_sharding)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::Sharding);
        #[cfg(canic_capability_wasm_store)]
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::WasmStore);

        capabilities
    }};
}

/// Emit the cfg-pruned managed status types and their uniformly authorized dispatchers.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_managed_status_endpoint {
    () => {
        ::canic::__internal::contracts::__canic_managed_wire_types! {
            serde_crate = "::canic::__internal::serde",
            metrics_attributes = [#[cfg(canic_capability_observability_metrics)]],
            history_attributes = [#[cfg(canic_capability_observability_history)]],
            child_provisioning_attributes = [#[cfg(canic_capability_child_provisioning)]],
            automatic_topup_attributes = [#[cfg(canic_capability_automatic_topup)]],
            logs_attributes = [#[cfg(canic_capability_observability_logs)]],
            diagnostics_attributes = [#[cfg(canic_capability_observability_diagnostics)]],
            local_auth_attributes = [#[cfg(any( canic_capability_delegated_token_issuer, canic_capability_local_application_authorization ))]],
            token_issuer_attributes = [#[cfg(canic_capability_delegated_token_issuer)]],
            application_authorization_attributes = [#[cfg(canic_capability_local_application_authorization)]],
            caller_authority_attributes = [#[cfg(canic_capability_caller_authority)]],
            fleet_admission_attributes = [#[cfg(canic_capability_fleet_admission_projection)]],
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
                PublicStatusRequest::Overview => Ok(PublicStatusResponse::Overview(
                    $crate::__canic_role_overview!(),
                )),
                #[cfg(canic_capability_child_provisioning)]
                PublicStatusRequest::Children(page) => Ok(PublicStatusResponse::Children(
                    $crate::__internal::core::api::topology::children::CanisterChildrenApi::page(
                        page,
                    ),
                )),
            }
        }


        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_observability(
            request: ObservabilityRequest,
        ) -> Result<ObservabilityResponse, ::canic::Error> {
            match request {
                ObservabilityRequest::Binding => {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::managed_binding()
                        .map(ObservabilityResponse::Binding)
                }
                ObservabilityRequest::ChildFunding(child) => {
                    $crate::__internal::core::api::observability::ObservabilityApi::child_funding(child)
                        .map(ObservabilityResponse::ChildFunding)
                }
                ObservabilityRequest::CycleBalance => Ok(
                    ObservabilityResponse::CycleBalance(
                        ::canic::dto::role::CycleBalanceStatusResponse {
                            cycles: $crate::__internal::cdk::api::canister_cycle_balance(),
                        },
                    ),
                ),

                #[cfg(canic_capability_observability_history)]
                ObservabilityRequest::CycleHistory(page) => {
                    Ok(ObservabilityResponse::CycleHistory(
                        $crate::__internal::core::api::cycles::CycleTrackerQuery::page(page),
                    ))
                }
                #[cfg(canic_capability_automatic_topup)]

                #[cfg(canic_capability_observability_history)]
                ObservabilityRequest::CycleTopups(page) => {
                    Ok(ObservabilityResponse::CycleTopups(
                        $crate::__internal::core::api::cycles::CycleTrackerQuery::topups(page),
                    ))
                }
                ObservabilityRequest::Health => Ok(ObservabilityResponse::Health(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::health(Some(
                        $crate::__internal::cdk::api::time(),
                    )),
                )),

                #[cfg(canic_capability_observability_logs)]
                ObservabilityRequest::Logs(request) => {
                    Ok(ObservabilityResponse::Logs(
                        $crate::__internal::core::api::log::LogQuery::page(
                            request.crate_name,
                            request.topic,
                            request.min_level,
                            request.page,
                        ),
                    ))
                }

                #[cfg(canic_capability_observability_diagnostics)]
                ObservabilityRequest::MemoryAllocations => {
                    $crate::__internal::core::api::memory::MemoryQuery::allocations()
                        .map(ObservabilityResponse::MemoryAllocations)
                }

                #[cfg(canic_capability_observability_metrics)]
                ObservabilityRequest::Metrics(request) => {
                    $crate::__canic_role_metrics_status!(request)
                        .map(ObservabilityResponse::Metrics)
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


        #[cfg(any(
            canic_capability_delegated_token_issuer,
            canic_capability_local_application_authorization
        ))]
        #[$crate::canic_query(public)]
        async fn canic_auth_status(
            request: AuthStatusRequest,
        ) -> Result<AuthStatusResponse, ::canic::Error> {
            match request {
                #[cfg(canic_capability_delegated_token_issuer)]
                AuthStatusRequest::ActiveDelegationProof => {
                    $crate::__internal::core::api::auth::AuthApi::active_delegation_proof_status()
                        .map(AuthStatusResponse::ActiveDelegationProof)
                }
                #[cfg(canic_capability_local_application_authorization)]
                AuthStatusRequest::ApplicationSession => {
                    $crate::__internal::core::api::auth::AuthApi::application_session_status()
                        .map(AuthStatusResponse::ApplicationSession)
                }
                #[cfg(canic_capability_delegated_token_issuer)]
                AuthStatusRequest::DelegatedToken(request) => {
                    $crate::__internal::core::api::auth::AuthApi::get_delegated_token(request)
                        .map(AuthStatusResponse::DelegatedToken)
                }
            }
        }


        #[$crate::canic_query(requires(caller::is_root()))]
        async fn canic_control_status(
            request: ControlStatusRequest,
        ) -> Result<ControlStatusResponse, ::canic::Error> {
            match request {
                #[cfg(canic_capability_local_application_authorization)]
                ControlStatusRequest::ApplicationSessionAudit(page) => {
                    $crate::__internal::core::api::auth::AuthApi::application_session_audit(page)
                        .map(ControlStatusResponse::ApplicationSessionAudit)
                }
                #[cfg(canic_capability_caller_authority)]
                ControlStatusRequest::CallerAuthority(request) => {
                    $crate::__internal::core::api::caller_authority::CallerAuthorityApi::status(request.operation_id)
                        .map(ControlStatusResponse::CallerAuthority)
                }
                ControlStatusRequest::Operation(request) => {
                    $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::operation_status(
                        request.operation_id,
                    )
                    .map(CanisterOperationStatusResponse::ConfigureRuntime)
                    .map(ControlStatusResponse::Operation)
                }
            }
        }


        #[cfg(canic_capability_fleet_admission_projection)]
        #[$crate::canic_query(requires(any(caller::is_controller(), caller::is_root())))]
        async fn canic_admission_status(
            request: AdmissionStatusRequest,
        ) -> Result<AdmissionStatusResponse, ::canic::Error> {
            match request {
                #[cfg(canic_capability_fleet_admission_projection)]
                AdmissionStatusRequest::Admission(page) => {
                    $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::status(page)
                        .map(AdmissionStatusResponse::Admission)
                }
            }
        }
    };
}

/// Emit the standalone-local status surface without Fleet binding or operation authority.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_local_status_endpoint {
    () => {
        ::canic::__internal::contracts::__canic_local_wire_types! {
            serde_crate = "::canic::__internal::serde",
            metrics_attributes = [#[cfg(canic_capability_observability_metrics)]],
            history_attributes = [#[cfg(canic_capability_observability_history)]],
            child_provisioning_attributes = [#[cfg(canic_capability_child_provisioning)]],
            automatic_topup_attributes = [#[cfg(canic_capability_automatic_topup)]],
            logs_attributes = [#[cfg(canic_capability_observability_logs)]],
            diagnostics_attributes = [#[cfg(canic_capability_observability_diagnostics)]],
        }

        #[$crate::canic_query(public)]
        async fn canic_public_status(
            request: PublicStatusRequest,
        ) -> Result<PublicStatusResponse, ::canic::Error> {
            match request {
                PublicStatusRequest::Health => Ok(PublicStatusResponse::Health(
                    ::canic::__internal::core::api::public_status::PublicStatusApi::health(),
                )),

                #[cfg(canic_capability_observability_history)]
                PublicStatusRequest::History(request) => Ok(PublicStatusResponse::History(
                    ::canic::__internal::core::api::public_status::PublicStatusApi::history(
                        request,
                    ),
                )),

                #[cfg(canic_capability_observability_metrics)]
                PublicStatusRequest::Metrics(request) => Ok(PublicStatusResponse::Metrics(
                    ::canic::__internal::core::api::public_status::PublicStatusApi::metrics(
                        request,
                    ),
                )),
                #[cfg(canic_capability_child_provisioning)]
                PublicStatusRequest::Children(page) => Ok(PublicStatusResponse::Children(
                    $crate::__internal::core::api::topology::children::CanisterChildrenApi::page(
                        page,
                    ),
                )),
            }
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_observability(
            request: ObservabilityRequest,
        ) -> Result<ObservabilityResponse, ::canic::Error> {
            match request {
                ObservabilityRequest::ChildFunding(child) => {
                    $crate::__internal::core::api::observability::ObservabilityApi::child_funding(
                        child,
                    )
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
                #[cfg(canic_capability_automatic_topup)]
                #[cfg(canic_capability_observability_history)]
                ObservabilityRequest::CycleTopups(page) => Ok(ObservabilityResponse::CycleTopups(
                    $crate::__internal::core::api::cycles::CycleTrackerQuery::topups(page),
                )),
                ObservabilityRequest::Health => Ok(ObservabilityResponse::Health(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::health(Some(
                        $crate::__internal::cdk::api::time(),
                    )),
                )),

                #[cfg(canic_capability_observability_logs)]
                ObservabilityRequest::Logs(request) => Ok(ObservabilityResponse::Logs(
                    $crate::__internal::core::api::log::LogQuery::page(
                        request.crate_name,
                        request.topic,
                        request.min_level,
                        request.page,
                    ),
                )),

                #[cfg(canic_capability_observability_metrics)]
                ObservabilityRequest::Metrics(request) => {
                    $crate::__canic_role_metrics_status!(request)
                        .map(ObservabilityResponse::Metrics)
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
    };
}

/// Emit the cfg-pruned managed command types and their uniformly authorized dispatchers.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_managed_command_endpoint {
    () => {
        $crate::__canic_emit_relay_observability_response!();

        ::canic::__internal::contracts::__canic_managed_command_wire_types! {
            serde_crate = "::canic::__internal::serde",
            fleet_admission_attributes = [#[cfg(canic_capability_fleet_admission_projection)]],
            application_authorization_attributes = [#[cfg(canic_capability_local_application_authorization)]],
            caller_authority_attributes = [#[cfg(canic_capability_caller_authority)]],
            token_issuer_attributes = [#[cfg(canic_capability_delegated_token_issuer)]],
            child_provisioning_attributes = [#[cfg(canic_capability_child_provisioning)]],
        }


        #[doc(hidden)]
        fn __canic_inspect_managed_update_message() {
            if $crate::__internal::core::ingress::payload::current_method_name()
                != $crate::__internal::contracts::protocol::CANIC_COMMAND
            {
                $crate::__internal::core::ingress::payload::inspect_update_message();
                return;
            }

            let bytes = $crate::__internal::core::ingress::payload::current_payload_bytes();
            if !$crate::__internal::core::ingress::payload::payload_within_limit(
                bytes.len(),
                $crate::__internal::core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
            ) {
                return;
            }
            if ::canic::__internal::candid::decode_one::<CanisterCommand>(&bytes).is_ok() {
                $crate::__internal::core::ingress::payload::accept_current_message();
            }
        }

        #[$crate::canic_update(
            internal,
            public,
            payload(max_bytes = ::canic::__internal::core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES)
        )]
        async fn canic_command(
            command: CanisterCommand,
        ) -> Result<CanisterCommandResponse, ::canic::Error> {
            if !matches!(&command, CanisterCommand::SynchronizeState(_)) {
                $crate::__internal::core::access::expr::eval_default_fleet_guard(
                    $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                    $crate::__internal::contracts::ids::EndpointCall {
                        endpoint: $crate::__internal::contracts::ids::EndpointId::new(
                            $crate::__internal::contracts::protocol::CANIC_COMMAND,
                        ),
                        kind: $crate::__internal::contracts::ids::EndpointCallKind::Update,
                    },
                )
                    .map_err(::canic::Error::from)?;
            }
            match command {
                #[cfg(canic_capability_fleet_admission_projection)]
                CanisterCommand::ActivateFleetAdmission(request) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::activate(request)
                        .map(CanisterCommandResponse::ActivateFleetAdmission)
                }
                #[cfg(canic_capability_local_application_authorization)]
                CanisterCommand::ApplicationSession(command) => {
                    let response = match command {
                        ::canic::dto::auth::ApplicationSessionCommand::Establish(request) => {
                            $crate::__internal::core::api::auth::AuthApi::establish_application_session(request)?
                        }
                        ::canic::dto::auth::ApplicationSessionCommand::Clear => {
                            $crate::__internal::core::api::auth::AuthApi::clear_application_session()?
                        }
                    };
                    Ok(CanisterCommandResponse::ApplicationSession(response))
                }
                #[cfg(canic_capability_caller_authority)]
                CanisterCommand::CallerAuthority(request) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await.map_err(::canic::Error::from)?;
                    $crate::__internal::core::api::caller_authority::CallerAuthorityApi::apply(request)
                        .map(CanisterCommandResponse::CallerAuthority)
                }
                #[cfg(canic_capability_caller_authority)]
                CanisterCommand::ReleaseApplicationStartup(publication) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await.map_err(::canic::Error::from)?;
                    let operation_id = publication.operation_id;
                    $crate::__internal::core::api::application_startup::ApplicationStartupApi::release(publication)?;
                    __canic_schedule_application_startup();
                    Ok(CanisterCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                CanisterCommand::ConfigureRuntime(request) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    let operation_id = request.operation_id;
                    #[cfg(canic_capability_automatic_topup)]
                    let configure_runtime = $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::configure_component_runtime_with_automatic_topup;
                    #[cfg(not(canic_capability_automatic_topup))]
                    let configure_runtime = $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::configure;
                    configure_runtime(request)?;
                    #[cfg(canic_capability_fleet_admission_projection)]
                    $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::open_fresh()?;
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_init_nonroot_bootstrap();
                    Ok(CanisterCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                #[cfg(canic_capability_delegated_token_issuer)]
                CanisterCommand::InstallDelegationProof(request) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_controller(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    $crate::__internal::core::api::auth::AuthApi::install_active_delegation_proof(
                        request,
                    )
                    .map(CanisterCommandResponse::InstallDelegationProof)
                }

                CanisterCommand::Observe(request) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_controller(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    $crate::__canic_sensitive_observability_response!(request, RelayedObservabilityResponse)
                        .map(CanisterCommandResponse::Observe)
                }
                #[cfg(canic_capability_fleet_admission_projection)]
                CanisterCommand::OpenFleetAdmission(request) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::open(request)
                        .map(CanisterCommandResponse::OpenFleetAdmission)
                }
                #[cfg(canic_capability_delegated_token_issuer)]
                CanisterCommand::PrepareDelegatedToken(request) => {
                    $crate::__internal::core::api::auth::AuthApi::prepare_delegated_token(request)
                        .await
                        .map(CanisterCommandResponse::PrepareDelegatedToken)
                }
                #[cfg(canic_capability_fleet_admission_projection)]
                CanisterCommand::PrepareFleetAdmission(request) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::prepare(request)
                        .map(CanisterCommandResponse::PrepareFleetAdmission)
                }
                #[cfg(canic_capability_child_provisioning)]
                CanisterCommand::RespondCapability(envelope) => {
                    $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_nonroot(
                        envelope,
                    )
                    .await
                        .map(CanisterCommandResponse::RespondCapability)
                }
                CanisterCommand::SynchronizeState(snapshot) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_parent(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    $crate::__internal::core::api::cascade::CascadeApi::sync_state(snapshot)
                        .await
                        .map(CanisterCommandResponse::SynchronizeState)
                }
            }
        }
    };
}

/// Decode the digest embedded by the canonical profile-bound artifact build.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_protocol_profile_digest {
    () => {{
        let Some(__canic_protocol_profile_digest) = option_env!("CANIC_PROTOCOL_PROFILE_DIGEST")
        else {
            panic!("canonical role artifact must embed its protocol-profile digest");
        };
        __canic_protocol_profile_digest
            .parse::<$crate::__internal::core::role_contract::ProtocolProfileDigest>()
            .expect("embedded protocol-profile digest must be canonical lowercase SHA-256")
            .into_bytes()
    }};
}

/// Build the immutable overview from the same cfg authority that prunes the surface.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_role_overview {
    () => {{
        let capabilities = $crate::__canic_compiled_role_capabilities!();
        $crate::__internal::core::api::role::RoleOverviewApi::overview(
            $crate::__internal::contracts::ids::CanisterRole::from(env!("CANIC_CANISTER_ROLE")),
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
        )
    }};
}

/// Dispatch one metrics request through only the compiled metric families.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_role_metrics_status {
    ($request:expr) => {{
        let request = $request;
        match request.kind {
            #[cfg(canic_metrics_core)]
            ::canic::dto::metrics::MetricsKind::Core => Ok(
                $crate::__internal::core::api::metrics::MetricsQuery::core(request.page),
            ),
            #[cfg(canic_metrics_placement)]
            ::canic::dto::metrics::MetricsKind::Placement => {
                Ok($crate::__internal::core::api::metrics::MetricsQuery::placement(request.page))
            }
            #[cfg(canic_metrics_platform)]
            ::canic::dto::metrics::MetricsKind::Platform => {
                Ok($crate::__internal::core::api::metrics::MetricsQuery::platform(request.page))
            }
            #[cfg(canic_metrics_runtime)]
            ::canic::dto::metrics::MetricsKind::Runtime => {
                Ok($crate::__internal::core::api::metrics::MetricsQuery::runtime(request.page))
            }
            #[cfg(canic_metrics_security)]
            ::canic::dto::metrics::MetricsKind::Security => {
                Ok($crate::__internal::core::api::metrics::MetricsQuery::security(request.page))
            }
            #[cfg(canic_metrics_storage)]
            ::canic::dto::metrics::MetricsKind::Storage => {
                Ok($crate::__internal::core::api::metrics::MetricsQuery::storage(request.page))
            }
            _ => Err(::canic::Error::from_registered(
                ::canic::diagnostics::codes::REQUEST_INVALID,
            )),
        }
    }};
}

/// Dispatch exact cycle and performance observations after endpoint-owned authorization.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_sensitive_observability_response {
    ($request:expr, $response:ident) => {{
        match $request {
            ::canic::dto::observability::CanisterObservabilityRequest::ChildFunding(child) => {
                $crate::__internal::core::api::observability::ObservabilityApi::child_funding(child)
                    .map($response::ChildFunding)
            }
            ::canic::dto::observability::CanisterObservabilityRequest::CycleBalance => Ok(
                $response::CycleBalance(::canic::dto::role::CycleBalanceStatusResponse {
                    cycles: $crate::__internal::cdk::api::canister_cycle_balance(),
                }),
            ),
            #[cfg(canic_capability_observability_history)]
            ::canic::dto::observability::CanisterObservabilityRequest::CycleHistory(page) => {
                Ok($response::CycleHistory(
                    $crate::__internal::core::api::cycles::CycleTrackerQuery::page(page),
                ))
            }
            #[cfg(all(
                canic_capability_observability_history,
                canic_capability_automatic_topup
            ))]
            ::canic::dto::observability::CanisterObservabilityRequest::CycleTopups(page) => {
                Ok($response::CycleTopups(
                    $crate::__internal::core::api::cycles::CycleTrackerQuery::topups(page),
                ))
            }
            #[cfg(canic_capability_observability_diagnostics)]
            ::canic::dto::observability::CanisterObservabilityRequest::MemoryAllocations => {
                $crate::__internal::core::api::memory::MemoryQuery::allocations()
                    .map($response::MemoryAllocations)
            }
            #[cfg(canic_capability_observability_metrics)]
            ::canic::dto::observability::CanisterObservabilityRequest::Metrics(request) => {
                $crate::__canic_role_metrics_status!(request).map($response::Metrics)
            }
            #[cfg(not(all(
                canic_capability_observability_history,
                canic_capability_automatic_topup,
                canic_capability_observability_diagnostics,
                canic_capability_observability_metrics
            )))]
            _ => Err(::canic::Error::from_registered(
                ::canic::diagnostics::codes::REQUEST_INVALID,
            )),
        }
    }};
}
