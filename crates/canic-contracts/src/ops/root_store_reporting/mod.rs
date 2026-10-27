//! Lossless Root inventory conversion; no publication or storage policy is owned here.
use crate::dto::{
    root_store_reporting::{Overview, Store},
    template::{WasmStoreOverviewResponse, WasmStoreOverviewStoreResponse},
};
impl From<WasmStoreOverviewStoreResponse> for Store {
    fn from(value: WasmStoreOverviewStoreResponse) -> Self {
        Self {
            binding: value.binding,
            pid: value.pid.into(),
            created_at: value.created_at,
            publication_slot: value.publication_slot,
            gc: value.gc,
            approved_payload_bytes: value.approved_payload_bytes,
            approved_payload_size: value.approved_payload_size,
            max_store_bytes: value.max_store_bytes,
            max_store_size: value.max_store_size,
            remaining_approved_payload_bytes: value.remaining_approved_payload_bytes,
            remaining_approved_payload_size: value.remaining_approved_payload_size,
            headroom_bytes: value.headroom_bytes,
            headroom_size: value.headroom_size,
            within_approved_headroom: value.within_approved_headroom,
            approved_template_count: value.approved_template_count,
            max_templates: value.max_templates,
            approved_release_count: value.approved_release_count,
            max_template_versions_per_template: value.max_template_versions_per_template,
            approved_templates: value.approved_templates,
        }
    }
}
impl From<Store> for WasmStoreOverviewStoreResponse {
    fn from(value: Store) -> Self {
        Self {
            binding: value.binding,
            pid: value.pid.into(),
            created_at: value.created_at,
            publication_slot: value.publication_slot,
            gc: value.gc,
            approved_payload_bytes: value.approved_payload_bytes,
            approved_payload_size: value.approved_payload_size,
            max_store_bytes: value.max_store_bytes,
            max_store_size: value.max_store_size,
            remaining_approved_payload_bytes: value.remaining_approved_payload_bytes,
            remaining_approved_payload_size: value.remaining_approved_payload_size,
            headroom_bytes: value.headroom_bytes,
            headroom_size: value.headroom_size,
            within_approved_headroom: value.within_approved_headroom,
            approved_template_count: value.approved_template_count,
            max_templates: value.max_templates,
            approved_release_count: value.approved_release_count,
            max_template_versions_per_template: value.max_template_versions_per_template,
            approved_templates: value.approved_templates,
        }
    }
}
impl From<WasmStoreOverviewResponse> for Overview {
    fn from(value: WasmStoreOverviewResponse) -> Self {
        Self {
            publication: value.publication,
            stores: value.stores.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<Overview> for WasmStoreOverviewResponse {
    fn from(value: Overview) -> Self {
        Self {
            publication: value.publication,
            stores: value.stores.into_iter().map(Into::into).collect(),
        }
    }
}
