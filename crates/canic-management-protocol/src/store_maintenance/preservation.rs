//! Current Store entry-scan evidence.
use ic_wasm_store_protocol::{Release, preservation::Catalog};
use serde::{Deserialize, Serialize};

/// Bounded catalog commitment, without retaining Store's catalog in Root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Preservation {
    /// Exact counters and quota observed under closed admission.
    pub catalog: Catalog,
    /// Last committed release in the selected scan order.
    pub after: Option<Release>,
    /// Number of records included, including incomplete uploads and removed labels.
    pub observed: u64,
    /// Ordered metadata commitment, excluding payload bytes and management cache.
    pub digest: [u8; 32],
    /// Explicit end page and matching total count have been retained.
    pub complete: bool,
    /// Last entry was removed; false before the first page.
    pub after_removed: bool,
}
