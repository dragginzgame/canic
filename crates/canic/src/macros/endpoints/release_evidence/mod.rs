//! Shared Root/Coordinator release discovery; never settles or prunes authority.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_release_evidence_endpoints {
    (before = [$($before:tt)*] $(, root_gate = $root_gate:path)?) => {
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::release_evidence::intents::METHOD_ID, request = ::canic::protocol::release_evidence::REQUEST_LIMITS, response = ::canic::protocol::release_evidence::RESPONSE_LIMITS))]
        fn canic_release_intents(start_after: Option<::canic::dto::release_intents::IntentReleaseKey>) -> Result<::canic::dto::release_intents::IntentReleaseResponse, ::canic::Error> {
            $($root_gate(::canic::protocol::release_evidence::intents::METHOD, true)?;)?
            $($before)*
            $crate::__internal::core::api::observability::ObservabilityApi::release_intents(start_after)
        }
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::release_evidence::receipts::METHOD_ID, request = ::canic::protocol::release_evidence::REQUEST_LIMITS, response = ::canic::protocol::release_evidence::RESPONSE_LIMITS))]
        fn canic_release_receipts(start_after: Option<[u8; 32]>) -> Result<::canic::dto::release_receipts::ReplayReleaseResponse, ::canic::Error> {
            $($root_gate(::canic::protocol::release_evidence::receipts::METHOD, true)?;)?
            $($before)*
            $crate::__internal::core::api::observability::ObservabilityApi::release_receipts(start_after)
        }
    };
}
