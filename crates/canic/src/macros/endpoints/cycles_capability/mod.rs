//! Shared non-root cycles endpoint; the existing RPC workflow verifies structural authority and replay.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_cycles_capability_endpoint {
    () => {
        #[$crate::update(internal, public, wire(id = ::canic::protocol::cycles_capability::METHOD_ID, request = ::canic::protocol::cycles_capability::REQUEST_LIMITS, response = ::canic::protocol::cycles_capability::RESPONSE_LIMITS))]
        async fn canic_request_cycles(envelope: ::canic::dto::capability::NonrootCyclesCapabilityEnvelopeV1) -> Result<::canic::dto::capability::NonrootCyclesCapabilityResponseV1, ::canic::Error> {
            $crate::__internal::core::access::expr::eval_default_fleet_guard(
                $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates,
                ::canic::ids::EndpointCall {
                    endpoint: ::canic::ids::EndpointId::new(::canic::protocol::cycles_capability::METHOD),
                    kind: ::canic::ids::EndpointCallKind::Update,
                },
            ).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::rpc::RpcApi::response_capability_v1_nonroot(envelope).await
        }
    };
}
