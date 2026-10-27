//! Native import review context; retained reservation evidence belongs to pool_census.
use crate::dto::{fleet_snapshot::FleetSubnetRootBinding, pool_import::PoolImportIdentity};
/// Supplied capacity held out of allocation until its exact reviewed import completes.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Bootstrap {
    pub review_sha256: [u8; 32],
    pub operator: ic_platform::Principal,
    pub sources: Vec<ic_platform::Principal>,
}
/// Complete current import authority and next sequence, without issuing work.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Context {
    pub maximum_call_debit_cycles: u128,
    pub bootstrap: Option<Bootstrap>,
    pub binding: FleetSubnetRootBinding,
    pub root_authority_sha256: [u8; 32],
    pub next_sequence: u64,
    pub active_import: Option<PoolImportIdentity>,
}
