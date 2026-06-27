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
            canister_pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::dto::rpc::UpgradeCanisterResponse, ::canic::Error> {
            let res =
                $crate::__internal::core::api::rpc::RpcApi::upgrade_canister_request(canister_pid)
                    .await?;

            Ok(res)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_canister_status(
            pid: ::canic::cdk::candid::Principal,
        ) -> Result<::canic::dto::canister::CanisterStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::ic::mgmt::MgmtApi::canister_status(pid).await
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_config() -> Result<String, ::canic::Error> {
            $crate::__internal::core::api::config::ConfigApi::export_toml()
        }

        // TBD: future app-level topology contract; keep private to controllers until
        // host/backup semantics are designed.
        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_app_registry()
        -> Result<::canic::dto::topology::AppRegistryResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::topology::registry::AppRegistryApi::registry())
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

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_upsert_delegation_renewal_provisioner(
            request: ::canic::dto::auth::RootDelegationRenewalProvisionerUpsertRequest,
        ) -> Result<::canic::dto::auth::RootDelegationRenewalProvisionerResponse, ::canic::Error>
        {
            $crate::__internal::core::api::auth::AuthApi::upsert_delegation_renewal_provisioner_root(request)
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_delegation_renewal_provisioners(
        ) -> Result<::canic::dto::auth::RootDelegationRenewalProvisionerListResponse, ::canic::Error>
        {
            $crate::__internal::core::api::auth::AuthApi::delegation_renewal_provisioners_root()
        }

        #[$crate::canic_query(requires(any(
            caller::is_controller(),
            caller::is_delegation_renewal_provisioner()
        )))]
        async fn canic_delegation_renewal_work(
        ) -> Result<::canic::dto::auth::RootDelegationRenewalWorkListResponse, ::canic::Error>
        {
            $crate::__internal::core::api::auth::AuthApi::delegation_renewal_work_root()
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_prepare_delegation_proof_batch(
            request: ::canic::dto::auth::RootDelegationProofBatchPrepareRequest,
        ) -> Result<::canic::dto::auth::RootDelegationProofBatchPrepareResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::prepare_delegation_proof_batch_root(
                request,
            )
        }

        #[$crate::canic_query(requires(caller::is_controller()))]
        async fn canic_get_delegation_proof_batch(
            request: ::canic::dto::auth::RootDelegationProofBatchGetRequest,
        ) -> Result<::canic::dto::auth::RootDelegationProofBatchGetResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::get_delegation_proof_batch_root(request)
        }

        #[$crate::canic_query(requires(any(
            caller::is_controller(),
            caller::is_delegation_renewal_provisioner()
        )))]
        async fn canic_get_delegation_renewal_proof_batch(
            request: ::canic::dto::auth::RootDelegationRenewalProofBatchGetRequest,
        ) -> Result<::canic::dto::auth::RootDelegationProofBatchGetResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::get_delegation_renewal_proof_batch_root(
                request,
            )
        }

        #[$crate::canic_update(requires(any(
            caller::is_controller(),
            caller::is_delegation_renewal_provisioner()
        )))]
        async fn canic_install_delegation_proof_batch(
            request: ::canic::dto::auth::RootDelegationProofBatchInstallRequest,
        ) -> Result<::canic::dto::auth::RootDelegationProofBatchInstallResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::install_delegation_proof_batch_root(
                request,
            )
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
