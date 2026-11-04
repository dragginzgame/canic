//! Root registration endpoints authenticate governance and the actual issuer caller.
//! Core owns retained intent, possession verification and lifecycle fences.

#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_root_portable_issuer_endpoints {
    () => {
        #[cfg(canic_capability_root_delegation)]
        #[$crate::update(requires(caller::is_controller()), on_access_denied = "reject", wire(
            id = ::canic::protocol::portable_enrollment::registration::prepare::METHOD_ID,
            request = ::canic::protocol::portable_enrollment::registration::REQUEST_LIMITS,
            response = ::canic::protocol::portable_enrollment::registration::RESPONSE_LIMITS
        ))]
        async fn canic_root_prepare_portable_issuer(request: ::canic::dto::portable_enrollment::PreparePortableIssuer)
            -> Result<::canic::dto::portable_enrollment::PortableIssuerChallenge, ::canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)
                .map_err(|_| ::canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal::Unavailable)?;
            $crate::__internal::core::api::auth::AuthApi::prepare_portable_issuer_root(request).await.map_err(Into::into)
        }

        #[cfg(canic_capability_root_delegation)]
        #[$crate::update(requires(custom($crate::__internal::core::access::auth::portable_registration::ApprovedPortableIssuerPredicate)), on_access_denied = "reject", wire(
            id = ::canic::protocol::portable_enrollment::registration::complete::METHOD_ID,
            request = ::canic::protocol::portable_enrollment::registration::REQUEST_LIMITS,
            response = ::canic::protocol::portable_enrollment::registration::RESPONSE_LIMITS
        ))]
        fn canic_root_register_portable_issuer(request: ::canic::dto::portable_enrollment::RegisterPortableIssuer)
            -> Result<::canic::dto::portable_enrollment::PortableIssuerEnrollment, ::canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)
                .map_err(|_| ::canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal::Unavailable)?;
            $crate::__internal::core::api::auth::AuthApi::register_portable_issuer_root(request).map_err(Into::into)
        }

        #[cfg(canic_capability_root_delegation)]
        #[$crate::update(requires(custom($crate::__internal::core::access::auth::portable_registration::ApprovedPortableIssuerPredicate)), on_access_denied = "reject", wire(
            id = ::canic::protocol::portable_enrollment::registration::lookup::METHOD_ID,
            request = ::canic::protocol::portable_enrollment::registration::REQUEST_LIMITS,
            response = ::canic::protocol::portable_enrollment::registration::RESPONSE_LIMITS
        ))]
        fn canic_root_portable_issuer_registration(request: ::canic::dto::portable_enrollment::PortableIssuerRegistrationLookup)
            -> Result<::canic::dto::portable_enrollment::PortableIssuerRegistrationStatus, ::canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal> {
            $crate::__internal::core::api::auth::AuthApi::portable_issuer_registration_root(request.operation_id).map_err(Into::into)
        }

        #[cfg(canic_capability_root_delegation)]
        #[$crate::update(requires(caller::is_controller()), on_access_denied = "reject", wire(
            id = ::canic::protocol::portable_enrollment::registration::retire::METHOD_ID,
            request = ::canic::protocol::portable_enrollment::registration::REQUEST_LIMITS,
            response = ::canic::protocol::portable_enrollment::registration::RESPONSE_LIMITS
        ))]
        fn canic_root_retire_portable_issuer(request: ::canic::dto::portable_enrollment::RetirePortableIssuer)
            -> Result<(), ::canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)
                .map_err(|_| ::canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal::Unavailable)?;
            $crate::__internal::core::api::auth::AuthApi::retire_portable_issuer_root(request.issuer).map_err(Into::into)
        }
    };
}
