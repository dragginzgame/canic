//! Module: macros::endpoints::fleet_coordinator
//!
//! Responsibility: emit the dedicated Fleet Coordinator role surface.
//! Does not own: Registry state, validation, lifecycle orchestration, or root behavior.
//! Boundary: every export delegates immediately to the Coordinator API facade.

/// Emit the Fleet Coordinator's exact command and status methods.
#[macro_export]
macro_rules! canic_emit_fleet_coordinator_endpoints {
    () => {
        #[doc(hidden)]
        const fn __canic_fleet_coordinator_payload_max_bytes(
            command: &::canic::dto::fleet_coordinator::CoordinatorCommand,
        ) -> usize {
            match command {
                ::canic::dto::fleet_coordinator::CoordinatorCommand::ProvisionComponents(_) => {
                    ::canic::__internal::core::control_plane_support::ops::component_provisioning_plan::MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES
                }
                ::canic::dto::fleet_coordinator::CoordinatorCommand::RequestRootFunding(_) => {
                    ::canic::dto::fleet_funding::MAX_FLEET_ROOT_FUNDING_COMMAND_PAYLOAD_BYTES
                }
                _ => {
                    ::canic::__internal::core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES
                }
            }
        }

        #[doc(hidden)]
        fn __canic_inspect_fleet_coordinator_update_message() {
            if $crate::__internal::core::ingress::payload::current_method_name()
                != $crate::__internal::core::protocol::CANIC_COORDINATOR_COMMAND
            {
                $crate::__internal::core::ingress::payload::inspect_update_message();
                return;
            }

            let bytes = $crate::__internal::core::ingress::payload::current_payload_bytes();
            if bytes.len()
                > ::canic::__internal::core::control_plane_support::ops::component_provisioning_plan::MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES
            {
                return;
            }
            let Ok(command) = ::canic::__internal::candid::decode_one::<
                ::canic::dto::fleet_coordinator::CoordinatorCommand,
            >(&bytes) else {
                return;
            };
            if $crate::__internal::core::ingress::payload::payload_within_limit(
                bytes.len(),
                __canic_fleet_coordinator_payload_max_bytes(&command),
            ) {
                $crate::__internal::core::ingress::payload::accept_current_message();
            }
        }

        #[$crate::canic_update(
            public,
            payload(max_bytes = ::canic::__internal::core::control_plane_support::ops::component_provisioning_plan::MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CANONICAL_BYTES)
        )]
        async fn canic_coordinator_command(
            command: ::canic::dto::fleet_coordinator::CoordinatorCommand,
        ) -> Result<::canic::dto::fleet_coordinator::CoordinatorCommandResponse, ::canic::Error> {
            use ::canic::dto::fleet_coordinator::CoordinatorCommand;

            if !$crate::__internal::core::ingress::payload::payload_within_limit(
                $crate::__internal::cdk::raw::msg_arg_data_size(),
                __canic_fleet_coordinator_payload_max_bytes(&command),
            ) {
                return Err(::canic::Error::from_registered(
                    $crate::__internal::core::diagnostics::codes::REQUEST_CAPACITY,
                ));
            }

            let caller = $crate::__internal::cdk::api::msg_caller();
            if matches!(&command, CoordinatorCommand::AcknowledgeRootSnapshot(_)) {
                $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::authorize_calling_root_snapshot()?;
            }
            if matches!(&command, CoordinatorCommand::RequestRootFunding(_)) {
                $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::authorize_calling_root_funding()?;
            }
            let controller_command = matches!(
                &command,
                CoordinatorCommand::ActivateRegistry(_)
                    | CoordinatorCommand::ApplyFundingPolicyRotation(_)
                    | CoordinatorCommand::BeginFundingPolicyRotation(_)
                    | CoordinatorCommand::CompleteRootDeletion(_)
                    | CoordinatorCommand::JoinRoot(_)
                    | CoordinatorCommand::MutateAdmission(_)
                    | CoordinatorCommand::PrepareAuthoritySnapshot(_)
                    | CoordinatorCommand::PrepareRootDeletionExecution(_)
                    | CoordinatorCommand::ProvisionComponents(_)
                    | CoordinatorCommand::RemoveRoot(_)
                    | CoordinatorCommand::Retire(_)
                    | CoordinatorCommand::ResumeAuthoritySnapshot(_)
                    | CoordinatorCommand::SetRootFunding(_)
                    | CoordinatorCommand::StageFundingPolicyRotationRoot(_)
            );
            if controller_command {
                $crate::__internal::core::access::auth::is_controller(caller)
                    .await
                    .map_err(::canic::Error::from)?;
            }
            let recovery_command = matches!(
                &command,
                CoordinatorCommand::PrepareAuthoritySnapshot(_)
                    | CoordinatorCommand::ResumeAuthoritySnapshot(_)
            );
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(
                recovery_command,
            )?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::command(
                command,
            )
            .await
        }

        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum PublicStatusRequest {
            Health,
            Overview,
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum PublicStatusResponse {
            Health(::canic::dto::public_status::PublicHealth),
            Overview(::canic::dto::role::RoleOverviewResponse),
        }
        #[$crate::canic_query(public)]
        async fn canic_public_status(
            request: PublicStatusRequest,
        ) -> Result<PublicStatusResponse, ::canic::Error> {
            match request {
                PublicStatusRequest::Health => Ok(PublicStatusResponse::Health(
                    ::canic::__internal::core::api::public_status::PublicStatusApi::health(),
                )),
                PublicStatusRequest::Overview => {
                    let capabilities = ::std::collections::BTreeSet::from([
                        $crate::__internal::core::role_contract::RoleCapabilityKey::FleetCoordinator,
                    ]);
                    Ok(PublicStatusResponse::Overview(
                        $crate::__internal::core::api::role::RoleOverviewApi::overview(
                            $crate::__internal::core::ids::CanisterRole::from("fleet_coordinator"),
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
            }
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum ObservabilityRequest {
            Admission(::canic::dto::fleet_admission::FleetAdmissionStatusRequest),
            AuthorityRestore,
            Funding,
            RegistryManifest,
            RegistryVersion,
            RootAcknowledgements,
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum ObservabilityResponse {
            Admission(::canic::dto::fleet_admission::FleetAdmissionStatusResponse),
            AuthorityRestore(::canic::dto::authority_restore::AuthorityRestoreFenceStatusResponse),
            Funding(::canic::dto::fleet_coordinator::CoordinatorFundingStatusResponse),
            RegistryManifest(::canic::dto::fleet_registry::FleetRegistryManifest),
            RegistryVersion(::canic::dto::fleet_registry::FleetRegistryVersion),
            RootAcknowledgements(Vec<::canic::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgement>),
        }
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_observability(
            request: ObservabilityRequest,
        ) -> Result<ObservabilityResponse, ::canic::Error> {
            match request {
                ObservabilityRequest::Admission(request) => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::admission_status(request)
                        .map(ObservabilityResponse::Admission)
                }
                ObservabilityRequest::AuthorityRestore => {
                    $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::status()
                        .map(ObservabilityResponse::AuthorityRestore)
                }
                ObservabilityRequest::Funding => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_funding_status()
                        .map(ObservabilityResponse::Funding)
                }
                ObservabilityRequest::RegistryManifest => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::manifest()
                        .map(ObservabilityResponse::RegistryManifest)
                }
                ObservabilityRequest::RegistryVersion => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::version()
                        .map(ObservabilityResponse::RegistryVersion)
                }
                ObservabilityRequest::RootAcknowledgements => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_snapshot_acknowledgements()
                        .map(ObservabilityResponse::RootAcknowledgements)
                }
                        }
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CoordinatorOperationReadRequest {
            Operation(::canic::dto::role::OperationStatusRequest),
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CoordinatorOperationReadResponse {
            Operation(::canic::dto::fleet_coordinator::CoordinatorOperationStatusResponse),
        }
        #[$crate::canic_query(public)]
        async fn canic_coordinator_operation_status(
            request: CoordinatorOperationReadRequest,
        ) -> Result<CoordinatorOperationReadResponse, ::canic::Error> {
            match request {
                CoordinatorOperationReadRequest::Operation(request) => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::operation_status(
                        request.operation_id,
                    )
                    .map(CoordinatorOperationReadResponse::Operation)
                }
            }
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CoordinatorRegistryReadRequest {
            Registry,
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum CoordinatorRegistryReadResponse {
            Registry(::canic::dto::fleet_registry::FleetRegistry),
        }
        #[$crate::canic_query(requires(custom(::canic::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorRegistryCallerPredicate)))]
        async fn canic_coordinator_registry(
            request: CoordinatorRegistryReadRequest,
        ) -> Result<CoordinatorRegistryReadResponse, ::canic::Error> {
            match request {
                CoordinatorRegistryReadRequest::Registry => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::registry_for_calling_status()
                        .map(CoordinatorRegistryReadResponse::Registry)
                }
            }
        }
    };
}
