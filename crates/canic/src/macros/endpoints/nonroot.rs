//! Module: macros::endpoints::nonroot
//!
//! Responsibility: emit non-root endpoint macros for propagation and issuer support.
//! Does not own: cascade state application, delegated-token issuance, or proof storage.
//! Boundary: exposes facade macros that delegate immediately to core APIs.

/// Emit the managed Component-tree Directory and runtime activation endpoints.
#[macro_export]
macro_rules! canic_emit_component_runtime_endpoints {
    () => {
        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_component_runtime_directory_prepare(
            request: ::canic::dto::component_registry::ComponentRuntimeDirectoryPreparationRequest,
        ) -> Result<::canic::dto::component_registry::ComponentRuntimeStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::prepare_directory(request)
        }

        #[$crate::canic_query(internal, requires(caller::is_root()))]
        async fn canic_component_runtime_status(
        ) -> Result<::canic::dto::component_registry::ComponentRuntimeStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::status()
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_component_runtime_directory_synchronize(
            request: ::canic::dto::component_registry::ComponentRuntimeDirectorySynchronizationRequest,
        ) -> Result<::canic::dto::component_registry::ComponentRuntimeStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::synchronize_directory(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_component_runtime_activate(
            request: ::canic::dto::component_registry::ComponentRuntimeActivationRequest,
        ) -> Result<::canic::dto::component_registry::ComponentRuntimeStatusResponse, ::canic::Error> {
            let transition =
                $crate::__internal::core::api::component_runtime::ComponentRuntimeApi::activate(request)?;
            __canic_schedule_prepared_activation_init(transition.application_init_args);
            Ok(transition.status)
        }
    };
}

/// Emit the managed non-root endpoints used during Fleet activation.
#[macro_export]
macro_rules! canic_emit_nonroot_fleet_activation_endpoints {
    () => {
        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_prepare_fleet_credential_generation(
            request: ::canic::dto::fleet_activation::FleetCredentialGenerationRequest,
        ) -> Result<::canic::dto::fleet_activation::FleetActivationStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::fleet_activation::FleetActivationApi::prepare_nonroot_credential_generation(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_activate_fleet(
            request: ::canic::dto::fleet_activation::FleetActivationRequest,
        ) -> Result<::canic::dto::fleet_activation::FleetActivationStatusResponse, ::canic::Error> {
            let transition =
                $crate::__internal::core::api::fleet_activation::FleetActivationApi::activate_nonroot(request)?;
            __canic_schedule_prepared_activation_init(transition.application_init_args);
            Ok(transition.status)
        }
    };
}

/// Emit the non-root sync endpoints used for state and topology propagation.
#[macro_export]
macro_rules! canic_emit_nonroot_sync_topology_endpoints {
    () => {
        #[$crate::canic_update(
            internal,
            requires(caller::is_parent()),
            payload(max_bytes = ::canic::__internal::core::protocol::CASCADE_SNAPSHOT_MAX_BYTES)
        )]
        async fn canic_sync_state(
            snapshot: ::canic::dto::cascade::StateSnapshotInput,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::cascade::CascadeApi::sync_state(snapshot).await
        }

        #[$crate::canic_update(
            internal,
            requires(caller::is_parent()),
            payload(max_bytes = ::canic::__internal::core::protocol::CASCADE_SNAPSHOT_MAX_BYTES)
        )]
        async fn canic_sync_topology(
            snapshot: ::canic::dto::cascade::TopologySnapshotInput,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::cascade::CascadeApi::sync_topology(snapshot).await
        }
    };
}

/// Emit the non-root delegated-token issuer provisioning endpoints.
#[macro_export]
macro_rules! canic_emit_nonroot_auth_attestation_endpoints {
    () => {
        #[$crate::canic_update(public)]
        async fn canic_prepare_delegated_token(
            request: ::canic::dto::auth::DelegatedTokenPrepareRequest,
        ) -> Result<::canic::dto::auth::DelegatedTokenPrepareResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::prepare_delegated_token(request).await
        }

        #[$crate::canic_query(public)]
        async fn canic_get_delegated_token(
            request: ::canic::dto::auth::DelegatedTokenGetRequest,
        ) -> Result<::canic::dto::auth::DelegatedToken, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::get_delegated_token(request)
        }

        #[$crate::canic_update(requires(caller::is_controller()))]
        async fn canic_install_active_delegation_proof(
            request: ::canic::dto::auth::InstallActiveDelegationProofRequest,
        ) -> Result<::canic::dto::auth::InstallActiveDelegationProofResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::install_active_delegation_proof(request)
        }

        #[$crate::canic_query(public)]
        async fn canic_active_delegation_proof_status()
        -> Result<::canic::dto::auth::ActiveDelegationProofStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::active_delegation_proof_status()
        }
    };
}
