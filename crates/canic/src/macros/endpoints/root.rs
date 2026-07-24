//! Module: macros::endpoints::root
//!
//! Responsibility: emit root-canister endpoint macros for control and authority surfaces.
//! Does not own: root state, pool policy, auth proof issuance, or wasm-store workflows.
//! Boundary: exposes facade macros that delegate immediately to core/control-plane APIs.

/// Emit root-only control-plane, registry, and operator admin endpoints.
#[macro_export]
macro_rules! canic_emit_root_admin_endpoints {
    () => {
        #[$crate::canic_update(internal, requires(caller::is_controller()))]
        async fn canic_app(
            cmd: ::canic::dto::state::AppCommand,
        ) -> Result<::canic::dto::state::AppCommandResponse, ::canic::Error> {
            $crate::__internal::core::api::state::AppStateApi::execute_command(cmd).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_canister_upgrade(
            canister_pid: ::canic::__internal::cdk::Principal,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::rpc::RpcApi::upgrade_canister_request(canister_pid).await
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_canister_status(
            pid: ::canic::__internal::cdk::Principal,
        ) -> Result<::canic::dto::canister::CanisterStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_status(pid).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::__internal::core::api::config::ConfigApi::export_toml()
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_icp_refill(
            request: ::canic::dto::icp_refill::IcpRefillRequest,
        ) -> Result<::canic::dto::icp_refill::IcpRefillEndpointResponse, ::canic::Error> {
            $crate::__internal::core::api::icp_refill::IcpRefillApi::refill(request).await
        }

        #[$crate::canic_query(public)]
        fn canic_subnet_registry()
        -> Result<::canic::dto::topology::SubnetRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::registry::SubnetRegistryApi::registry())
        }

        #[$crate::canic_query(public)]
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

/// Emit root-only auth, delegation, and attestation authority endpoints.
#[macro_export]
macro_rules! canic_emit_root_auth_attestation_endpoints {
    () => {
        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_upsert_root_issuer_policy(
            request: ::canic::dto::auth::RootIssuerPolicyUpsertRequest,
        ) -> Result<::canic::dto::auth::RootIssuerPolicyResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::upsert_root_issuer_policy_root(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_upsert_root_issuer_renewal_template(
            request: ::canic::dto::auth::RootIssuerRenewalTemplateUpsertRequest,
        ) -> Result<::canic::dto::auth::RootIssuerRenewalTemplateResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::upsert_root_issuer_renewal_template_root(
                request,
            )
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_root_issuer_renewal_status(
            request: ::canic::dto::auth::RootIssuerRenewalStatusRequest,
        ) -> Result<::canic::dto::auth::RootIssuerRenewalStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::root_issuer_renewal_status_root(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_get_or_create_chain_key_delegation_proof(
        ) -> Result<::canic::dto::auth::RootDelegationProofBatchProof, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::get_or_create_chain_key_delegation_proof_root()
                .await
        }

        #[$crate::canic_update(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_prepare_role_attestation(
            request: ::canic::dto::auth::RoleAttestationRequest,
        ) -> Result<::canic::dto::auth::RoleAttestationPrepareResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::prepare_role_attestation_root(request)
        }

        #[$crate::canic_query(internal, requires(caller::is_registered_to_subnet()))]
        async fn canic_get_role_attestation(
            request: ::canic::dto::auth::RoleAttestationGetRequest,
        ) -> Result<::canic::dto::auth::SignedRoleAttestation, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::get_role_attestation_root(request)
        }
    };
}

/// Emit root-only wasm-store bootstrap and publication control endpoints.
#[macro_export]
macro_rules! canic_emit_root_wasm_store_endpoints {
    () => {
        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_wasm_store_bootstrap_resume_root_admin() -> Result<(), ::canic::Error> {
            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_init_root_bootstrap()
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

    };
}
