//! Owned test controls; external ledger/registry methods retain Candid.

pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4_194_304,
    allocation: 16_777_216,
    nodes: 262_144,
    depth: 64,
};
// A magnitude cannot exceed the complete observation frame's byte ceiling.
pub const MAGNITUDE_BYTES: usize = 4_194_304;
pub const SET_TRANSFER_FEE_OVERRIDE: u64 = 0x5445535400000c00;
pub const LOSE_NEXT_TRANSFER_REPLY: u64 = 0x5445535400000c01;
pub const WITHDRAWAL_COUNT: u64 = 0x5445535400000c02;
pub const SET_CREATION_RETRY_REFUSAL: u64 = 0x5445535400000c03;
pub const TRANSFER_COUNT: u64 = 0x5445535400000c04;
pub const REQUEST_COUNT: u64 = 0x5445535400000c05;
pub const REQUESTED_AMOUNTS: u64 = 0x5445535400000c06;
