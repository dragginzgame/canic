#![cfg(feature = "blob-storage")]

mod fixture {
    canic::canic_emit_blob_storage_endpoints!(guard = caller::is_controller());

    #[test]
    fn blob_storage_endpoint_macro_accepts_host_guard() {
        let _ = canic_blob_storage_blobs_are_live;
        let _ = canic_blob_storage_blobs_to_delete;
        let _ = canic_blob_storage_confirm_blob_deletion;
        let _ = canic_blob_storage_create_certificate;
    }
}
