//! Complete retained management evidence for a controller-reviewed maintenance window.
use crate::{Status, upgrade};
use serde::{Deserialize, Serialize};

/// Replicated, read-only controller observation of all retained approvals.
pub const OBSERVE: u64 = 30;
/// Bounded full journal response; no artifact bytes are exported.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 8192,
    allocation: (1 + upgrade::MAX_UPGRADES) * crate::LIMITS.allocation,
    nodes: (1 + upgrade::MAX_UPGRADES) * crate::LIMITS.nodes,
    depth: crate::LIMITS.depth + 2,
};

/// Creation evidence and every admitted upgrade, oldest first.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Custody {
    /// Original durable creation outcome. Live page count and executable marker are zero/false.
    pub creation: Status,
    /// Completed historical upgrades followed by the current admitted upgrade.
    pub upgrades: Vec<upgrade::Status>,
}
