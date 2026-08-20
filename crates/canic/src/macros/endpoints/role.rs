//! Module: macros::endpoints::role
//!
//! Responsibility: materialize the build-resolved role capability set for status discovery.
//! Does not own: capability derivation, endpoint authorization, or status dispatch.
//! Boundary: consumes only closed cfgs emitted by `canic::build!` from the validated role contract.

/// Build the exact typed capability set resolved for this configured canister role.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_compiled_role_capabilities {
    () => {{
        let mut capabilities = ::std::collections::BTreeSet::new();

        #[cfg(not(canic_capability_runtime))]
        compile_error!("configured Canic roles must include the Runtime capability");
        capabilities.insert($crate::__internal::core::role_contract::RoleCapabilityKey::Runtime);

        #[cfg(canic_capability_automatic_topup)]
        capabilities
            .insert($crate::__internal::core::role_contract::RoleCapabilityKey::AutomaticTopup);
        #[cfg(canic_capability_delegated_token_issuer)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::DelegatedTokenIssuer,
        );
        #[cfg(canic_capability_delegated_token_verifier)]
        capabilities.insert(
            $crate::__internal::core::role_contract::RoleCapabilityKey::DelegatedTokenVerifier,
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

/// Emit the cfg-pruned managed status types and their variant-authorizing dispatcher.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_managed_status_endpoint {
    () => {
        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CanisterStatusRequest {
            #[cfg(canic_delegated_token_issuer)]
            ActiveDelegationProof,
            #[cfg(canic_capability_local_application_authorization)]
            ApplicationSession,
            #[cfg(canic_capability_local_application_authorization)]
            ApplicationSessionAudit(::canic::dto::page::PageRequest),
            Binding,
            #[cfg(canic_capability_sharding)]
            Children(::canic::dto::page::PageRequest),
            CycleBalance,
            CycleHistory(::canic::dto::page::PageRequest),
            #[cfg(canic_capability_automatic_topup)]
            CycleTopups(::canic::dto::page::PageRequest),
            #[cfg(canic_delegated_token_issuer)]
            DelegatedToken(::canic::dto::auth::DelegatedTokenGetRequest),
            Health,
            Logs(::canic::dto::role::LogStatusRequest),
            Metrics(::canic::dto::role::MetricsStatusRequest),
            Operation(::canic::dto::role::OperationStatusRequest),
            Overview,
            Readiness,
            Runtime,
            RuntimeWhitelist(::canic::dto::page::PageRequest),
        }

        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CanisterOperationStatusResponse {
            ConfigureRuntime(::canic::dto::role::ComponentRuntimeOperationStatus),
        }

        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CanisterStatusResponse {
            #[cfg(canic_delegated_token_issuer)]
            ActiveDelegationProof(::canic::dto::auth::ActiveDelegationProofStatusResponse),
            #[cfg(canic_capability_local_application_authorization)]
            ApplicationSession(::canic::dto::auth::ApplicationSessionStatus),
            #[cfg(canic_capability_local_application_authorization)]
            ApplicationSessionAudit(::canic::dto::auth::ApplicationSessionAuditResponse),
            Binding(::canic::ids::ManagedCanisterBinding),
            #[cfg(canic_capability_sharding)]
            Children(
                ::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>,
            ),
            CycleBalance(::canic::dto::role::CycleBalanceStatusResponse),
            CycleHistory(
                ::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>,
            ),
            #[cfg(canic_capability_automatic_topup)]
            CycleTopups(
                ::canic::dto::page::Page<::canic::dto::cycles::CycleTopupEvent>,
            ),
            #[cfg(canic_delegated_token_issuer)]
            DelegatedToken(::canic::dto::auth::DelegatedToken),
            Health(::canic::dto::runtime::CanicHealthStatus),
            Logs(::canic::dto::page::Page<::canic::dto::log::LogEntry>),
            Metrics(::canic::dto::page::Page<::canic::dto::metrics::MetricEntry>),
            Operation(CanisterOperationStatusResponse),
            Overview(::canic::dto::role::RoleOverviewResponse),
            Readiness(::canic::dto::runtime::CanicReadinessStatus),
            Runtime(::canic::dto::runtime::CanicRuntimeStatus),
            RuntimeWhitelist(::canic::dto::runtime_whitelist::RuntimeWhitelistStatusResponse),
        }

        #[$crate::canic_query(public)]
        async fn canic_status(
            request: CanisterStatusRequest,
        ) -> Result<CanisterStatusResponse, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller();
            match &request {
                CanisterStatusRequest::Binding
                | CanisterStatusRequest::Health
                | CanisterStatusRequest::Logs(_)
                | CanisterStatusRequest::Readiness
                | CanisterStatusRequest::Runtime => {
                    $crate::__internal::core::access::auth::is_controller(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                }
                CanisterStatusRequest::Operation(_) => {
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                }
                CanisterStatusRequest::RuntimeWhitelist(_) => {
                    $crate::__internal::core::access::auth::is_controller_or_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                }
                #[cfg(canic_delegated_token_issuer)]
                CanisterStatusRequest::ActiveDelegationProof
                | CanisterStatusRequest::DelegatedToken(_) => {}
                #[cfg(canic_capability_local_application_authorization)]
                CanisterStatusRequest::ApplicationSession => {}
                #[cfg(canic_capability_local_application_authorization)]
                CanisterStatusRequest::ApplicationSessionAudit(_) => {
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                }
                CanisterStatusRequest::CycleBalance
                | CanisterStatusRequest::CycleHistory(_)
                | CanisterStatusRequest::Metrics(_)
                | CanisterStatusRequest::Overview => {}
                #[cfg(canic_capability_automatic_topup)]
                CanisterStatusRequest::CycleTopups(_) => {}
                #[cfg(canic_capability_sharding)]
                CanisterStatusRequest::Children(_) => {}
            }

            match request {
                #[cfg(canic_delegated_token_issuer)]
                CanisterStatusRequest::ActiveDelegationProof => {
                    $crate::__internal::core::api::auth::AuthApi::active_delegation_proof_status()
                        .map(CanisterStatusResponse::ActiveDelegationProof)
                }
                #[cfg(canic_capability_local_application_authorization)]
                CanisterStatusRequest::ApplicationSession => {
                    $crate::__internal::core::api::auth::AuthApi::application_session_status()
                        .map(CanisterStatusResponse::ApplicationSession)
                }
                #[cfg(canic_capability_local_application_authorization)]
                CanisterStatusRequest::ApplicationSessionAudit(page) => {
                    $crate::__internal::core::api::auth::AuthApi::application_session_audit(page)
                        .map(CanisterStatusResponse::ApplicationSessionAudit)
                }
                CanisterStatusRequest::Binding => {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::managed_binding()
                        .map(CanisterStatusResponse::Binding)
                }
                #[cfg(canic_capability_sharding)]
                CanisterStatusRequest::Children(page) => Ok(CanisterStatusResponse::Children(
                    $crate::__internal::core::api::topology::children::CanisterChildrenApi::page(
                        page,
                    ),
                )),
                CanisterStatusRequest::CycleBalance => Ok(
                    CanisterStatusResponse::CycleBalance(
                        ::canic::dto::role::CycleBalanceStatusResponse {
                            cycles: $crate::__internal::cdk::api::canister_cycle_balance(),
                        },
                    ),
                ),
                CanisterStatusRequest::CycleHistory(page) => {
                    Ok(CanisterStatusResponse::CycleHistory(
                        $crate::__internal::core::api::cycles::CycleTrackerQuery::page(page),
                    ))
                }
                #[cfg(canic_capability_automatic_topup)]
                CanisterStatusRequest::CycleTopups(page) => {
                    Ok(CanisterStatusResponse::CycleTopups(
                        $crate::__internal::core::api::cycles::CycleTrackerQuery::topups(page),
                    ))
                }
                #[cfg(canic_delegated_token_issuer)]
                CanisterStatusRequest::DelegatedToken(request) => {
                    $crate::__internal::core::api::auth::AuthApi::get_delegated_token(request)
                        .map(CanisterStatusResponse::DelegatedToken)
                }
                CanisterStatusRequest::Health => Ok(CanisterStatusResponse::Health(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::health(Some(
                        $crate::__internal::cdk::api::time(),
                    )),
                )),
                CanisterStatusRequest::Logs(request) => {
                    Ok(CanisterStatusResponse::Logs(
                        $crate::__internal::core::api::log::LogQuery::page(
                            request.crate_name,
                            request.topic,
                            request.min_level,
                            request.page,
                        ),
                    ))
                }
                CanisterStatusRequest::Metrics(request) => {
                    $crate::__canic_role_metrics_status!(request)
                        .map(CanisterStatusResponse::Metrics)
                }
                CanisterStatusRequest::Operation(request) => {
                    $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::operation_status(
                        request.operation_id,
                    )
                    .map(CanisterOperationStatusResponse::ConfigureRuntime)
                    .map(CanisterStatusResponse::Operation)
                }
                CanisterStatusRequest::Overview => Ok(CanisterStatusResponse::Overview(
                    $crate::__canic_role_overview!(),
                )),
                CanisterStatusRequest::Readiness => Ok(CanisterStatusResponse::Readiness(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::readiness(
                        $crate::__internal::cdk::api::time(),
                    ),
                )),
                CanisterStatusRequest::Runtime => Ok(CanisterStatusResponse::Runtime(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::runtime_status(
                        $crate::__internal::cdk::api::time(),
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION"),
                        $crate::VERSION,
                        $crate::__internal::cdk::api::canister_version(),
                    ),
                )),
                CanisterStatusRequest::RuntimeWhitelist(page) => {
                    $crate::__internal::core::api::runtime_whitelist::RuntimeWhitelistApi::status(
                        page,
                    )
                    .map(CanisterStatusResponse::RuntimeWhitelist)
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
        #[derive(
            ::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CanisterStatusRequest {
            #[cfg(canic_capability_sharding)]
            Children(::canic::dto::page::PageRequest),
            CycleBalance,
            CycleHistory(::canic::dto::page::PageRequest),
            #[cfg(canic_capability_automatic_topup)]
            CycleTopups(::canic::dto::page::PageRequest),
            Health,
            Logs(::canic::dto::role::LogStatusRequest),
            Metrics(::canic::dto::role::MetricsStatusRequest),
            Readiness,
            Runtime,
        }

        #[derive(
            ::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CanisterStatusResponse {
            #[cfg(canic_capability_sharding)]
            Children(::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>),
            CycleBalance(::canic::dto::role::CycleBalanceStatusResponse),
            CycleHistory(::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>),
            #[cfg(canic_capability_automatic_topup)]
            CycleTopups(::canic::dto::page::Page<::canic::dto::cycles::CycleTopupEvent>),
            Health(::canic::dto::runtime::CanicHealthStatus),
            Logs(::canic::dto::page::Page<::canic::dto::log::LogEntry>),
            Metrics(::canic::dto::page::Page<::canic::dto::metrics::MetricEntry>),
            Readiness(::canic::dto::runtime::CanicReadinessStatus),
            Runtime(::canic::dto::runtime::CanicRuntimeStatus),
        }

        #[$crate::canic_query(public)]
        async fn canic_status(
            request: CanisterStatusRequest,
        ) -> Result<CanisterStatusResponse, ::canic::Error> {
            let caller = $crate::__internal::cdk::api::msg_caller();
            match &request {
                CanisterStatusRequest::Health
                | CanisterStatusRequest::Logs(_)
                | CanisterStatusRequest::Readiness
                | CanisterStatusRequest::Runtime => {
                    $crate::__internal::core::access::auth::is_controller(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                }
                CanisterStatusRequest::CycleBalance
                | CanisterStatusRequest::CycleHistory(_)
                | CanisterStatusRequest::Metrics(_) => {}
                #[cfg(canic_capability_automatic_topup)]
                CanisterStatusRequest::CycleTopups(_) => {}
                #[cfg(canic_capability_sharding)]
                CanisterStatusRequest::Children(_) => {}
            }

            match request {
                #[cfg(canic_capability_sharding)]
                CanisterStatusRequest::Children(page) => Ok(CanisterStatusResponse::Children(
                    $crate::__internal::core::api::topology::children::CanisterChildrenApi::page(
                        page,
                    ),
                )),
                CanisterStatusRequest::CycleBalance => Ok(CanisterStatusResponse::CycleBalance(
                    ::canic::dto::role::CycleBalanceStatusResponse {
                        cycles: $crate::__internal::cdk::api::canister_cycle_balance(),
                    },
                )),
                CanisterStatusRequest::CycleHistory(page) => {
                    Ok(CanisterStatusResponse::CycleHistory(
                        $crate::__internal::core::api::cycles::CycleTrackerQuery::page(page),
                    ))
                }
                #[cfg(canic_capability_automatic_topup)]
                CanisterStatusRequest::CycleTopups(page) => {
                    Ok(CanisterStatusResponse::CycleTopups(
                        $crate::__internal::core::api::cycles::CycleTrackerQuery::topups(page),
                    ))
                }
                CanisterStatusRequest::Health => Ok(CanisterStatusResponse::Health(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::health(Some(
                        $crate::__internal::cdk::api::time(),
                    )),
                )),
                CanisterStatusRequest::Logs(request) => Ok(CanisterStatusResponse::Logs(
                    $crate::__internal::core::api::log::LogQuery::page(
                        request.crate_name,
                        request.topic,
                        request.min_level,
                        request.page,
                    ),
                )),
                CanisterStatusRequest::Metrics(request) => {
                    $crate::__canic_role_metrics_status!(request)
                        .map(CanisterStatusResponse::Metrics)
                }
                CanisterStatusRequest::Readiness => Ok(CanisterStatusResponse::Readiness(
                    $crate::__internal::core::api::runtime::RuntimeIntrospectionApi::readiness(
                        $crate::__internal::cdk::api::time(),
                    ),
                )),
                CanisterStatusRequest::Runtime => Ok(CanisterStatusResponse::Runtime(
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

/// Emit the cfg-pruned managed command types and their variant-authorizing dispatcher.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_managed_command_endpoint {
    () => {
        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CanisterCommand {
            #[cfg(canic_capability_local_application_authorization)]
            ApplicationSession(::canic::dto::auth::ApplicationSessionCommand),
            ConfigureRuntime(
                ::canic::dto::component_registry::ComponentRuntimeDirectoryPreparationRequest,
            ),
            #[cfg(canic_delegated_token_issuer)]
            InstallDelegationProof(
                ::canic::dto::auth::InstallActiveDelegationProofRequest,
            ),
            #[cfg(canic_delegated_token_issuer)]
            PrepareDelegatedToken(::canic::dto::auth::DelegatedTokenPrepareRequest),
            RespondCapability(::canic::dto::capability::NonrootCyclesCapabilityEnvelopeV1),
            RuntimeWhitelist(::canic::dto::runtime_whitelist::RuntimeWhitelistCommand),
        }

        #[derive(
            ::canic::__internal::candid::CandidType,
            ::canic::__internal::serde::Deserialize,
        )]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CanisterCommandResponse {
            #[cfg(canic_capability_local_application_authorization)]
            ApplicationSession(::canic::dto::auth::ApplicationSessionCommandResponse),
            #[cfg(canic_delegated_token_issuer)]
            InstallDelegationProof(
                ::canic::dto::auth::InstallActiveDelegationProofResponse,
            ),
            OperationAccepted(::canic::dto::role::OperationReceipt),
            #[cfg(canic_delegated_token_issuer)]
            PrepareDelegatedToken(::canic::dto::auth::DelegatedTokenPrepareResponse),
            RespondCapability(::canic::dto::capability::NonrootCyclesCapabilityResponseV1),
            RuntimeWhitelist(
                ::canic::dto::runtime_whitelist::RuntimeWhitelistMutationResponse,
            ),
        }

        #[doc(hidden)]
        fn __canic_inspect_managed_update_message() {
            if $crate::__internal::core::ingress::payload::current_method_name()
                != $crate::__internal::core::protocol::CANIC_COMMAND
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
            public,
            payload(max_bytes = ::canic::__internal::core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES)
        )]
        async fn canic_command(
            command: CanisterCommand,
        ) -> Result<CanisterCommandResponse, ::canic::Error> {
            use CanisterCommand::{ConfigureRuntime, RespondCapability};

            match command {
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
                ConfigureRuntime(request) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    let operation_id = request.operation_id;
                    #[cfg(canic_capability_automatic_topup)]
                    let configure_runtime = $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::configure_component_runtime_with_automatic_topup;
                    #[cfg(not(canic_capability_automatic_topup))]
                    let configure_runtime = $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::configure;
                    let transition = configure_runtime(request)?;
                    if transition.transitioned {
                        __canic_schedule_prepared_activation_init(
                            transition.application_init_args,
                        );
                    }
                    Ok(CanisterCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                #[cfg(canic_delegated_token_issuer)]
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
                #[cfg(canic_delegated_token_issuer)]
                CanisterCommand::PrepareDelegatedToken(request) => {
                    $crate::__internal::core::api::auth::AuthApi::prepare_delegated_token(request)
                        .await
                        .map(CanisterCommandResponse::PrepareDelegatedToken)
                }
                RespondCapability(envelope) => {
                    $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_nonroot(
                        envelope,
                    )
                    .await
                    .map(CanisterCommandResponse::RespondCapability)
                }
                CanisterCommand::RuntimeWhitelist(command) => {
                    let caller = $crate::__internal::cdk::api::msg_caller();
                    $crate::__internal::core::access::auth::is_controller_or_root(caller)
                        .await
                        .map_err(::canic::Error::from)?;
                    $crate::__internal::core::api::runtime_whitelist::RuntimeWhitelistApi::command(
                        command,
                    )
                    .map(CanisterCommandResponse::RuntimeWhitelist)
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
            $crate::__internal::core::ids::CanisterRole::from(env!("CANIC_CANISTER_ROLE")),
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
