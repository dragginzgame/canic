//! Native contracts for the managed admission and initialization probes.

pub const PUBLIC: u64 = 0x5445_5354_0000_0400;
pub const GUARD: u64 = 0x5445_5354_0000_0401;
pub const OWNED: u64 = 0x5445_5354_0000_0402;
pub const WORKFLOW_RUNS: u64 = 0x5445_5354_0000_0403;
pub const PARITY: u64 = 0x5445_5354_0000_0404;
pub const INITIALIZATION: u64 = 0x5445_5354_0000_0405;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 262_144,
    nodes: 4096,
    depth: 32,
};
/// Observe maximum application bytes with bounded framing and visitor overhead.
pub const INITIALIZATION_RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 131_072,
    allocation: 4_194_304,
    nodes: 131_072,
    depth: 32,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximum_initialization_response_roundtrips() {
        let response: Result<Vec<u8>, canic::Error> = Ok(vec![
            255;
            canic::dto::component_registry::MAX_COMPONENT_APPLICATION_INIT_BYTES
        ]);
        let bytes = ic_wire::native::encode_response(
            INITIALIZATION,
            &response,
            INITIALIZATION_RESPONSE_LIMITS,
        )
        .expect("maximum initialized application bytes fit the probe response");
        let decoded: Result<Vec<u8>, canic::Error> = ic_wire::native::decode_response(
            INITIALIZATION,
            &bytes,
            INITIALIZATION_RESPONSE_LIMITS,
        )
        .expect("decode maximum initialized application bytes");
        assert_eq!(decoded, response);
    }
}
