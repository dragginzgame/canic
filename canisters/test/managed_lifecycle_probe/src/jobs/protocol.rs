//! Private application qualification endpoint identities and bounded codec policy.
pub const ENQUEUE: u64 = 0x5445_5354_0000_0A00;
pub const DRAIN: u64 = 0x5445_5354_0000_0A01;
pub const STATUS: u64 = 0x5445_5354_0000_0A02;
pub const EFFECT: u64 = 0x5445_5354_0000_0A03;
pub const RECONCILE: u64 = 0x5445_5354_0000_0A04;
pub const FAULT: u64 = 0x5445_5354_0000_0A05;
pub const CORRUPT: u64 = 0x5445_5354_0000_0A06;
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 8192,
    allocation: 65_536,
    nodes: 2048,
    depth: 32,
};
