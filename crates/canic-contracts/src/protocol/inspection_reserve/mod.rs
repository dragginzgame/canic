//! Controller-only indicative reserve for the next protected management inspection.
/// Exact read method; an observation never authorizes a paid effect.
pub const METHOD: &str = "canic_root_inspection_reserve";
/// Native frame identity.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_010d;
/// One native target principal.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 4096,
    nodes: 256,
    depth: 16,
};
/// Exact caller/target and three cycle amounts, including a typed refusal.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 65536,
    nodes: 4096,
    depth: 32,
};
#[cfg(test)]
mod tests;
