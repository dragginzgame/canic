//! Module: macros::endpoints::blob_storage_billing
//!
//! Responsibility: emit opt-in blob-storage billing gateway endpoints.
//! Does not own: Cashier calls, billing configuration, or funding policy.
//! Boundary: generated endpoints delegate immediately to the core blob-storage API.

/// Emit the Caffeine immutable object-storage billing endpoint surface.
///
/// The host canister must configure blob-storage billing before these endpoints
/// can succeed. Guards are intentionally separate so products can use different
/// authorization for Cashier gateway sync and project-cycle funding.
#[macro_export]
macro_rules! canic_emit_blob_storage_billing_endpoints {
    (
        sync_gateway_principals_guard = $sync_guard:expr,
        fund_from_cycles_guard = $fund_guard:expr,
        status_guard = $status_guard:expr $(,)?
    ) => {
        #[cfg(not(feature = "blob-storage-billing"))]
        compile_error!(
            "canic_emit_blob_storage_billing_endpoints! requires the canic facade feature \"blob-storage-billing\""
        );

        #[cfg(feature = "blob-storage-billing")]
        #[$crate::canic_update(
            requires($sync_guard),
            name = "_immutableObjectStorageUpdateGatewayPrincipals"
        )]
        async fn canic_blob_storage_update_gateway_principals() -> Result<(), ::canic::Error> {
            $crate::__internal::core::api::blob_storage::BlobStorageApi::sync_gateway_principals_from_configured_cashier()
                .await
                .map(|_| ())
        }

        #[cfg(feature = "blob-storage-billing")]
        #[$crate::canic_update(
            requires($fund_guard),
            name = "_immutableObjectStorageFundFromProjectCycles"
        )]
        async fn canic_blob_storage_fund_from_project_cycles(
            requested_cycles: u128,
        ) -> Result<::canic::dto::blob_storage::BlobProjectCyclesTopUpReport, ::canic::Error> {
            $crate::__internal::core::api::blob_storage::BlobStorageApi::fund_from_project_cycles(
                requested_cycles,
            )
            .await
        }

        #[cfg(feature = "blob-storage-billing")]
        #[$crate::canic_update(requires($status_guard), name = "get_blob_storage_status")]
        async fn canic_blob_storage_status(
            request: ::canic::dto::blob_storage::BlobStorageStatusRequest,
        ) -> Result<::canic::dto::blob_storage::BlobStorageStatusResponse, ::canic::Error> {
            Ok($crate::__internal::core::api::blob_storage::BlobStorageApi::status(
                request,
            )
            .await)
        }
    };
    () => {
        compile_error!(
            "canic_emit_blob_storage_billing_endpoints! requires sync_gateway_principals_guard = <access expression>, fund_from_cycles_guard = <access expression>, status_guard = <access expression>"
        );
    };
    ($($tt:tt)+) => {
        compile_error!(
            "canic_emit_blob_storage_billing_endpoints! syntax is sync_gateway_principals_guard = <access expression>, fund_from_cycles_guard = <access expression>, status_guard = <access expression>"
        );
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn blob_storage_billing_endpoint_macro_requires_named_guards() {
        let source = include_str!("blob_storage_billing.rs");

        assert!(
            source.contains("sync_gateway_principals_guard = <access expression>")
                && source.contains("fund_from_cycles_guard = <access expression>")
                && source.contains("status_guard = <access expression>"),
            "billing endpoint macro should require explicit named guards"
        );
    }
}
