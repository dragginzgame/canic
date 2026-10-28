//! Shared state propagation endpoint; runtime workflows own application and downstream reconciliation.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_state_cascade_endpoint {
    () => {
        #[$crate::update(internal, public, wire(id = ::canic::protocol::cascade::state::METHOD_ID, request = ::canic::protocol::cascade::REQUEST_LIMITS, response = ::canic::protocol::cascade::RESPONSE_LIMITS))]
        async fn canic_synchronize_state(snapshot: ::canic::dto::cascade::StateSnapshotInput) -> Result<::canic::dto::fleet_state_control::CascadeReport, ::canic::Error> {
            // Parent state recovery remains available while ordinary updates are fenced.
            let caller = $crate::__internal::cdk::api::msg_caller().into();
            $crate::__internal::core::access::auth::is_parent(caller).map_err(::canic::Error::from)?;
            $crate::__internal::core::api::cascade::CascadeApi::sync_state(snapshot).await.map(Into::into)
        }
    };
}
