//! Module: macros::endpoints::wasm_store
//!
//! Responsibility: emit local wasm-store endpoint macros for non-root stores.
//! Does not own: template chunk storage, manifest validation, or GC workflow.
//! Boundary: exposes facade macros that delegate immediately to wasm-store APIs.

/// Emit the canonical local wasm-store canister endpoint surface.
#[macro_export]
macro_rules! canic_emit_local_wasm_store_endpoints {
    () => {
        $crate::__canic_fixture_reply_support!();
        #[$crate::canic_update(internal, requires(caller::is_root()), wire(id = ::canic::protocol::store_activation::prepare::METHOD_ID, request = ::canic::protocol::store_activation::REQUEST_LIMITS, response = ::canic::protocol::store_activation::RESPONSE_LIMITS))]
        fn canic_wasm_store_prepare_credential(request: ::canic::dto::fleet_activation::FleetCredentialGenerationRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            let operation_id = request.operation_id;
            $crate::__internal::core::api::fleet_activation::FleetActivationApi::prepare_nonroot_credential_generation(request)?;
            Ok(::canic::dto::role::OperationReceipt { operation_id })
        }

        #[$crate::canic_update(internal, requires(caller::is_root()), wire(id = ::canic::protocol::store_activation::activate::METHOD_ID, request = ::canic::protocol::store_activation::REQUEST_LIMITS, response = ::canic::protocol::store_activation::RESPONSE_LIMITS))]
        fn canic_wasm_store_activate(request: ::canic::dto::fleet_activation::FleetActivationRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            let operation_id = request.operation_id;
            let transition = $crate::__internal::core::api::fleet_activation::FleetActivationApi::activate_nonroot(request)?;
            if transition.transitioned {
                __canic_schedule_prepared_activation_init(transition.application_init_args);
            }
            Ok(::canic::dto::role::OperationReceipt { operation_id })
        }

        $crate::__canic_emit_cycles_capability_endpoint!();
        #[$crate::canic_update(internal, requires(caller::is_root()), wire(id = ::canic::protocol::store_gc::METHOD_ID, request = ::canic::protocol::store_gc::REQUEST_LIMITS, response = ::canic::protocol::store_gc::RESPONSE_LIMITS))]
        fn canic_wasm_store_run_gc(request: ::canic::dto::template::WasmStoreGcRequest) -> Result<::canic::dto::role::OperationReceipt, ::canic::Error> {
            let operation_id = request.operation_id;
            ::canic::api::canister::template::WasmStoreCanisterApi::request_gc(request)?;
            Ok(::canic::dto::role::OperationReceipt { operation_id })
        }

        #[$crate::canic_update(internal, requires(caller::is_root()), wire(id = ::canic::protocol::store_publication::fixture_prepare::METHOD_ID, request = ::canic::protocol::store_publication::REQUEST_LIMITS, response = ::canic::protocol::store_publication::RESPONSE_LIMITS))]
        fn canic_wasm_store_prepare_fixture(request: ::canic::dto::fixture_provisioning::FixtureDescriptor) -> Result<Result<::canic::dto::fixture_provisioning::FixtureSourceStatus, ::canic::dto::fixture_provisioning::FixtureStoreError>, ::canic::Error> {
            $crate::__internal::control_plane::api::fixture_store::FixtureStoreApi::prepare(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_root()), wire(id = ::canic::protocol::store_publication::inspect::METHOD_ID, request = ::canic::protocol::store_publication::REQUEST_LIMITS, response = ::canic::protocol::store_publication::RESPONSE_LIMITS))]
        fn canic_wasm_store_inspect_template(request: ::canic::dto::template::TemplateLookupRequest) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::info(request.template_id, request.version)
        }

        #[$crate::canic_update(internal, requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::store_publication::prepare::METHOD_ID, request = ::canic::protocol::store_publication::REQUEST_LIMITS, response = ::canic::protocol::store_publication::RESPONSE_LIMITS))]
        fn canic_wasm_store_prepare_chunk_set(request: ::canic::dto::template::TemplateChunkSetPrepareInput) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::prepare(request)
        }

        #[$crate::canic_update(internal, requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::store_publication::stage::METHOD_ID, request = ::canic::protocol::store_publication::REQUEST_LIMITS, response = ::canic::protocol::store_publication::RESPONSE_LIMITS))]
        fn canic_wasm_store_stage_manifest(request: ::canic::dto::template::TemplateManifestInput) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::stage_manifest(request)
        }

        $crate::__canic_fixture_reply_endpoint! {
        #[$crate::canic_update(internal, requires(caller::is_root()), wire(id = ::canic::protocol::store_control::grant::METHOD_ID, request = ::canic::protocol::store_control::REQUEST_LIMITS, response = ::canic::protocol::store_control::RESPONSE_LIMITS))]
        fn canic_wasm_store_set_fixture_grant(request: ::canic::dto::store_control::FixtureGrantRequest) -> Result<Result<::canic::dto::runtime_reporting::FixtureGrant, ::canic::dto::fixture_provisioning::FixtureStoreError>, ::canic::Error> {
            let enabled = request.enabled;
            let result = $crate::__internal::control_plane::api::fixture_store::FixtureStoreApi::set_grant(request.into())?;
            if result.is_ok() {
                $crate::__canic_fixture_reply_checkpoint!(if enabled { __CanicFixtureReplyKind::Grant } else { __CanicFixtureReplyKind::Revoke });
            }
            let _ = enabled;
            Ok(result.map(Into::into))
        }

        }

        #[$crate::canic_update(internal, requires(caller::is_root()), wire(id = ::canic::protocol::store_control::reclaim::METHOD_ID, request = ::canic::protocol::store_control::REQUEST_LIMITS, response = ::canic::protocol::store_control::RESPONSE_LIMITS))]
        async fn canic_wasm_store_reclaim_deletion_cycles(request: ::canic::dto::template::WasmStoreDeletionCycleReclamationRequest) -> Result<::canic::dto::store_control::ReclamationResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::reclaim_deletion_cycles(request).await.map(Into::into)
        }

        $crate::__canic_emit_state_cascade_endpoint!();

        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::cascade::topology::METHOD_ID, request = ::canic::protocol::cascade::REQUEST_LIMITS, response = ::canic::protocol::cascade::RESPONSE_LIMITS))]
        async fn canic_synchronize_topology(snapshot: ::canic::dto::cascade_transport::TopologySnapshot) -> Result<(), ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                $crate::__internal::contracts::ids::EndpointCall {
                    endpoint: $crate::__internal::contracts::ids::EndpointId::new(::canic::protocol::cascade::topology::METHOD),
                    kind: $crate::__internal::contracts::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_parent(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::cascade::CascadeApi::sync_topology(snapshot.into()).await
        }

        $crate::__canic_emit_public_health_endpoint!();
        $crate::__canic_emit_public_metrics_endpoint!();

        $crate::__canic_emit_public_history_endpoint!();

        #[$crate::canic_query(public, wire(id = ::canic::protocol::public_overview::METHOD_ID, request = ::canic::protocol::public_overview::REQUEST_LIMITS, response = ::canic::protocol::public_overview::RESPONSE_LIMITS))]
        fn canic_public_overview() -> Result<::canic::dto::role::RoleOverviewResponse, ::canic::Error> {
            let capabilities =
                $crate::__internal::core::role_contract::built_in_role_capabilities(
                    $crate::__internal::core::role_contract::BuiltInRoleKind::WasmStore,
                );
            Ok(
                $crate::__internal::core::api::role::RoleOverviewApi::overview(
                    $crate::__internal::contracts::ids::CanisterRole::from("wasm_store"),
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

        $crate::__canic_emit_cycle_balance_endpoint!(before = []);

        $crate::__canic_emit_cycle_history_endpoint!(before = []);

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::store_status::authority::METHOD_ID, request = ::canic::protocol::store_status::REQUEST_LIMITS, response = ::canic::protocol::store_status::RESPONSE_LIMITS))]
        fn canic_wasm_store_authority() -> Result<::canic::dto::store_status::Authority, ::canic::Error> {
            $crate::__internal::core::api::fleet_activation::FleetActivationApi::wasm_store_authority().map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::store_status::activation::METHOD_ID, request = ::canic::protocol::store_status::REQUEST_LIMITS, response = ::canic::protocol::store_status::RESPONSE_LIMITS))]
        fn canic_wasm_store_activation_status(request: ::canic::dto::role::OperationStatusRequest) -> Result<::canic::dto::runtime_reporting::FleetActivationStatusResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::activation_operation_status(request.operation_id).map(Into::into)
        }

        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::store_status::gc::METHOD_ID, request = ::canic::protocol::store_status::REQUEST_LIMITS, response = ::canic::protocol::store_status::RESPONSE_LIMITS))]
        fn canic_wasm_store_gc_status(request: ::canic::dto::role::OperationStatusRequest) -> Result<::canic::dto::template::WasmStoreGcOperationStatus, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::gc_operation_status(request.operation_id)
        }

        #[$crate::canic_query(requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::store_inventory::catalog::METHOD_ID, request = ::canic::protocol::store_inventory::REQUEST_LIMITS, response = ::canic::protocol::store_inventory::RESPONSE_LIMITS))]
        fn canic_wasm_store_inventory() -> Result<Vec<::canic::dto::template::WasmStoreCatalogEntryResponse>, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::catalog()
        }

        #[$crate::canic_query(requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::store_inventory::storage::METHOD_ID, request = ::canic::protocol::store_inventory::REQUEST_LIMITS, response = ::canic::protocol::store_inventory::RESPONSE_LIMITS))]
        fn canic_wasm_store_storage() -> Result<::canic::dto::template::WasmStoreStatusResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::status()
        }

        #[$crate::canic_query(requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::store_catalog::fixture::METHOD_ID, request = ::canic::protocol::store_catalog::REQUEST_LIMITS, response = ::canic::protocol::store_catalog::RESPONSE_LIMITS))]
        fn canic_wasm_store_fixture_status(content: [u8; 32]) -> Result<Result<::canic::dto::fixture_provisioning::FixtureSourceStatus, ::canic::dto::fixture_provisioning::FixtureStoreError>, ::canic::Error> {
            Ok($crate::__internal::control_plane::api::fixture_store::FixtureStoreApi::source_status(content))
        }

        #[$crate::canic_query(requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::store_catalog::grant::METHOD_ID, request = ::canic::protocol::store_catalog::REQUEST_LIMITS, response = ::canic::protocol::store_catalog::RESPONSE_LIMITS))]
        fn canic_wasm_store_fixture_grant(target: ::canic::__internal::ic_platform::Principal) -> Result<Option<::canic::dto::runtime_reporting::FixtureGrant>, ::canic::Error> {
            Ok($crate::__internal::control_plane::api::fixture_store::FixtureStoreApi::grant_status(target.into()).map(Into::into))
        }

        #[$crate::canic_query(requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::store_catalog::template::METHOD_ID, request = ::canic::protocol::store_catalog::REQUEST_LIMITS, response = ::canic::protocol::store_catalog::RESPONSE_LIMITS))]
        fn canic_wasm_store_template_status(request: ::canic::dto::template::TemplateLookupRequest) -> Result<::canic::dto::template::TemplateStagingStatusResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::staging_status(request)
        }

        #[$crate::canic_update(internal, requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::template_bytes::METHOD_ID, request = ::canic::protocol::template_bytes::REQUEST_LIMITS, response = ::canic::protocol::template_bytes::RESPONSE_LIMITS))]
        fn canic_wasm_store_publish_chunk(
            request: ::canic::dto::template_bytes::PublishRequest,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::publish_chunk(request.into())
        }

        #[$crate::canic_update(internal, requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), wire(id = ::canic::protocol::fixture_bytes::upload::METHOD_ID, request = ::canic::protocol::fixture_bytes::CHUNK_LIMITS, response = ::canic::protocol::fixture_bytes::UPLOAD_RESPONSE_LIMITS))]
        fn canic_wasm_store_publish_fixture(
            request: ::canic::dto::fixture_bytes::UploadRequest,
        ) -> Result<Result<::canic::dto::fixture_provisioning::FixtureSourceStatus, ::canic::dto::fixture_provisioning::FixtureStoreError>, ::canic::Error> {
            $crate::__internal::control_plane::api::fixture_store::FixtureStoreApi::upload(request.into())
        }

        $crate::__canic_fixture_reply_endpoint! {
        #[$crate::canic_update(internal, public, wire(id = ::canic::protocol::fixture_bytes::read::METHOD_ID, request = ::canic::protocol::fixture_bytes::READ_REQUEST_LIMITS, response = ::canic::protocol::fixture_bytes::CHUNK_LIMITS))]
        fn canic_wasm_store_fixture_chunk(
            request: ::canic::dto::fixture_bytes::ReadRequest,
        ) -> Result<Result<::canic::dto::fixture_bytes::ChunkResponse, ::canic::dto::fixture_provisioning::FixtureStoreError>, ::canic::Error> {
            let request: ::canic::dto::fixture_provisioning::FixtureChunkRead = request.into();
            let caller: $crate::__internal::core::cdk::types::Principal = $crate::__internal::cdk::api::msg_caller().into();
            // Framework admission cannot express this request-bound grant. Authenticate
            // its exact caller and full revision here before delegating payload access.
            if let Err(error) = $crate::__internal::control_plane::api::fixture_store::FixtureStoreApi::authorize_read(caller, &request) {
                return Ok(Err(error));
            }
            let result = $crate::__internal::control_plane::api::fixture_store::FixtureStoreApi::read(request);
            if result.is_ok() {
                $crate::__canic_fixture_reply_checkpoint!(__CanicFixtureReplyKind::Chunk);
            }
            Ok(result.map(|bytes| ::canic::dto::fixture_bytes::ChunkResponse { bytes }))
        }
        }

        #[$crate::canic_update(internal, requires(caller::is_root()), wire(id = ::canic::protocol::wasm_store_chunk::METHOD_ID, request = ::canic::protocol::wasm_store_chunk::REQUEST_LIMITS, response = ::canic::protocol::wasm_store_chunk::RESPONSE_LIMITS))]
        fn canic_wasm_store_chunk(
            request: ::canic::dto::template::TemplateChunkRequest,
        ) -> Result<::canic::dto::template::TemplateChunkResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::chunk(
                request.template_id,
                request.version,
                request.chunk_index,
            )
        }
    };
}
