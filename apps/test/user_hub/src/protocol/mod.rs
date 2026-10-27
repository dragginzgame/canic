//! Native admission probe contract for this installed application fixture.

pub const ADMISSION: u64 = 0x5445_5354_0000_0300;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};
/// Admit a complete operation identity and its accompanying scalar fields.
pub const OPERATION_REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    nodes: 256,
    ..REQUEST_LIMITS
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 262_144,
    nodes: 4096,
    depth: 32,
};

pub const GENERATION: u64 = 0x5445_5354_0000_0501;
pub const SET_ROW: u64 = 0x5445_5354_0000_0502;
pub const ROWS: u64 = 0x5445_5354_0000_0503;
pub const RECOVERY_BALANCE: u64 = 0x5445_5354_0000_0504;

pub const CREATE_ACCOUNT: u64 = 0x5445_5354_0000_0600;
pub const CREATE_FIXTURE_CHILD: u64 = 0x5445_5354_0000_0601;
pub const PLAN_ACCOUNT: u64 = 0x5445_5354_0000_0602;

pub const CALLER_PROBE: u64 = 0x5445_5354_0000_0620;
pub const CALLER_TARGET: u64 = 0x5445_5354_0000_0621;
pub const CALLER_HOLD: u64 = 0x5445_5354_0000_0622;
pub const CALLER_EFFECT_STATUS: u64 = 0x5445_5354_0000_0623;
pub const CALLER_EFFECT: u64 = 0x5445_5354_0000_0624;

pub const RELEASE_FIXTURE: u64 = 0x5445_5354_0000_0900;
