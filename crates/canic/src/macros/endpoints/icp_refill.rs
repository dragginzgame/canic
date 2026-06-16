//! Module: macros::endpoints::icp_refill
//!
//! Responsibility: emit the opt-in ICP refill endpoint macro.
//! Does not own: refill replay, ledger calls, CMC notification, or cost guards.
//! Boundary: exposes a guarded facade macro that delegates to the core refill API.

/// Emit the opt-in canister-side ICP-to-cycles refill endpoint.
///
/// The host canister must supply an access expression. Omitting the guard is a
/// compile-time error.
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

#[cfg(test)]
mod tests {
    #[test]
    fn icp_refill_endpoint_macro_requires_guard_branch() {
        let source = include_str!("icp_refill.rs");

        assert!(
            source.contains("compile_error!(\"canic_emit_icp_refill_endpoints! requires guard = <access expression>\")"),
            "missing-guard macro branch should stay a compile-time error"
        );
    }
}
