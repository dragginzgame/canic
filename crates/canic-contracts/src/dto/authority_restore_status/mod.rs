//! Native read-only authority-fence projection; stored and mutation contracts stay separate.
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Current fence phase with a native recipient identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum Phase {
    Open,
    Sealed,
    ReleaseSealed {
        review_sha256: [u8; 32],
        recipient: Principal,
    },
}

/// Exact retained fence evidence; a read never resumes or changes the authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Status {
    pub authority_canister: Principal,
    pub phase: Phase,
    pub operation_id: Option<[u8; 32]>,
    pub history_total_num_changes: Option<u64>,
    pub changed_at_ns: Option<u64>,
}
