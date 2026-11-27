//! Complete retained management evidence for a controller-reviewed maintenance window.
use crate::{Status, upgrade};
use serde::{Deserialize, Serialize};

/// Replicated, read-only controller observation of all retained approvals.
pub const OBSERVE: u64 = 30;
/// Complete Root evidence, including a retained Store maintenance operation.
pub const ROOT: u64 = 57;
/// Complete bounded Store history in addition to child custody.
pub const ROOT_HISTORY_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: ROOT_LIMITS.bytes
        + (crate::store_maintenance::MAX_UPGRADES - 1) * crate::store_maintenance::LIMITS.bytes,
    allocation: ROOT_LIMITS.allocation
        + (crate::store_maintenance::MAX_UPGRADES - 1)
            * crate::store_maintenance::LIMITS.allocation,
    nodes: ROOT_LIMITS.nodes
        + (crate::store_maintenance::MAX_UPGRADES - 1) * crate::store_maintenance::LIMITS.nodes,
    depth: ROOT_LIMITS.depth + 2,
};
/// Combined metadata budget; child-only and Store-only contracts remain unchanged.
pub const ROOT_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: crate::stored::LIMITS.bytes + crate::store_maintenance::LIMITS.bytes,
    allocation: crate::stored::LIMITS.allocation + crate::store_maintenance::LIMITS.allocation,
    nodes: crate::stored::LIMITS.nodes + crate::store_maintenance::LIMITS.nodes,
    depth: crate::stored::LIMITS.depth + 2,
};
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

/// One synchronous snapshot of all Root-owned management metadata; no Wasm bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Root {
    /// Both child slots, their complete journals and artifact references.
    pub stored: crate::stored::Custody,
    /// Current Store operation, including uncertain or completed effects.
    pub store_operation: Option<crate::store_maintenance::Status>,
}

/// Complete custody across repeat Store upgrades; the original Root codec stays unchanged.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootHistory {
    /// Child custody and the latest Store attempt.
    pub current: Root,
    /// Completed Store attempts in admission order, excluding the current attempt.
    pub store_history: Vec<crate::store_maintenance::Status>,
}
