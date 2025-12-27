use crate::dto::prelude::*;

///
/// EndpointHealthView
/// Derived endpoint-level health view joined at read time.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EndpointHealthView {
    pub endpoint: String,
    pub attempted: u64,
    pub denied: u64,
    pub completed: u64,
    pub ok: u64,
    pub err: u64,
    pub avg_instr: u64,
    pub total_instr: u64,
}
