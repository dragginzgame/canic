//! Controller-authenticated state reads shared by the authority roles.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_authority_restore_status_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_state::authority_restore::METHOD_ID, request = ::canic::protocol::protected_state::authority_restore::REQUEST_LIMITS, response = ::canic::protocol::protected_state::authority_restore::RESPONSE_LIMITS))]
        fn canic_authority_restore_status() -> Result<::canic::dto::authority_restore_status::Status, ::canic::Error> {
            $($before)*
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::status().map(Into::into)
        }
    };
}

/// Emit both exact snapshot mutations with role-owned lifecycle prerequisites.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_authority_snapshot_endpoints {
    (before = [$($before:stmt)*], prepare = $prepare:path, resume = $resume:path) => {
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_state::snapshot_prepare::METHOD_ID, request = ::canic::protocol::protected_state::snapshot_prepare::REQUEST_LIMITS, response = ::canic::protocol::protected_state::snapshot_prepare::RESPONSE_LIMITS))]
        async fn canic_authority_snapshot_prepare(request: ::canic::dto::authority_restore::AuthoritySnapshotRequest) -> Result<::canic::dto::authority_restore_status::Status, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(true)?;
            $($before)*
            $prepare(request).await.map(Into::into)
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_state::snapshot_resume::METHOD_ID, request = ::canic::protocol::protected_state::snapshot_resume::REQUEST_LIMITS, response = ::canic::protocol::protected_state::snapshot_resume::RESPONSE_LIMITS))]
        async fn canic_authority_snapshot_resume(request: ::canic::dto::authority_restore::AuthoritySnapshotRequest) -> Result<::canic::dto::authority_restore_status::Status, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(true)?;
            $($before)*
            $resume(request).await.map(Into::into)
        }
    };
}
