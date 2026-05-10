// -----------------------------------------------------------------------------
// Root endpoint emitters
// -----------------------------------------------------------------------------

// Leaf emitter for the root-only control-plane, registry, and operator admin surface.
#[macro_export]
macro_rules! canic_emit_root_admin_endpoints {
    () => {
        #[$crate::canic_update(internal, requires(caller::is_controller()))]
        async fn canic_app(cmd: ::canic::dto::state::AppCommand) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::state::AppStateApi::execute_command(cmd).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_canister_upgrade(
            canister_pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::dto::rpc::UpgradeCanisterResponse, ::canic::Error> {
            let res =
                $crate::__internal::core::api::rpc::RpcApi::upgrade_canister_request(canister_pid)
                    .await?;

            Ok(res)
        }

        #[$crate::canic_update(requires(caller::is_root(), caller::is_controller()))]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::dto::canister::CanisterStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_status(pid).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::__internal::core::api::config::ConfigApi::export_toml()
        }

        #[$crate::canic_query]
        fn canic_app_registry()
        -> Result<::canic::dto::topology::AppRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::registry::AppRegistryApi::registry())
        }

        #[$crate::canic_query]
        fn canic_subnet_registry()
        -> Result<::canic::dto::topology::SubnetRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::registry::SubnetRegistryApi::registry())
        }

        #[$crate::canic_query]
        async fn canic_pool_list()
        -> Result<::canic::dto::pool::CanisterPoolResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::pool::CanisterPoolApi::list())
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_pool_admin(
            cmd: ::canic::dto::pool::PoolAdminCommand,
        ) -> Result<::canic::dto::pool::PoolAdminResponse, ::canic::Error> {
            $crate::__internal::core::api::pool::CanisterPoolApi::admin(cmd).await
        }
    };
}

// Leaf emitter for the root-only auth, delegation, and attestation authority surface.
#[macro_export]
macro_rules! canic_emit_root_auth_attestation_endpoints {
    () => {
        #[$crate::canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_request_delegation(
            request: ::canic::dto::auth::DelegationProofIssueRequest,
        ) -> Result<::canic::dto::auth::DelegationProof, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::issue_delegation_proof(request).await
        }

        #[$crate::canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_request_role_attestation(
            request: ::canic::dto::auth::RoleAttestationRequest,
        ) -> Result<::canic::dto::auth::SignedRoleAttestation, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::request_role_attestation_root(request)
                .await
        }

        #[$crate::canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_attestation_key_set()
        -> Result<::canic::dto::auth::AttestationKeySet, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::attestation_key_set().await
        }
    };
}

// Leaf emitter for the root-only WasmStore bootstrap/publication control surface.
#[macro_export]
macro_rules! canic_emit_root_wasm_store_endpoints {
    () => {
        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_stage_manifest_admin(
            request: ::canic::dto::template::TemplateManifestInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::stage_root_wasm_store_manifest(
                request,
            )
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_prepare_admin(
            request: ::canic::dto::template::TemplateChunkSetPrepareInput,
        ) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::prepare_root_wasm_store_chunk_set(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()), payload(max_bytes = ::canic::CANIC_WASM_CHUNK_BYTES + 64 * 1024))]
        async fn canic_wasm_store_bootstrap_publish_chunk_admin(
            request: ::canic::dto::template::TemplateChunkInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::publish_root_wasm_store_chunk(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_resume_root_admin() -> Result<(), ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_init_root_bootstrap();
            Ok(())
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_debug(
        ) -> Result<::canic::dto::template::WasmStoreBootstrapDebugResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::debug_bootstrap()
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_template_stage_manifest_admin(
            request: ::canic::dto::template::TemplateManifestInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::stage_manifest(request);
            Ok(())
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_template_prepare_admin(
            request: ::canic::dto::template::TemplateChunkSetPrepareInput,
        ) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::prepare_chunk_set(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()), payload(max_bytes = ::canic::CANIC_WASM_CHUNK_BYTES + 64 * 1024))]
        async fn canic_template_publish_chunk_admin(
            request: ::canic::dto::template::TemplateChunkInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::publish_chunk(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_template_publish_to_current_store_admin() -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreBootstrapApi::publish_staged_release_set_to_current_store().await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_admin(
            cmd: ::canic::dto::template::WasmStoreAdminCommand,
        ) -> Result<::canic::dto::template::WasmStoreAdminResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStorePublicationApi::admin(cmd).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_wasm_store_overview(
        ) -> Result<::canic::dto::template::WasmStoreOverviewResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStorePublicationApi::overview()
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_publication_status(
        ) -> Result<::canic::dto::template::WasmStorePublicationStatusResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStorePublicationApi::status().await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_retired_status(
        ) -> Result<Option<::canic::dto::template::WasmStoreRetiredStoreStatusResponse>, ::canic::Error> {
            ::canic::api::canister::template::WasmStorePublicationApi::retired_store_status().await
        }

    };
}
