// -----------------------------------------------------------------------------
// ICP refill endpoint emitters
// -----------------------------------------------------------------------------

/// Emit the opt-in canister-side ICP-to-cycles refill endpoint.
///
/// The host canister must supply an access expression. Omitting the guard is a
/// compile-time error.
///
/// ```compile_fail
/// canic::canic_emit_icp_refill_endpoints!();
/// ```
#[macro_export]
macro_rules! canic_emit_icp_refill_endpoints {
    (guard = $guard:expr $(,)?) => {
        #[cfg(not(feature = "icp-refill"))]
        compile_error!(
            "canic_emit_icp_refill_endpoints! requires the canic facade feature \"icp-refill\""
        );

        #[cfg(feature = "icp-refill")]
        #[$crate::canic_update(requires($guard))]
        async fn canic_icp_refill(
            request: ::canic::dto::icp_refill::IcpRefillRequest,
        ) -> Result<::canic::dto::icp_refill::IcpRefillEndpointResponse, ::canic::Error> {
            $crate::__internal::core::api::icp_refill::IcpRefillApi::refill(request).await
        }
    };
    () => {
        compile_error!("canic_emit_icp_refill_endpoints! requires guard = <access expression>");
    };
    ($($tt:tt)+) => {
        compile_error!("canic_emit_icp_refill_endpoints! syntax is guard = <access expression>");
    };
}
