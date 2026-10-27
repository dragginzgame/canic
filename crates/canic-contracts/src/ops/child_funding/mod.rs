//! Lossless transport conversion; funding authority remains parent-owned.
use crate::dto::{child_funding as wire, observability as source};
impl From<source::ChildFundingUsage> for wire::ChildFundingUsage {
    fn from(value: source::ChildFundingUsage) -> Self {
        Self {
            parent: value.parent.into(),
            child: value.child.into(),
            observed_at_ns: value.observed_at_ns,
            accounted_cycles: value.accounted_cycles,
            last_accounted_at_secs: value.last_accounted_at_secs,
            pending_operations: value.pending_operations,
            reserved_cycles: value.reserved_cycles,
        }
    }
}
impl From<wire::ChildFundingUsage> for source::ChildFundingUsage {
    fn from(value: wire::ChildFundingUsage) -> Self {
        Self {
            parent: value.parent.into(),
            child: value.child.into(),
            observed_at_ns: value.observed_at_ns,
            accounted_cycles: value.accounted_cycles,
            last_accounted_at_secs: value.last_accounted_at_secs,
            pending_operations: value.pending_operations,
            reserved_cycles: value.reserved_cycles,
        }
    }
}
