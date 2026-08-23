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
                != $crate::__internal::core::protocol::CANIC_COMMAND
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
            >(&bytes)
            else {
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
        async fn canic_command(
            command: ::canic::dto::fleet_coordinator::CoordinatorCommand,
        ) -> Result<::canic::dto::fleet_coordinator::CoordinatorCommandResponse, ::canic::Error>
        {
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
                    | CoordinatorCommand::PrepareAuthoritySnapshot(_)
                    | CoordinatorCommand::PrepareRootDeletionExecution(_)
                    | CoordinatorCommand::ProvisionComponents(_)
                    | CoordinatorCommand::RemoveRoot(_)
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

        #[$crate::canic_query(public)]
        async fn canic_status(
            request: ::canic::dto::fleet_coordinator::CoordinatorStatusRequest,
        ) -> Result<::canic::dto::fleet_coordinator::CoordinatorStatusResponse, ::canic::Error>
        {
            use ::canic::dto::fleet_coordinator::{
                CoordinatorStatusRequest, CoordinatorStatusResponse,
            };

            let caller = $crate::__internal::cdk::api::msg_caller();
            if matches!(&request, CoordinatorStatusRequest::Registry) {
                $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::authorize_calling_registry_status()?;
            }
            if !matches!(
                &request,
                CoordinatorStatusRequest::Operation(_)
                    | CoordinatorStatusRequest::Overview
                    | CoordinatorStatusRequest::Registry
            ) {
                $crate::__internal::core::access::auth::is_controller(caller)
                    .await
                    .map_err(::canic::Error::from)?;
            }

            match request {
                CoordinatorStatusRequest::AuthorityRestore => {
                    $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::status()
                        .map(CoordinatorStatusResponse::AuthorityRestore)
                }
                CoordinatorStatusRequest::Funding => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_funding_status()
                        .map(CoordinatorStatusResponse::Funding)
                }
                CoordinatorStatusRequest::Operation(request) => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::operation_status(
                        request.operation_id,
                    )
                    .map(CoordinatorStatusResponse::Operation)
                }
                CoordinatorStatusRequest::Overview => {
                    let capabilities = ::std::collections::BTreeSet::from([
                        $crate::__internal::core::role_contract::RoleCapabilityKey::FleetCoordinator,
                    ]);
                    Ok(CoordinatorStatusResponse::Overview(
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
                CoordinatorStatusRequest::Registry => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::registry_for_calling_status()
                        .map(CoordinatorStatusResponse::Registry)
                }
                CoordinatorStatusRequest::RegistryManifest => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::manifest()
                        .map(CoordinatorStatusResponse::RegistryManifest)
                }
                CoordinatorStatusRequest::RegistryVersion => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::version()
                        .map(CoordinatorStatusResponse::RegistryVersion)
                }
                CoordinatorStatusRequest::RootAcknowledgements => {
                    $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_snapshot_acknowledgements()
                        .map(CoordinatorStatusResponse::RootAcknowledgements)
                }
            }
        }
    };
}
