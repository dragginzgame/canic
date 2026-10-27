//! Role identity and readiness, independent of capability-selected status unions.

/// Explicit method identity shared by every role exposing an Overview.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0004;

/// Framed unit request; this operation accepts no application arguments.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 64,
    nodes: 8,
    depth: 4,
};

/// Bounded metadata, capabilities and bootstrap diagnostics with Wire accounting.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 65_536 * 16,
    nodes: 4096,
    depth: 16,
};

#[cfg(test)]
mod tests;
