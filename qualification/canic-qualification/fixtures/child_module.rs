//! Module and approval used to publish Store-backed test inputs.
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) struct Stage<A = canic_management_protocol::Approval> {
    pub(crate) approval: A,
    #[serde(with = "serde_bytes")]
    pub(crate) wasm: Vec<u8>,
}
