//! Root capability entrypoints retain Component Registry authorization and existing lifecycle fences.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_authorize_root_capability {
    ($method:expr) => {{
        use $crate::__internal::core::access::expr::AccessCheck as _;
        let call = ::canic::ids::EndpointCall { endpoint: ::canic::ids::EndpointId::new($method), kind: ::canic::ids::EndpointCallKind::Update };
        $crate::__internal::core::access::expr::eval_default_fleet_guard(
            $crate::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates, call).map_err(::canic::Error::from)?;
        let caller = $crate::__internal::cdk::api::msg_caller().into();
        let context = $crate::__internal::core::access::expr::AccessContext { caller, call };
        $crate::__internal::control_plane::api::component_rpc::RootCapabilityCallerPredicate.eval(&context).map_err(::canic::Error::from)?;
        $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
        $crate::__internal::core::control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow::require_root_update_allowed($method, true)?;
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_root_capability_endpoints {
    () => {
#[$crate::canic_update(internal, public, wire(id = ::canic::protocol::root_capability::acknowledge::METHOD_ID, request = ::canic::protocol::root_capability::REQUEST_LIMITS, response = ::canic::protocol::root_capability::RESPONSE_LIMITS))]
 async fn canic_root_capability_acknowledge(request: ::canic::dto::root_capability::CapabilityRequest<::canic::dto::rpc::AcknowledgePlacementReceiptRequest>) -> Result<(), ::canic::Error> {
    $crate::__canic_authorize_root_capability!(::canic::protocol::root_capability::acknowledge::METHOD);
    let envelope = $crate::__internal::contracts::ops::root_capability::into_root_request(request, |value| ::canic::dto::rpc::Request::AcknowledgePlacementReceipt(value));
    let response = $crate::__internal::control_plane::api::component_rpc::ComponentRpcApi::response_capability_v1_root(envelope).await?;
    $crate::__internal::contracts::ops::root_capability::acknowledge_response(response)
 }
#[$crate::canic_update(internal, public, wire(id = ::canic::protocol::root_capability::allocate::METHOD_ID, request = ::canic::protocol::root_capability::REQUEST_LIMITS, response = ::canic::protocol::root_capability::RESPONSE_LIMITS))]
 async fn canic_root_capability_allocate(request: ::canic::dto::root_capability::CapabilityRequest<::canic::dto::root_capability::CreateCanisterRequest>) -> Result<::canic::dto::root_capability::CreateCanisterResponse, ::canic::Error> {
    $crate::__canic_authorize_root_capability!(::canic::protocol::root_capability::allocate::METHOD);
    let envelope = $crate::__internal::contracts::ops::root_capability::into_root_request(request, |value| ::canic::dto::rpc::Request::AllocatePlacementChild(value.into()));
    let response = $crate::__internal::control_plane::api::component_rpc::ComponentRpcApi::response_capability_v1_root(envelope).await?;
    $crate::__internal::contracts::ops::root_capability::create_response(response)
 }
#[$crate::canic_update(internal, public, wire(id = ::canic::protocol::root_capability::create::METHOD_ID, request = ::canic::protocol::root_capability::REQUEST_LIMITS, response = ::canic::protocol::root_capability::RESPONSE_LIMITS))]
 async fn canic_root_capability_create(request: ::canic::dto::root_capability::CapabilityRequest<::canic::dto::root_capability::CreateCanisterRequest>) -> Result<::canic::dto::root_capability::CreateCanisterResponse, ::canic::Error> {
    $crate::__canic_authorize_root_capability!(::canic::protocol::root_capability::create::METHOD);
    let envelope = $crate::__internal::contracts::ops::root_capability::into_root_request(request, |value| ::canic::dto::rpc::Request::CreateCanister(value.into()));
    let response = $crate::__internal::control_plane::api::component_rpc::ComponentRpcApi::response_capability_v1_root(envelope).await?;
    $crate::__internal::contracts::ops::root_capability::create_response(response)
 }
#[$crate::canic_update(internal, public, wire(id = ::canic::protocol::root_capability::recycle::METHOD_ID, request = ::canic::protocol::root_capability::REQUEST_LIMITS, response = ::canic::protocol::root_capability::RESPONSE_LIMITS))]
 async fn canic_root_capability_recycle(request: ::canic::dto::root_capability::CapabilityRequest<::canic::dto::root_capability::RecycleCanisterRequest>) -> Result<(), ::canic::Error> {
    $crate::__canic_authorize_root_capability!(::canic::protocol::root_capability::recycle::METHOD);
    let envelope = $crate::__internal::contracts::ops::root_capability::into_root_request(request, |value| ::canic::dto::rpc::Request::RecycleCanister(value.into()));
    let response = $crate::__internal::control_plane::api::component_rpc::ComponentRpcApi::response_capability_v1_root(envelope).await?;
    $crate::__internal::contracts::ops::root_capability::recycle_response(response)
 }
#[$crate::canic_update(internal, public, wire(id = ::canic::protocol::root_capability::cycles::METHOD_ID, request = ::canic::protocol::root_capability::REQUEST_LIMITS, response = ::canic::protocol::root_capability::RESPONSE_LIMITS))]
 async fn canic_root_capability_cycles(request: ::canic::dto::root_capability::CapabilityRequest<::canic::dto::rpc::CyclesRequest>) -> Result<::canic::dto::rpc::CyclesResponse, ::canic::Error> {
    $crate::__canic_authorize_root_capability!(::canic::protocol::root_capability::cycles::METHOD);
    let envelope = $crate::__internal::contracts::ops::root_capability::into_root_request(request, |value| ::canic::dto::rpc::Request::Cycles(value));
    let response = $crate::__internal::control_plane::api::component_rpc::ComponentRpcApi::response_capability_v1_root(envelope).await?;
    $crate::__internal::contracts::ops::root_capability::cycles_response(response)
 }
    };
}
