//! Module: macros::endpoints::blob_storage
//!
//! Responsibility: emit the opt-in immutable object storage gateway endpoints.
//! Does not own: blob lifecycle storage, gateway authorization, or protocol DTOs.
//! Boundary: exposes a guarded facade macro that delegates to the core blob-storage API.

/// Emit the Caffeine immutable object-storage gateway endpoint surface.
///
/// The host canister must supply an access expression for certificate creation.
/// Gateway liveness/deletion endpoints use the protocol's own gateway-principal
/// checks and do not run through the product frontend auth path.
#[macro_export]
macro_rules! canic_emit_blob_storage_endpoints {
    (guard = $guard:expr $(,)?) => {
        #[cfg(not(feature = "blob-storage"))]
        compile_error!(
            "canic_emit_blob_storage_endpoints! requires the canic facade feature \"blob-storage\""
        );

        #[cfg(feature = "blob-storage")]
        #[$crate::canic_query(internal, public, name = "_immutableObjectStorageBlobsAreLive")]
        fn canic_blob_storage_blobs_are_live(hash_bytes_list: Vec<Vec<u8>>) -> Vec<bool> {
            $crate::__internal::core::api::blob_storage::BlobStorageApi::blobs_are_live(
                hash_bytes_list,
            )
        }

        #[cfg(feature = "blob-storage")]
        #[$crate::canic_query(internal, public, name = "_immutableObjectStorageBlobsToDelete")]
        fn canic_blob_storage_blobs_to_delete() -> Vec<String> {
            let caller = $crate::cdk::api::msg_caller();
            $crate::__internal::core::api::blob_storage::BlobStorageApi::pending_deletion_hashes_for_gateway(
                caller,
            )
        }

        #[cfg(feature = "blob-storage")]
        #[$crate::canic_update(internal, public, name = "_immutableObjectStorageConfirmBlobDeletion")]
        fn canic_blob_storage_confirm_blob_deletion(hash_bytes_list: Vec<Vec<u8>>) {
            let caller = $crate::cdk::api::msg_caller();
            $crate::__internal::core::api::blob_storage::BlobStorageApi::confirm_deleted_by_gateway_hash_bytes_batch(
                caller,
                hash_bytes_list,
            );
        }

        #[cfg(feature = "blob-storage")]
        #[$crate::canic_update(requires($guard), name = "_immutableObjectStorageCreateCertificate")]
        async fn canic_blob_storage_create_certificate(
            root_hash: String,
        ) -> Result<::canic::dto::blob_storage::CreateCertificateResult, ::canic::Error> {
            $crate::__internal::core::api::blob_storage::BlobStorageApi::create_certificate(
                root_hash,
            )
        }
    };
    () => {
        compile_error!("canic_emit_blob_storage_endpoints! requires guard = <access expression>");
    };
    ($($tt:tt)+) => {
        compile_error!("canic_emit_blob_storage_endpoints! syntax is guard = <access expression>");
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn blob_storage_endpoint_macro_requires_guard_branch() {
        let source = include_str!("blob_storage.rs");

        assert!(
            source.contains("compile_error!(\"canic_emit_blob_storage_endpoints! requires guard = <access expression>\")"),
            "missing-guard macro branch should stay a compile-time error"
        );
    }
}
