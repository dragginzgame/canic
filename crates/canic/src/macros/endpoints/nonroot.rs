// -----------------------------------------------------------------------------
// Non-root endpoint emitters
// -----------------------------------------------------------------------------

// Leaf emitter for the non-root sync surface used for state/topology propagation.
#[macro_export]
macro_rules! canic_emit_nonroot_sync_topology_endpoints {
    () => {
        #[$crate::canic_update(internal, requires(caller::is_parent()))]
        async fn canic_sync_state(
            snapshot: ::canic::dto::cascade::StateSnapshotInput,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::cascade::CascadeApi::sync_state(snapshot).await
        }

        #[$crate::canic_update(internal, requires(caller::is_parent()))]
        async fn canic_sync_topology(
            snapshot: ::canic::dto::cascade::TopologySnapshotInput,
        ) -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::cascade::CascadeApi::sync_topology(snapshot).await
        }
    };
}

// Leaf emitter for the non-root auth/attestation provisioning surface.
#[macro_export]
macro_rules! canic_emit_nonroot_auth_attestation_endpoints {
    () => {
        #[$crate::canic_update]
        async fn canic_prepare_delegated_token(
            request: ::canic::dto::auth::DelegatedTokenPrepareRequest,
        ) -> Result<::canic::dto::auth::DelegatedTokenPrepareResponse, ::canic::Error> {
            $crate::__internal::core::api::auth::AuthApi::prepare_delegated_token(request)
        }

        #[$crate::canic_query]
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
    };
}
