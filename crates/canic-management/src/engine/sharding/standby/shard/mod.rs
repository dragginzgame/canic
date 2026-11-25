//! Root custody for a drained shard donated to inventory.
use canic::sharding::{Allocation, ShardTarget};
use canic_management_protocol::stored::{ChildRetirement, RetirementPhase, Stage};

#[derive(serde::Serialize, serde::Deserialize)]
pub(in crate::engine) struct Retirement {
    pub input: Stage<ChildRetirement>,
    pub phase: RetirementPhase,
    pub proof_deadline: u64,
}

impl Retirement {
    pub fn target(&self) -> ShardTarget {
        let approval = &self.input.approval;
        ShardTarget {
            shard: approval.child.parse().expect("retained shard principal"),
            allocation: Allocation {
                slot: approval.slot,
                generation: approval.generation,
            },
        }
    }
}

#[cfg(feature = "role-issuer")]
mod execution;
#[cfg(feature = "role-issuer")]
pub use execution::retire;
