//! Native transport for exact parent-local funding evidence.
use crate::cycles::Cycles;

/// Parent-local ledger and unresolved grants for one exact child.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ChildFundingUsage {
    pub parent: ic_platform::Principal,
    pub child: ic_platform::Principal,
    pub observed_at_ns: u64,
    pub accounted_cycles: Cycles,
    pub last_accounted_at_secs: u64,
    pub pending_operations: u32,
    pub reserved_cycles: Option<Cycles>,
}
