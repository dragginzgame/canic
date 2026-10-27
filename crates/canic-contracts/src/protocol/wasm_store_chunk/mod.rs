//! Bounded Root-to-Store chunk reads; storage encoding remains unchanged.

/// Maintained template chunk-read method identity.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0200;

/// Two template labels and one chunk index within the existing ingress envelope.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 131_072,
    nodes: 256,
    depth: 16,
};

/// One management-sized 1 MiB chunk plus the result/error envelope.
/// Wire reserves the frame and charges eight allocation bytes per payload byte.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1_114_112,
    allocation: 1_114_112 * 9,
    nodes: 256,
    depth: 16,
};

#[cfg(test)]
mod tests;
