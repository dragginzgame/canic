//! Application-owned counter identities and bounded unit/u64 payloads.

pub(crate) const COUNT_ID: u64 = 0x454d_4244_0000_0001;
pub(crate) const INCREMENT_ID: u64 = 0x454d_4244_0000_0002;

pub(crate) const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 256,
    nodes: 8,
    depth: 4,
};
