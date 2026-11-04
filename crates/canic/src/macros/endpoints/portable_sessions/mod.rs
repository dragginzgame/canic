//! Module: macros::endpoints::portable_sessions
//!
//! Responsibility: export Auth's bounded contracts for receiver-capable roles.
//! Boundary: Core acquires the actual caller and composes protected authority.

#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_portable_session_endpoints {
    () => {
        #[cfg(canic_capability_local_application_authorization)]
        #[$crate::update(internal, requires(caller::is_root()), on_access_denied = "reject", wire(
            id = ::canic::protocol::portable_inspection::policy::METHOD_ID,
            request = ::canic::protocol::portable_inspection::policy::REQUEST_LIMITS,
            response = ::canic::protocol::portable_inspection::policy::RESPONSE_LIMITS
        ))]
        fn canic_portable_policy_inspect() -> Result<::canic::dto::portable_inspection::PolicyReport, ::canic::dto::portable_inspection::Refusal> {
            ::canic::__internal::core::api::auth::portable::inspect_policy()
        }
        #[cfg(canic_capability_local_application_authorization)]
        #[$crate::update(internal, requires(caller::is_root()), on_access_denied = "reject", wire(
            id = ::canic::protocol::portable_inspection::METHOD_ID,
            request = ::canic::protocol::portable_inspection::REQUEST_LIMITS,
            response = ::canic::protocol::portable_inspection::RESPONSE_LIMITS
        ))]
        fn canic_portable_sessions_inspect(request: ::canic::dto::portable_inspection::Request)
            -> Result<::canic::dto::portable_inspection::Page, ::canic::dto::portable_inspection::Refusal> {
            ::canic::__internal::core::api::auth::portable::inspect(request)
        }

        #[cfg(canic_capability_local_application_authorization)]
        #[$crate::update(internal, public, on_access_denied = "reject", wire(
            id = ::canic::__internal::core::api::auth::portable::EstablishSession::ID,
            request = ::canic::__internal::core::api::auth::portable::EstablishSession::REQUEST_LIMITS,
            response = ::canic::__internal::core::api::auth::portable::EstablishSession::RESPONSE_LIMITS
        ))]
        fn canic_establish_portable_session(input: ::canic::__internal::core::api::auth::portable::SessionInput)
            -> Result<::canic::__internal::core::api::auth::portable::SessionView, ::canic::__internal::core::api::auth::portable::AdmissionRefusal> {
            ::canic::__internal::core::api::auth::portable::establish(input)
        }

        #[cfg(canic_capability_local_application_authorization)]
        #[$crate::query(internal, public, on_access_denied = "reject", wire(
            id = ::canic::__internal::core::api::auth::portable::ReadSession::ID,
            request = ::canic::__internal::core::api::auth::portable::ReadSession::REQUEST_LIMITS,
            response = ::canic::__internal::core::api::auth::portable::ReadSession::RESPONSE_LIMITS
        ))]
        fn canic_portable_session_read()
            -> Result<::canic::__internal::core::api::auth::portable::SessionView, ::canic::__internal::core::api::auth::portable::SessionReadRefusal> {
            ::canic::__internal::core::api::auth::portable::read()
        }

        #[cfg(canic_capability_local_application_authorization)]
        #[$crate::update(internal, public, on_access_denied = "reject", wire(
            id = ::canic::__internal::core::api::auth::portable::Logout::ID,
            request = ::canic::__internal::core::api::auth::portable::Logout::REQUEST_LIMITS,
            response = ::canic::__internal::core::api::auth::portable::Logout::RESPONSE_LIMITS
        ))]
        fn canic_logout_portable_session() -> ::canic::__internal::core::api::auth::portable::LogoutResult {
            ::canic::__internal::core::api::auth::portable::logout()
        }
    };
}
