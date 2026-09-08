//! Module: macros::endpoints::wasm_store
//!
//! Responsibility: emit local wasm-store endpoint macros for non-root stores.
//! Does not own: template chunk storage, manifest validation, or GC workflow.
//! Boundary: exposes facade macros that delegate immediately to wasm-store APIs.

/// Emit the canonical local wasm-store canister endpoint surface.
#[macro_export]
macro_rules! canic_emit_local_wasm_store_endpoints {
    () => {
        #[doc(hidden)]
        const fn __canic_wasm_store_command_payload_max_bytes(
            command: &::canic::dto::template::StoreCommand,
        ) -> usize {
            match command {
                ::canic::dto::template::StoreCommand::SynchronizeState(_)
                | ::canic::dto::template::StoreCommand::SynchronizeTopology(_) => {
                    ::canic::__internal::core::protocol::CASCADE_SNAPSHOT_MAX_BYTES
                }
                _ => ::canic::__internal::core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
            }
        }

        #[doc(hidden)]
        fn __canic_inspect_wasm_store_update_message() {
            let method = $crate::__internal::core::ingress::payload::current_method_name();
            if method == $crate::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK
                || method == $crate::protocol::CANIC_WASM_STORE_CHUNK
            {
                $crate::__internal::core::ingress::payload::inspect_update_message();
                return;
            }
            if method != $crate::__internal::core::protocol::CANIC_WASM_STORE_COMMAND {
                $crate::__internal::core::ingress::payload::inspect_update_message();
                return;
            }

            let bytes = $crate::__internal::core::ingress::payload::current_payload_bytes();
            if bytes.len() > ::canic::__internal::core::protocol::CASCADE_SNAPSHOT_MAX_BYTES {
                return;
            }
            let Ok(command) =
                ::canic::__internal::candid::decode_one::<::canic::dto::template::StoreCommand>(&bytes)
            else {
                return;
            };
            if $crate::__internal::core::ingress::payload::payload_within_limit(
                bytes.len(),
                __canic_wasm_store_command_payload_max_bytes(&command),
            ) {
                $crate::__internal::core::ingress::payload::accept_current_message();
            }
        }

        #[$crate::canic_update(
            public,
            payload(max_bytes = ::canic::__internal::core::protocol::CASCADE_SNAPSHOT_MAX_BYTES)
        )]
        async fn canic_wasm_store_command(
            command: ::canic::dto::template::StoreCommand,
        ) -> Result<::canic::dto::template::StoreCommandResponse, ::canic::Error> {
            use ::canic::dto::template::{StoreCommand, StoreCommandResponse};

            let caller = $crate::__internal::cdk::api::msg_caller();
            if matches!(
                &command,
                StoreCommand::ActivateFleet(_)
                    | StoreCommand::InspectTemplate(_)
                    | StoreCommand::PrepareFleetCredential(_)
                    | StoreCommand::ReclaimDeletionCycles(_)
                    | StoreCommand::RunGc(_)
            ) {
                $crate::__internal::core::access::auth::is_root(caller)
                    .await
                    .map_err(::canic::Error::from)?;
            }
            if matches!(
                &command,
                StoreCommand::SynchronizeState(_) | StoreCommand::SynchronizeTopology(_)
            ) {
                $crate::__internal::core::access::auth::is_parent(caller)
                    .await
                    .map_err(::canic::Error::from)?;
            }
            if matches!(
                &command,
                StoreCommand::PrepareChunkSet(_) | StoreCommand::StageManifest(_)
            ) {
                use $crate::__internal::core::access::expr::AsyncAccessPredicate as _;
                let context = $crate::__internal::core::access::expr::AccessContext {
                    caller,
                    call: $crate::__internal::core::ids::EndpointCall {
                        endpoint: $crate::__internal::core::ids::EndpointId::new(
                            $crate::__internal::core::protocol::CANIC_WASM_STORE_COMMAND,
                        ),
                        kind: $crate::__internal::core::ids::EndpointCallKind::Update,
                    },
                };
                $crate::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate
                    .eval(&context)
                    .await
                    .map_err(::canic::Error::from)?;
            }

            match command {
                StoreCommand::ActivateFleet(request) => {
                    let operation_id = request.operation_id;
                    let transition = $crate::__internal::core::api::fleet_activation::FleetActivationApi::activate_nonroot(request)?;
                    if transition.transitioned {
                        __canic_schedule_prepared_activation_init(transition.application_init_args);
                    }
                    Ok(StoreCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                StoreCommand::InspectTemplate(request) => {
                    ::canic::api::canister::template::WasmStoreCanisterApi::info(
                        request.template_id,
                        request.version,
                    )
                    .map(StoreCommandResponse::InspectTemplate)
                }
                StoreCommand::PrepareChunkSet(request) => {
                    ::canic::api::canister::template::WasmStoreCanisterApi::prepare(request)
                        .map(StoreCommandResponse::PrepareChunkSet)
                }
                StoreCommand::PrepareFleetCredential(request) => {
                    let operation_id = request.operation_id;
                    $crate::__internal::core::api::fleet_activation::FleetActivationApi::prepare_nonroot_credential_generation(request)?;
                    Ok(StoreCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                StoreCommand::ReclaimDeletionCycles(request) => {
                    ::canic::api::canister::template::WasmStoreCanisterApi::reclaim_deletion_cycles(request)
                        .await
                        .map(StoreCommandResponse::ReclaimDeletionCycles)
                }
                StoreCommand::RespondCapability(envelope) => {
                    $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_nonroot(
                        envelope,
                    )
                    .await
                    .map(StoreCommandResponse::RespondCapability)
                }
                StoreCommand::RunGc(request) => {
                    let operation_id = request.operation_id;
                    let should_advance = ::canic::api::canister::template::WasmStoreCanisterApi::status()?
                        .gc
                        .mode
                        != ::canic::ids::WasmStoreGcMode::Normal;
                    ::canic::api::canister::template::WasmStoreCanisterApi::prepare_gc(operation_id)?;
                    if should_advance {
                        $crate::__internal::core::api::timer::TimerApi::defer_lifecycle_required(
                            ::core::time::Duration::ZERO,
                            "canic:wasm_store:gc",
                            async move {
                                let _ = ::canic::api::canister::template::WasmStoreCanisterApi::run_gc(operation_id).await;
                            },
                        );
                    }
                    Ok(StoreCommandResponse::OperationAccepted(
                        ::canic::dto::role::OperationReceipt { operation_id },
                    ))
                }
                StoreCommand::StageManifest(request) => {
                    ::canic::api::canister::template::WasmStoreCanisterApi::stage_manifest(request)?;
                    Ok(StoreCommandResponse::StageManifest)
                }
                StoreCommand::SynchronizeState(snapshot) => {
                    $crate::__internal::core::api::cascade::CascadeApi::sync_state(snapshot)
                        .await?;
                    Ok(StoreCommandResponse::SynchronizeState)
                }
                StoreCommand::SynchronizeTopology(snapshot) => {
                    $crate::__internal::core::api::cascade::CascadeApi::sync_topology(
                        snapshot,
                    )
                    .await?;
                    Ok(StoreCommandResponse::SynchronizeTopology)
                }
            }
        }

        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum PublicStatusRequest {
            Health,
            Metrics(::canic::dto::public_status::PublicMetricsRequest),
            History(::canic::dto::public_status::PublicHistoryRequest),
            Overview,
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum PublicStatusResponse {
            Health(::canic::dto::public_status::PublicHealth),
            Metrics(::canic::dto::public_status::PublicMetricsSnapshot),
            History(::canic::dto::public_status::PublicHistorySnapshot),
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
                PublicStatusRequest::History(request) => Ok(PublicStatusResponse::History(::canic::__internal::core::api::public_status::PublicStatusApi::history(request))),
                PublicStatusRequest::Metrics(request) => Ok(PublicStatusResponse::Metrics(
                    ::canic::__internal::core::api::public_status::PublicStatusApi::metrics(request),
                )),
                PublicStatusRequest::Overview => {
                    let capabilities =
                        $crate::__internal::core::role_contract::built_in_role_capabilities(
                            $crate::__internal::core::role_contract::BuiltInRoleKind::WasmStore,
                        );
                    Ok(PublicStatusResponse::Overview(
                        $crate::__internal::core::api::role::RoleOverviewApi::overview(
                            $crate::__internal::core::ids::CanisterRole::from("wasm_store"),
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
            CycleBalance,
            CycleHistory(::canic::dto::page::PageRequest),
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum ObservabilityResponse {
            CycleBalance(::canic::dto::role::CycleBalanceStatusResponse),
            CycleHistory(::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>),
        }
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_observability(
            request: ObservabilityRequest,
        ) -> Result<ObservabilityResponse, ::canic::Error> {
            match request {
                ObservabilityRequest::CycleBalance => Ok(ObservabilityResponse::CycleBalance(
                    ::canic::dto::role::CycleBalanceStatusResponse {
                        cycles: $crate::__internal::cdk::api::canister_cycle_balance(),
                    },
                )),
                ObservabilityRequest::CycleHistory(page) => Ok(ObservabilityResponse::CycleHistory(
                    $crate::__internal::core::api::cycles::CycleTrackerQuery::page(page),
                )),
            }
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum StoreStatusRequest {
            Authority,
            Operation(::canic::dto::role::OperationStatusRequest),
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum StoreStatusResponse {
            Authority(::canic::ids::FleetSubnetWasmStoreAuthority),
            Operation(::canic::dto::template::StoreOperationStatusResponse),
        }
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_wasm_store_status(
            request: StoreStatusRequest,
        ) -> Result<StoreStatusResponse, ::canic::Error> {
            match request {
                StoreStatusRequest::Authority => {
                    $crate::__internal::core::api::fleet_activation::FleetActivationApi::wasm_store_authority()
                        .map(StoreStatusResponse::Authority)
                }
                StoreStatusRequest::Operation(request) => {
                    ::canic::api::canister::template::WasmStoreCanisterApi::operation_status(
                        request.operation_id,
                    )
                    .map(StoreStatusResponse::Operation)
                }
            }
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum StoreCatalogReadRequest {
            Catalog,
            Storage,
            Template(::canic::dto::template::TemplateLookupRequest),
        }
        #[derive(::canic::__internal::candid::CandidType, ::canic::__internal::serde::Deserialize)]
        #[serde(crate = "::canic::__internal::serde")]
        pub enum StoreCatalogReadResponse {
            Catalog(Vec<::canic::dto::template::WasmStoreCatalogEntryResponse>),
            Storage(::canic::dto::template::WasmStoreStatusResponse),
            Template(::canic::dto::template::TemplateStagingStatusResponse),
        }
        #[$crate::canic_query(requires(custom(
            ::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate
        )))]
        async fn canic_wasm_store_catalog(
            request: StoreCatalogReadRequest,
        ) -> Result<StoreCatalogReadResponse, ::canic::Error> {
            match request {
                StoreCatalogReadRequest::Catalog => {
                    ::canic::api::canister::template::WasmStoreCanisterApi::catalog()
                        .map(StoreCatalogReadResponse::Catalog)
                }
                StoreCatalogReadRequest::Storage => {
                    ::canic::api::canister::template::WasmStoreCanisterApi::status()
                        .map(StoreCatalogReadResponse::Storage)
                }
                StoreCatalogReadRequest::Template(request) => {
                    ::canic::api::canister::template::WasmStoreCanisterApi::staging_status(request)
                        .map(StoreCatalogReadResponse::Template)
                }
            }
        }

        #[$crate::canic_update(internal, requires(custom(::canic::__internal::control_plane::api::template::WasmStoreMutationCallerPredicate)), payload(max_bytes = ::canic::CANIC_WASM_CHUNK_BYTES + 64 * 1024))]
        async fn canic_wasm_store_publish_chunk(
            request: ::canic::dto::template::TemplateChunkInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::publish_chunk(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_chunk(
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
