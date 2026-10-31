//! Module: macros::endpoints::role
//!
//! Responsibility: materialize the build-resolved role capability set for status discovery.
//! Does not own: capability derivation, endpoint authorization, or status dispatch.
//! Boundary: consumes only closed cfgs emitted by `canic::build!` from the validated role contract.

/// Emit the capability-independent typed public responsiveness query.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_public_health_endpoint {
    () => {
        #[$crate::query(public, on_access_denied = "reject", wire(id = ::canic::protocol::public_health::METHOD_ID, request = ::canic::protocol::public_health::REQUEST_LIMITS, response = ::canic::protocol::public_health::RESPONSE_LIMITS))]
        fn canic_public_health() -> ::canic::dto::public_status::PublicHealth {
            ::canic::__internal::core::api::public_status::PublicStatusApi::health()
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
        $crate::__canic_emit_public_health_endpoint!();
        #[cfg(canic_capability_observability_metrics)]
        $crate::__canic_emit_public_metrics_endpoint!();

        #[cfg(canic_capability_observability_history)]
        $crate::__canic_emit_public_history_endpoint!();

        #[$crate::query(public, wire(id = ::canic::protocol::public_overview::METHOD_ID, request = ::canic::protocol::public_overview::REQUEST_LIMITS, response = ::canic::protocol::public_overview::RESPONSE_LIMITS))]
        fn canic_public_overview() -> Result<::canic::dto::role::RoleOverviewResponse, ::canic::Error> {
            Ok($crate::__canic_role_overview!())
        }

        #[cfg(canic_capability_child_provisioning)]
        #[$crate::query(public, wire(id = ::canic::protocol::public_children::METHOD_ID, request = ::canic::protocol::public_children::REQUEST_LIMITS, response = ::canic::protocol::public_children::RESPONSE_LIMITS))]
        fn canic_public_children(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::children::CanisterChildrenApi::page(page))
        }

        $crate::__canic_emit_cycle_balance_endpoint!(before = []);

        $crate::__canic_emit_protected_health_endpoint!(before = []);
        $crate::__canic_emit_protected_readiness_endpoint!(before = []);
        #[cfg(canic_capability_observability_diagnostics)]
        $crate::__canic_emit_protected_runtime_endpoint!(before = []);
        $crate::__canic_emit_protected_binding_endpoint!(before = []);

        $crate::__canic_emit_child_funding_endpoint!(before = []);

        #[cfg(canic_capability_observability_logs)]
        $crate::__canic_emit_protected_logs_endpoint!(before = []);

        #[cfg(canic_capability_observability_metrics)]
        $crate::__canic_emit_protected_metrics_endpoint!(before = []);

        #[cfg(canic_capability_observability_diagnostics)]
        $crate::__canic_emit_protected_memory_allocations_endpoint!(before = []);

        #[cfg(canic_capability_observability_history)]
        $crate::__canic_emit_cycle_history_endpoint!(before = []);

        #[cfg(all(canic_capability_observability_history, canic_capability_automatic_topup))]
        $crate::__canic_emit_cycle_topups_endpoint!();


        #[cfg(canic_capability_delegated_token_issuer)]
        #[$crate::query(public, wire(id = ::canic::protocol::issuer_proof::status::METHOD_ID, request = ::canic::protocol::issuer_proof::status::REQUEST_LIMITS, response = ::canic::protocol::issuer_proof::status::RESPONSE_LIMITS))]
        fn canic_active_delegation_proof() -> Result<::canic::dto::issuer_proof::ActiveDelegationProofStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::active_delegation_proof_status().map(Into::into)
        }

        #[cfg(canic_capability_caller_authority)]
        #[$crate::update(internal, public, wire(id = ::canic::protocol::caller_authority::PREPARE_ID, request = ::canic::protocol::caller_authority::REQUEST_LIMITS, response = ::canic::protocol::caller_authority::RESPONSE_LIMITS))]
        fn canic_caller_authority_prepare(publication: ::canic::dto::caller_authority_status::CallerAuthorityPublication) -> Result<::canic::dto::caller_authority_status::CallerAuthorityReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                ::canic::ids::EndpointCall { endpoint: ::canic::ids::EndpointId::new(::canic::protocol::caller_authority::PREPARE), kind: ::canic::ids::EndpointCallKind::Update },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_root(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::caller_authority::CallerAuthorityApi::apply(::canic::dto::caller_authority::CallerAuthorityCommand::Prepare(publication.into())).map(Into::into)
        }

        #[cfg(canic_capability_caller_authority)]
        #[$crate::update(internal, public, wire(id = ::canic::protocol::caller_authority::COMMIT_ID, request = ::canic::protocol::caller_authority::REQUEST_LIMITS, response = ::canic::protocol::caller_authority::RESPONSE_LIMITS))]
        fn canic_caller_authority_commit(publication: ::canic::dto::caller_authority_status::CallerAuthorityPublication) -> Result<::canic::dto::caller_authority_status::CallerAuthorityReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                ::canic::ids::EndpointCall { endpoint: ::canic::ids::EndpointId::new(::canic::protocol::caller_authority::COMMIT), kind: ::canic::ids::EndpointCallKind::Update },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_root(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::caller_authority::CallerAuthorityApi::apply(::canic::dto::caller_authority::CallerAuthorityCommand::Commit(publication.into())).map(Into::into)
        }

        #[cfg(canic_capability_caller_authority)]
        #[$crate::update(internal, public, wire(id = ::canic::protocol::caller_authority::COMPLETE_ID, request = ::canic::protocol::caller_authority::REQUEST_LIMITS, response = ::canic::protocol::caller_authority::RESPONSE_LIMITS))]
        fn canic_caller_authority_complete(publication: ::canic::dto::caller_authority_status::CallerAuthorityPublication) -> Result<::canic::dto::caller_authority_status::CallerAuthorityReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                ::canic::ids::EndpointCall { endpoint: ::canic::ids::EndpointId::new(::canic::protocol::caller_authority::COMPLETE), kind: ::canic::ids::EndpointCallKind::Update },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_root(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::caller_authority::CallerAuthorityApi::apply(::canic::dto::caller_authority::CallerAuthorityCommand::Complete(publication.into())).map(Into::into)
        }

        #[cfg(canic_capability_caller_authority)]
        #[$crate::update(internal, public, wire(id = ::canic::protocol::caller_authority::RELEASE_ID, request = ::canic::protocol::caller_authority::REQUEST_LIMITS, response = ::canic::protocol::caller_authority::RESPONSE_LIMITS))]
        fn canic_application_startup_release(publication: ::canic::dto::caller_authority_status::CallerAuthorityPublication) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                ::canic::ids::EndpointCall { endpoint: ::canic::ids::EndpointId::new(::canic::protocol::caller_authority::RELEASE), kind: ::canic::ids::EndpointCallKind::Update },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_root(caller).map_err(::canic::Error::from)?;
            let operation_id = publication.operation_id;
            $crate::__internal::core::api::application_startup::ApplicationStartupApi::release(publication.into())?;
            __canic_schedule_application_startup();
            Ok(::canic::dto::role::OperationReceipt { operation_id })
        }

        #[cfg(canic_capability_caller_authority)]
        #[$crate::query(requires(caller::is_root()), wire(id = ::canic::protocol::caller_authority_status::METHOD_ID, request = ::canic::protocol::caller_authority_status::REQUEST_LIMITS, response = ::canic::protocol::caller_authority_status::RESPONSE_LIMITS))]
        fn canic_caller_authority_status(request: ::canic::dto::role::OperationStatusRequest) -> Result<::canic::dto::caller_authority_status::CallerAuthorityStatus, ::canic::Error> {
            $crate::__internal::core::api::caller_authority::CallerAuthorityApi::status(request.operation_id).map(Into::into)
        }

        #[$crate::query(requires(caller::is_root()), wire(id = ::canic::protocol::runtime_reporting::METHOD_ID, request = ::canic::protocol::runtime_reporting::REQUEST_LIMITS, response = ::canic::protocol::runtime_reporting::RESPONSE_LIMITS))]
        fn canic_runtime_operation(request: ::canic::dto::role::OperationStatusRequest) -> Result<::canic::dto::runtime_reporting::ComponentRuntimeOperationStatus, ::canic::Error> {
            $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::operation_status(request.operation_id).map(Into::into)
        }

        #[cfg(canic_capability_fleet_admission_projection)]
        #[$crate::query(requires(any(caller::is_controller(), caller::is_root())), wire(id = ::canic::protocol::admission_control::managed_status::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_admission_status(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::admission_control::ManagedStatus, ::canic::Error> {
            $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::status(page).map(Into::into)
        }

    };
}

/// Emit the standalone-local status surface without Fleet binding or operation authority.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_local_status_endpoint {
    () => {


        $crate::__canic_emit_public_health_endpoint!();
        #[cfg(canic_capability_observability_metrics)]
        $crate::__canic_emit_public_metrics_endpoint!();

        #[cfg(canic_capability_observability_history)]
        $crate::__canic_emit_public_history_endpoint!();

        #[cfg(canic_capability_child_provisioning)]
        #[$crate::query(public, wire(id = ::canic::protocol::public_children::METHOD_ID, request = ::canic::protocol::public_children::REQUEST_LIMITS, response = ::canic::protocol::public_children::RESPONSE_LIMITS))]
        fn canic_public_children(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::page::Page<::canic::dto::canister::CanisterInfo>, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::children::CanisterChildrenApi::page(page))
        }

        $crate::__canic_emit_cycle_balance_endpoint!(before = []);

        $crate::__canic_emit_protected_health_endpoint!(before = []);
        $crate::__canic_emit_protected_readiness_endpoint!(before = []);
        #[cfg(canic_capability_observability_diagnostics)]
        $crate::__canic_emit_protected_runtime_endpoint!(before = []);

        $crate::__canic_emit_child_funding_endpoint!(before = []);

        #[cfg(canic_capability_observability_logs)]
        $crate::__canic_emit_protected_logs_endpoint!(before = []);

        #[cfg(canic_capability_observability_metrics)]
        $crate::__canic_emit_protected_metrics_endpoint!(before = []);

        #[cfg(canic_capability_observability_history)]
        $crate::__canic_emit_cycle_history_endpoint!(before = []);

        #[cfg(all(canic_capability_observability_history, canic_capability_automatic_topup))]
        $crate::__canic_emit_cycle_topups_endpoint!();

    };
}

/// Emit managed native updates with their capability selection and authority checks.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_managed_command_endpoint {
    () => {
        $crate::__canic_emit_observation_updates!();

        #[cfg(canic_capability_child_provisioning)]
        $crate::__canic_emit_cycles_capability_endpoint!();

        #[cfg(canic_capability_fleet_admission_projection)]
        #[$crate::update(internal, public, wire(id = ::canic::protocol::admission_control::target_prepare::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_admission_prepare(request: ::canic::dto::admission_control::PrepareTargetRequest) -> Result<::canic::dto::admission_control::TargetReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall {
                    endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::admission_control::target_prepare::METHOD),
                    kind: $crate::__internal::contracts::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_root(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::prepare(request.into()).map(Into::into)
        }

        #[cfg(canic_capability_fleet_admission_projection)]
        #[$crate::update(internal, public, wire(id = ::canic::protocol::admission_control::target_activate::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_admission_activate(request: ::canic::dto::fleet_admission::FleetAdmissionActivateTargetRequest) -> Result<::canic::dto::admission_control::TargetReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall {
                    endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::admission_control::target_activate::METHOD),
                    kind: $crate::__internal::contracts::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_root(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::activate(request).map(Into::into)
        }

        #[cfg(canic_capability_fleet_admission_projection)]
        #[$crate::update(internal, public, wire(id = ::canic::protocol::admission_control::target_open::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_admission_open(request: ::canic::dto::fleet_admission::FleetAdmissionOpenTargetRequest) -> Result<::canic::dto::admission_control::TargetReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall {
                    endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::admission_control::target_open::METHOD),
                    kind: $crate::__internal::contracts::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_root(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::open(request).map(Into::into)
        }

        #[$crate::update(internal, public, wire(id = ::canic::protocol::runtime_configuration::METHOD_ID, request = ::canic::protocol::runtime_configuration::REQUEST_LIMITS, response = ::canic::protocol::runtime_configuration::RESPONSE_LIMITS))]
        fn canic_configure_runtime(request: ::canic::dto::runtime_configuration::ComponentRuntimeDirectoryPreparationRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall {
            endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::runtime_configuration::METHOD),
            kind: $crate::__internal::contracts::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_root(caller)
                .map_err(::canic::Error::from)?;
            let operation_id = request.operation_id;
            #[cfg(canic_capability_automatic_topup)]
            let configure_runtime = $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::configure_component_runtime_with_automatic_topup;
            #[cfg(not(canic_capability_automatic_topup))]
            let configure_runtime = $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::configure;
            configure_runtime(request.into())?;
            #[cfg(canic_capability_fleet_admission_projection)]
            $crate::__internal::core::api::fleet_admission_projection::FleetAdmissionProjectionApi::open_fresh()?;
            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_init_nonroot_bootstrap();
            Ok(::canic::dto::role::OperationReceipt { operation_id })
        }

        $crate::__canic_emit_state_cascade_endpoint!();

        #[cfg(canic_capability_delegated_token_issuer)]
        #[$crate::update(internal, public, wire(id = ::canic::protocol::issuer_proof::INSTALL_ID, request = ::canic::protocol::issuer_proof::REQUEST_LIMITS, response = ::canic::protocol::issuer_proof::RESPONSE_LIMITS))]
        fn canic_install_delegation_proof(request: ::canic::dto::issuer_proof::InstallActiveDelegationProofRequest) -> Result<::canic::dto::issuer_proof::InstallActiveDelegationProofResponse, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                ::canic::ids::EndpointCall {
                    endpoint: ::canic::ids::EndpointId::new(::canic::protocol::issuer_proof::INSTALL),
                    kind: ::canic::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_controller(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::auth::AuthApi::install_active_delegation_proof(request.into()).map(Into::into)
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
            _ => Err::<
                $crate::__internal::contracts::dto::page::Page<
                    $crate::__internal::contracts::dto::metrics::MetricEntry,
                >,
                _,
            >(::canic::Error::from_registered(
                ::canic::diagnostics::codes::REQUEST_INVALID,
            )),
        }
    }};
}

/// Emit the cached public metric query without role-dependent union tags.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_public_metrics_endpoint {
    () => {
        #[$crate::query(public, on_access_denied = "reject", wire(id = ::canic::protocol::public_metrics::METHOD_ID, request = ::canic::protocol::public_metrics::REQUEST_LIMITS, response = ::canic::protocol::public_metrics::RESPONSE_LIMITS))]
        fn canic_public_metrics(request: ::canic::dto::public_status::PublicMetricsRequest) -> ::canic::dto::public_status::PublicMetricsSnapshot {
            ::canic::__internal::core::api::public_status::PublicStatusApi::metrics(request)
        }
    };
}

/// Emit cached history without role-dependent union tags.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_public_history_endpoint {
    () => {
        #[$crate::query(public, on_access_denied = "reject", wire(id = ::canic::protocol::public_history::METHOD_ID, request = ::canic::protocol::public_history::REQUEST_LIMITS, response = ::canic::protocol::public_history::RESPONSE_LIMITS))]
        fn canic_public_history(request: ::canic::dto::public_status::PublicHistoryRequest) -> ::canic::dto::public_status::PublicHistorySnapshot {
            ::canic::__internal::core::api::public_status::PublicStatusApi::history(request)
        }
    };
}
