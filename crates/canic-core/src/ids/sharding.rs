use crate::{cdk::candid::CandidType, memory::impl_storable_bounded};
use serde::{Deserialize, Serialize};

///
/// ShardLifecycleState
///
/// Canonical lifecycle states for HRW-managed shards.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ShardLifecycleState {
    Created,
    Provisioned,
    Active,
    Retiring,
    Revoked,
}

impl_storable_bounded!(ShardLifecycleState, 32, false);
