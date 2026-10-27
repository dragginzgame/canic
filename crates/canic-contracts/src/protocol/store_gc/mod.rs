//! Exact Root-owned GC intent; transport does not expand deletion authority.
pub const METHOD: &str = "canic_wasm_store_run_gc";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_1404;
pub use crate::protocol::store_publication::{REQUEST_LIMITS, RESPONSE_LIMITS};
