// -----------------------------------------------------------------------------
// Local wasm-store endpoint emitters
// -----------------------------------------------------------------------------

// Leaf emitter for the canonical local wasm-store canister surface.
#[macro_export]
macro_rules! canic_emit_local_wasm_store_endpoints {
    () => {
        #[$crate::canic_query(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_catalog()
        -> Result<Vec<::canic::dto::template::WasmStoreCatalogEntryResponse>, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::catalog()
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_prepare(
            request: ::canic::dto::template::TemplateChunkSetPrepareInput,
        ) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::prepare(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_stage_manifest(
            request: ::canic::dto::template::TemplateManifestInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::stage_manifest(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_root()), payload(max_bytes = ::canic::CANIC_WASM_CHUNK_BYTES + 64 * 1024))]
        async fn canic_wasm_store_publish_chunk(
            request: ::canic::dto::template::TemplateChunkInput,
        ) -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::publish_chunk(request)
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_info(
            template_id: ::canic::ids::TemplateId,
            version: ::canic::ids::TemplateVersion,
        ) -> Result<::canic::dto::template::TemplateChunkSetInfoResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::info(template_id, version)
        }

        #[$crate::canic_query(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_status()
        -> Result<::canic::dto::template::WasmStoreStatusResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::status()
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_prepare_gc() -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::prepare_gc()
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_begin_gc() -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::begin_gc()
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_complete_gc() -> Result<(), ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::complete_gc().await
        }

        #[$crate::canic_update(internal, requires(caller::is_root()))]
        async fn canic_wasm_store_chunk(
            template_id: ::canic::ids::TemplateId,
            version: ::canic::ids::TemplateVersion,
            chunk_index: u32,
        ) -> Result<::canic::dto::template::TemplateChunkResponse, ::canic::Error> {
            ::canic::api::canister::template::WasmStoreCanisterApi::chunk(
                template_id,
                version,
                chunk_index,
            )
        }
    };
}
