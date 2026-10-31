//! Module: macros::endpoints::fleet_coordinator
//!
//! Responsibility: emit the dedicated Fleet Coordinator role surface.
//! Does not own: Registry state, validation, lifecycle orchestration, or root behavior.
//! Boundary: every export delegates immediately to the Coordinator API facade.

/// Emit the Fleet Coordinator's exact command and status methods.
#[macro_export]
macro_rules! canic_emit_fleet_coordinator_endpoints {
    () => {
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::batch_control::prepare::METHOD_ID, request = ::canic::protocol::batch_control::PLAN_LIMITS, response = ::canic::protocol::batch_control::RESPONSE_LIMITS))]
        fn canic_coordinator_provision_components(request: ::canic::dto::batch_control::PrepareRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::provision_components(request.into())
        }

        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::retirement_control::remove::METHOD_ID, request = ::canic::protocol::retirement_control::REQUEST_LIMITS, response = ::canic::protocol::retirement_control::RESPONSE_LIMITS))]
        async fn canic_coordinator_remove_root(request: ::canic::dto::retirement_control::RemovalRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::remove_root(request.into()).await
        }
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::retirement_control::retire::METHOD_ID, request = ::canic::protocol::retirement_control::REQUEST_LIMITS, response = ::canic::protocol::retirement_control::RESPONSE_LIMITS))]
        async fn canic_coordinator_retire(request: ::canic::dto::retirement_control::RetirementRequest) -> Result<::canic::dto::retirement_control::RetirementStatus, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::retire(request.into()).await.map(Into::into)
        }

        #[$crate::update(public, wire(id = ::canic::protocol::funding_control::request::METHOD_ID, request = ::canic::protocol::funding_control::REQUEST_LIMITS, response = ::canic::protocol::funding_control::RESPONSE_LIMITS))]
        async fn canic_coordinator_request_root_funding(request: ::canic::dto::funding_control::Request) -> Result<::canic::dto::funding_control::Response, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::authorize_calling_root_funding()?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::request_calling_root_funding(request.into()).await.map(Into::into)
        }
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::funding_control::begin::METHOD_ID, request = ::canic::protocol::funding_control::REQUEST_LIMITS, response = ::canic::protocol::funding_control::RESPONSE_LIMITS))]
        fn canic_coordinator_begin_funding_policy_rotation(request: ::canic::dto::funding_control::BeginRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {

            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::begin_funding_policy_rotation(request.into())
        }
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::funding_control::stage::METHOD_ID, request = ::canic::protocol::funding_control::REQUEST_LIMITS, response = ::canic::protocol::funding_control::RESPONSE_LIMITS))]
        fn canic_coordinator_stage_funding_policy_rotation_root(request: ::canic::dto::funding_control::StageRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {

            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::stage_funding_policy_rotation_root(request.into())
        }
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::funding_control::apply::METHOD_ID, request = ::canic::protocol::funding_control::REQUEST_LIMITS, response = ::canic::protocol::funding_control::RESPONSE_LIMITS))]
        fn canic_coordinator_apply_funding_policy_rotation(request: ::canic::dto::fleet_funding::FleetFundingPolicyRotationApplyRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {

            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::apply_funding_policy_rotation(request)
        }
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::funding_control::set_enabled::METHOD_ID, request = ::canic::protocol::funding_control::REQUEST_LIMITS, response = ::canic::protocol::funding_control::RESPONSE_LIMITS))]
        fn canic_coordinator_set_root_funding(request: ::canic::dto::state::SetCyclesFundingRequest) -> Result<::canic::dto::state::SetStateResponse<bool>, ::canic::Error> {

            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::set_root_funding(request)
        }

        #[$crate::update(public, wire(id = ::canic::protocol::coordinator_acknowledgements::acknowledge::METHOD_ID, request = ::canic::protocol::coordinator_acknowledgements::acknowledge::REQUEST_LIMITS, response = ::canic::protocol::coordinator_acknowledgements::RESPONSE_LIMITS))]
        fn canic_coordinator_acknowledge_root_snapshot(request: ::canic::dto::coordinator_acknowledgements::AcknowledgeRequest) -> Result<::canic::dto::coordinator_acknowledgements::RootAcknowledgement, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::authorize_calling_root_snapshot()?;
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::acknowledge_calling_root_snapshot(request.into()).map(Into::into)
        }

        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_deletion::prepare::METHOD_ID, request = ::canic::protocol::root_deletion::REQUEST_LIMITS, response = ::canic::protocol::root_deletion::RESPONSE_LIMITS))]
        fn canic_coordinator_prepare_root_deletion(request: ::canic::dto::root_deletion::PrepareRequest) -> Result<::canic::dto::root_deletion::Execution, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::begin_root_deletion_execution(request.into()).map(Into::into)
        }
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_deletion::complete::METHOD_ID, request = ::canic::protocol::root_deletion::REQUEST_LIMITS, response = ::canic::protocol::root_deletion::RESPONSE_LIMITS))]
        fn canic_coordinator_complete_root_deletion(request: ::canic::dto::root_deletion::CompleteRequest) -> Result<::canic::dto::root_deletion::Receipt, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::complete_root_deletion(request.into()).map(Into::into)
        }

        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_control::join::METHOD_ID, request = ::canic::protocol::registry_control::REQUEST_LIMITS, response = ::canic::protocol::registry_control::RESPONSE_LIMITS))]
        fn canic_coordinator_join_root(request: ::canic::dto::registry_control::JoinRequest) -> Result<::canic::dto::registry_control::JoinResponse, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::join_root(request.into()).map(Into::into)
        }
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_control::activate::METHOD_ID, request = ::canic::protocol::registry_control::REQUEST_LIMITS, response = ::canic::protocol::registry_control::RESPONSE_LIMITS))]
        fn canic_coordinator_activate_registry(request: ::canic::dto::registry_control::ActivateRequest) -> Result<::canic::dto::registry_control::ActivateResponse, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::activate_registry(request.into()).map(Into::into)
        }



        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::admission_control::status::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_coordinator_admission(request: ::canic::dto::admission_control::StatusRequest) -> Result<::canic::dto::admission_control::StatusResponse, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::admission_status(request.into()).map(Into::into)
        }

        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::admission_control::mutate::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_coordinator_admission_mutate(request: ::canic::dto::admission_control::MutationRequest) -> Result<::canic::dto::fleet_admission::FleetAdmissionMutationResponse, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::mutate_admission(request.into())
        }

        $crate::__canic_emit_public_health_endpoint!();

        #[$crate::query(public, wire(id = ::canic::protocol::public_overview::METHOD_ID, request = ::canic::protocol::public_overview::REQUEST_LIMITS, response = ::canic::protocol::public_overview::RESPONSE_LIMITS))]
        fn canic_public_overview() -> Result<::canic::dto::role::RoleOverviewResponse, ::canic::Error> {
            let capabilities = ::std::collections::BTreeSet::from([
                $crate::__internal::core::role_contract::RoleCapabilityKey::FleetCoordinator,
            ]);
            Ok(
                $crate::__internal::core::api::role::RoleOverviewApi::overview(
                    $crate::__internal::contracts::ids::CanisterRole::from("fleet_coordinator"),
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

        $crate::__canic_emit_release_evidence_endpoints!(before = []);

        $crate::__canic_emit_authority_snapshot_endpoints!(
            before = [],
            prepare = $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::prepare_authority_snapshot,
            resume = $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::resume_authority_snapshot
        );

        $crate::__canic_emit_authority_restore_status_endpoint!(before = []);

        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_discovery::manifest::METHOD_ID, request = ::canic::protocol::registry_discovery::REQUEST_LIMITS, response = ::canic::protocol::registry_discovery::RESPONSE_LIMITS))]
        fn canic_coordinator_registry_manifest() -> Result<::canic::dto::registry_discovery::RegistryManifest, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::manifest().map(Into::into)
        }

        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::registry_discovery::version::METHOD_ID, request = ::canic::protocol::registry_discovery::REQUEST_LIMITS, response = ::canic::protocol::registry_discovery::RESPONSE_LIMITS))]
        fn canic_coordinator_registry_version() -> Result<::canic::dto::registry_discovery::RegistryVersion, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::version().map(Into::into)
        }

        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::funding_read::coordinator_status::METHOD_ID, request = ::canic::protocol::funding_read::REQUEST_LIMITS, response = ::canic::protocol::funding_read::RESPONSE_LIMITS))]
        fn canic_coordinator_funding() -> Result<::canic::dto::funding_read::CoordinatorStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_funding_status().map(Into::into)
        }

        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::coordinator_acknowledgements::METHOD_ID, request = ::canic::protocol::coordinator_acknowledgements::REQUEST_LIMITS, response = ::canic::protocol::coordinator_acknowledgements::RESPONSE_LIMITS))]
        fn canic_coordinator_root_acknowledgements() -> Result<Vec<::canic::dto::coordinator_acknowledgements::RootAcknowledgement>, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_snapshot_acknowledgements()
                .map(|values| values.into_iter().map(Into::into).collect())
        }

        #[$crate::query(public, wire(id = ::canic::protocol::admission_control::operation::METHOD_ID, request = ::canic::protocol::admission_control::REQUEST_LIMITS, response = ::canic::protocol::admission_control::RESPONSE_LIMITS))]
        fn canic_coordinator_admission_operation(operation_id: [u8; 32]) -> Result<::canic::dto::admission_control::OperationStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::admission_operation_status(operation_id).map(Into::into)
        }

        #[$crate::query(public, wire(id = ::canic::protocol::funding_read::rotation::METHOD_ID, request = ::canic::protocol::funding_read::REQUEST_LIMITS, response = ::canic::protocol::funding_read::RESPONSE_LIMITS))]
        fn canic_coordinator_funding_rotation(operation_id: [u8; 32]) -> Result<::canic::dto::funding_read::RotationStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::funding_rotation_status(operation_id).map(Into::into)
        }

        #[$crate::query(public, wire(id = ::canic::protocol::retirement_control::status::METHOD_ID, request = ::canic::protocol::retirement_control::REQUEST_LIMITS, response = ::canic::protocol::retirement_control::RESPONSE_LIMITS))]
        fn canic_coordinator_retirement_status(operation_id: [u8; 32]) -> Result<::canic::dto::retirement_control::RetirementStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::retirement_operation_status(operation_id).map(Into::into)
        }

        #[$crate::query(public, wire(id = ::canic::protocol::retirement_control::removal_status::METHOD_ID, request = ::canic::protocol::retirement_control::REQUEST_LIMITS, response = ::canic::protocol::retirement_control::RESPONSE_LIMITS))]
        fn canic_coordinator_root_removal(operation_id: [u8; 32]) -> Result<::canic::dto::retirement_control::RemovalStatus, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::root_removal_operation_status(operation_id).map(Into::into)
        }

        #[$crate::query(public, wire(id = ::canic::protocol::component_read::coordinator_provisioning::METHOD_ID, request = ::canic::protocol::component_read::REQUEST_LIMITS, response = ::canic::protocol::component_read::RESPONSE_LIMITS))]
        fn canic_coordinator_component_provisioning(operation_id: [u8; 32]) -> Result<::canic::dto::component_read::CoordinatorProvisioning, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::provisioning_operation_status(operation_id).map(Into::into)
        }

        #[$crate::query(requires(custom(::canic::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorRegistryCallerPredicate)), wire(id = ::canic::protocol::registry_discovery::snapshot::METHOD_ID, request = ::canic::protocol::registry_discovery::REQUEST_LIMITS, response = ::canic::protocol::registry_discovery::RESPONSE_LIMITS))]
        fn canic_coordinator_registry() -> Result<::canic::dto::fleet_snapshot::FleetRegistry, ::canic::Error> {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::registry_for_calling_status().map(Into::into)
        }

    };
}
