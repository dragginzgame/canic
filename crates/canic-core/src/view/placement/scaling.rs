use crate::{cdk::types::BoundedString64, ids::CanisterRole};

///
/// ScalingWorkerPlanEntry
///

#[derive(Clone, Debug)]
pub struct ScalingWorkerPlanEntry {
    pub pool: BoundedString64,
    pub canister_role: CanisterRole,
}
